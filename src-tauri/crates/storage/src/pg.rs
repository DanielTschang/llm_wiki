use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Postgres, Row, Transaction};

use crate::derive::{derive, DERIVE_REV};
use crate::index::{self, PageLinks};
use crate::listing::{dir_meta, list_entries, FileInfo};
use crate::sql::{any_live, backend, below, collides, is_unique_violation, WriteFailure};
use crate::store::{Body, EntryKind, Meta, Precondition, ProjectFiles, ProjectId, WriteCtx};
use crate::{RelPath, StoreError};

/// Postgres-backed page storage: the `pages` table for current content and
/// `page_versions` for the append-only write log. Serves text documents only;
/// blobs and state files are routed elsewhere (see [`crate::route`]).
#[derive(Clone)]
pub struct PgFiles {
    pool: PgPool,
    max_versions: u32,
}

/// Versions kept per page, current one included. Older ones are pruned in the
/// same transaction as the write that pushes them out.
pub const DEFAULT_MAX_VERSIONS: u32 = 8;

/// Applies the schema in `migrations/`. Safe to run on every start.
pub async fn migrate(pool: &PgPool) -> Result<(), StoreError> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(backend)
}

fn content_hash(content: &str) -> Vec<u8> {
    Sha256::digest(content.as_bytes()).to_vec()
}

pub(crate) fn rel_from_row(row: &PgRow) -> Result<RelPath, StoreError> {
    RelPath::parse(&row.try_get::<String, _>("path").map_err(backend)?)
}

pub(crate) fn file_meta(path: RelPath, row: &PgRow) -> Result<Meta, StoreError> {
    Ok(Meta {
        path,
        kind: EntryKind::File,
        size: row.try_get::<i64, _>("size").map_err(backend)? as u64,
        modified: row
            .try_get::<DateTime<Utc>, _>("updated_at")
            .map_err(backend)?,
        version: Some(row.try_get::<i32, _>("version").map_err(backend)? as u32),
    })
}

impl PgFiles {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            max_versions: DEFAULT_MAX_VERSIONS,
        }
    }

    /// Keeps `max_versions` log rows per page (at least 1).
    pub fn with_max_versions(mut self, max_versions: u32) -> Self {
        self.max_versions = max_versions.max(1);
        self
    }

    /// Re-derives pages stored before the link tables existed, or under older
    /// derivation rules. Cheap when there is nothing to do; call on start.
    pub async fn reindex_stale(&self) -> Result<usize, StoreError> {
        index::reindex_stale(&self.pool).await
    }

    /// The links out of a page and the pages linking to it.
    pub async fn page_links(
        &self,
        project: ProjectId,
        path: &RelPath,
    ) -> Result<PageLinks, StoreError> {
        index::page_links(&self.pool, project, path).await
    }

    /// The pages whose `sources:` cite the Source stored at `source`.
    pub async fn pages_citing(
        &self,
        project: ProjectId,
        source: &RelPath,
    ) -> Result<Vec<String>, StoreError> {
        index::pages_citing(&self.pool, project, source).await
    }

    async fn log_version(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        page_id: uuid::Uuid,
        version: i32,
        op: &str,
        content: Option<&str>,
        frontmatter: Option<&serde_json::Map<String, serde_json::Value>>,
        ctx: &WriteCtx,
    ) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO page_versions
                 (page_id, version, op, content, content_sha256, frontmatter, tool, author_id, ingest_job_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(page_id)
        .bind(version)
        .bind(op)
        .bind(content)
        .bind(content.map(content_hash))
        .bind(frontmatter.map(sqlx::types::Json))
        .bind(&ctx.tool)
        .bind(ctx.actor)
        .bind(ctx.ingest_job_id)
        .execute(&mut **tx)
        .await
        .map_err(backend)?;
        // Every write adds exactly one row and bumps `version` by one, so the
        // newest `max_versions` rows are those with version > current - max.
        sqlx::query("DELETE FROM page_versions WHERE page_id = $1 AND version <= $2 - $3")
            .bind(page_id)
            .bind(version)
            .bind(self.max_versions as i32)
            .execute(&mut **tx)
            .await
            .map_err(backend)?;
        Ok(())
    }
}

#[async_trait]
impl ProjectFiles for PgFiles {
    async fn read(&self, project: ProjectId, path: &RelPath) -> Result<(Body, Meta), StoreError> {
        let row = sqlx::query(
            "SELECT content, version, octet_length(content)::bigint AS size, updated_at
             FROM pages WHERE project_id = $1 AND path = $2 AND deleted_at IS NULL",
        )
        .bind(project.0)
        .bind(path.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(backend)?
        .ok_or_else(|| StoreError::not_found(path))?;
        let content: String = row.try_get("content").map_err(backend)?;
        Ok((Body::Text(content), file_meta(path.clone(), &row)?))
    }

    async fn write(
        &self,
        project: ProjectId,
        path: &RelPath,
        body: Body,
        precondition: Precondition,
        ctx: &WriteCtx,
    ) -> Result<Meta, StoreError> {
        let Body::Text(content) = body else {
            return Err(StoreError::Backend(format!(
                "{path} is stored as a page and must be text"
            )));
        };
        if path.is_root() {
            return Err(StoreError::InvalidPath {
                path: String::new(),
                reason: "cannot write the root",
            });
        }
        // Two writers creating the same path race to the unique index; the
        // loser re-runs and then sees the winner's row.
        for attempt in 0..3 {
            match self
                .write_tx(project, path, &content, precondition, ctx)
                .await
            {
                Err(WriteFailure::Db(error)) if is_unique_violation(&error) => {
                    if precondition == Precondition::CreateOnly || attempt == 2 {
                        return Err(StoreError::already_exists(path));
                    }
                }
                Err(WriteFailure::Db(error)) => return Err(backend(error)),
                Err(WriteFailure::Store(error)) => return Err(error),
                Ok(meta) => return Ok(meta),
            }
        }
        unreachable!("the loop returns on its last attempt")
    }

    async fn delete(
        &self,
        project: ProjectId,
        path: &RelPath,
        ctx: &WriteCtx,
    ) -> Result<usize, StoreError> {
        let mut tx = self.pool.begin().await.map_err(backend)?;
        let rows = sqlx::query(
            "UPDATE pages SET deleted_at = now(), updated_at = now(), version = version + 1
             WHERE project_id = $1 AND deleted_at IS NULL
               AND (path = $2 OR path LIKE $3 ESCAPE '\\')
             RETURNING id, version, path",
        )
        .bind(project.0)
        .bind(path.as_str())
        .bind(below(path))
        .fetch_all(&mut *tx)
        .await
        .map_err(backend)?;
        if rows.is_empty() {
            return Err(StoreError::not_found(path));
        }
        let mut ids = Vec::new();
        let mut paths = Vec::new();
        for row in &rows {
            let id: uuid::Uuid = row.try_get("id").map_err(backend)?;
            let version = row.try_get("version").map_err(backend)?;
            self.log_version(&mut tx, id, version, "delete", None, None, ctx)
                .await?;
            ids.push(id);
            paths.push(row.try_get::<String, _>("path").map_err(backend)?);
        }
        index::forget_pages(&mut tx, project, &ids, &paths)
            .await
            .map_err(backend)?;
        tx.commit().await.map_err(backend)?;
        Ok(rows.len())
    }

    async fn rename(
        &self,
        project: ProjectId,
        from: &RelPath,
        to: &RelPath,
        ctx: &WriteCtx,
    ) -> Result<(), StoreError> {
        if to.is_within(from) {
            return Err(StoreError::InvalidPath {
                path: to.to_string(),
                reason: "destination is inside the source",
            });
        }
        let mut tx = self.pool.begin().await.map_err(backend)?;
        let moving = sqlx::query(
            "SELECT id, path FROM pages
             WHERE project_id = $1 AND deleted_at IS NULL
               AND (path = $2 OR path LIKE $3 ESCAPE '\\')
             FOR UPDATE",
        )
        .bind(project.0)
        .bind(from.as_str())
        .bind(below(from))
        .fetch_all(&mut *tx)
        .await
        .map_err(backend)?;
        if moving.is_empty() {
            return Err(StoreError::not_found(from));
        }
        let mut occupied: Vec<String> = to.ancestors().iter().map(ToString::to_string).collect();
        occupied.push(to.to_string());
        if any_live(&mut tx, "pages", project, &occupied, Some(&below(to))).await? {
            return Err(StoreError::already_exists(to));
        }
        let ids: Vec<uuid::Uuid> = moving
            .iter()
            .map(|row| row.try_get("id").map_err(backend))
            .collect::<Result<_, _>>()?;
        let mut paths: Vec<String> = moving
            .iter()
            .map(|row| row.try_get("path").map_err(backend))
            .collect::<Result<_, _>>()?;
        let moved = sqlx::query(
            "UPDATE pages
             SET path = $2 || substr(path, $3), version = version + 1, updated_at = now()
             WHERE id = ANY($1)
             RETURNING id, version, content, path",
        )
        .bind(&ids)
        .bind(to.as_str())
        .bind(from.as_str().chars().count() as i32 + 1)
        .fetch_all(&mut *tx)
        .await
        .map_err(backend)?;
        for row in &moved {
            let id = row.try_get("id").map_err(backend)?;
            let version = row.try_get("version").map_err(backend)?;
            let content: String = row.try_get("content").map_err(backend)?;
            let new_path: String = row.try_get("path").map_err(backend)?;
            let facts = derive(&new_path, &content);
            self.log_version(
                &mut tx,
                id,
                version,
                "rename",
                Some(&content),
                Some(&facts.frontmatter),
                ctx,
            )
            .await?;
            // Whether a page belongs to the link graph depends on its path.
            index::index_page(&mut tx, project, id, &new_path, &facts)
                .await
                .map_err(backend)?;
            paths.push(new_path);
        }
        index::relink(&mut tx, project, &ids, &paths)
            .await
            .map_err(backend)?;
        tx.commit().await.map_err(backend)?;
        Ok(())
    }

    async fn stat(&self, project: ProjectId, path: &RelPath) -> Result<Option<Meta>, StoreError> {
        let file = sqlx::query(
            "SELECT version, octet_length(content)::bigint AS size, updated_at
             FROM pages WHERE project_id = $1 AND path = $2 AND deleted_at IS NULL",
        )
        .bind(project.0)
        .bind(path.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(backend)?;
        if let Some(row) = file {
            return Ok(Some(file_meta(path.clone(), &row)?));
        }
        let newest: Option<DateTime<Utc>> = sqlx::query_scalar(
            "SELECT max(updated_at) FROM pages
             WHERE project_id = $1 AND deleted_at IS NULL AND path LIKE $2 ESCAPE '\\'",
        )
        .bind(project.0)
        .bind(below(path))
        .fetch_one(&self.pool)
        .await
        .map_err(backend)?;
        Ok(newest.map(|modified| dir_meta(path.clone(), modified)))
    }

    async fn read_tree(
        &self,
        project: ProjectId,
        dir: &RelPath,
        limit: usize,
    ) -> Result<Vec<(Body, Meta)>, StoreError> {
        let rows = sqlx::query(
            "SELECT path, content, version, octet_length(content)::bigint AS size, updated_at
             FROM pages
             WHERE project_id = $1 AND deleted_at IS NULL AND path LIKE $2 ESCAPE '\\'
             ORDER BY path COLLATE \"C\" LIMIT $3",
        )
        .bind(project.0)
        .bind(below(dir))
        .bind(limit.min(i64::MAX as usize) as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        rows.iter()
            .map(|row| {
                let content: String = row.try_get("content").map_err(backend)?;
                Ok((Body::Text(content), file_meta(rel_from_row(row)?, row)?))
            })
            .collect()
    }

    async fn list(
        &self,
        project: ProjectId,
        dir: &RelPath,
        recursive: bool,
    ) -> Result<Vec<Meta>, StoreError> {
        let rows = sqlx::query(
            "SELECT path, version, octet_length(content)::bigint AS size, updated_at
             FROM pages
             WHERE project_id = $1 AND deleted_at IS NULL AND path LIKE $2 ESCAPE '\\'",
        )
        .bind(project.0)
        .bind(below(dir))
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        let files = rows
            .iter()
            .map(|row| {
                let meta = file_meta(rel_from_row(row)?, row)?;
                Ok((
                    meta.path,
                    FileInfo {
                        size: meta.size,
                        modified: meta.modified,
                        version: meta.version,
                    },
                ))
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        list_entries(dir, recursive, files)
    }
}

impl PgFiles {
    async fn write_tx(
        &self,
        project: ProjectId,
        path: &RelPath,
        content: &str,
        precondition: Precondition,
        ctx: &WriteCtx,
    ) -> Result<Meta, WriteFailure> {
        let mut tx = self.pool.begin().await?;
        // A file cannot sit beneath a file, or take the place of a directory.
        if collides(&mut tx, "pages", project, path).await? {
            return Err(StoreError::already_exists(path).into());
        }
        let current: Option<(uuid::Uuid, i32)> = sqlx::query_as(
            "SELECT id, version FROM pages
             WHERE project_id = $1 AND path = $2 AND deleted_at IS NULL FOR UPDATE",
        )
        .bind(project.0)
        .bind(path.as_str())
        .fetch_optional(&mut *tx)
        .await?;
        match (precondition, current) {
            (Precondition::CreateOnly, Some(_)) => {
                return Err(StoreError::already_exists(path).into())
            }
            (Precondition::Version(expected), Some((_, actual))) if expected != actual as u32 => {
                return Err(StoreError::VersionConflict {
                    path: path.to_string(),
                    current: actual as u32,
                }
                .into())
            }
            (Precondition::Version(_), None) => return Err(StoreError::not_found(path).into()),
            _ => {}
        }
        let hash = content_hash(content);
        let facts = derive(path.as_str(), content);
        let frontmatter = sqlx::types::Json(&facts.frontmatter);
        let (id, version, updated_at, op) = match current {
            Some((id, _)) => {
                let row = sqlx::query(
                    "UPDATE pages SET content = $2, content_sha256 = $3,
                         frontmatter = $4, title = $5, page_type = $6, derived_rev = $7,
                         version = version + 1, updated_at = now()
                     WHERE id = $1 RETURNING version, updated_at",
                )
                .bind(id)
                .bind(content)
                .bind(&hash)
                .bind(&frontmatter)
                .bind(&facts.title)
                .bind(&facts.page_type)
                .bind(DERIVE_REV)
                .fetch_one(&mut *tx)
                .await?;
                (
                    id,
                    row.try_get::<i32, _>("version")?,
                    row.try_get("updated_at")?,
                    "update",
                )
            }
            None => {
                let row = sqlx::query(
                    "INSERT INTO pages
                         (project_id, path, content, content_sha256,
                          frontmatter, title, page_type, derived_rev)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id, version, updated_at",
                )
                .bind(project.0)
                .bind(path.as_str())
                .bind(content)
                .bind(&hash)
                .bind(&frontmatter)
                .bind(&facts.title)
                .bind(&facts.page_type)
                .bind(DERIVE_REV)
                .fetch_one(&mut *tx)
                .await?;
                (
                    row.try_get("id")?,
                    row.try_get::<i32, _>("version")?,
                    row.try_get("updated_at")?,
                    "create",
                )
            }
        };
        self.log_version(
            &mut tx,
            id,
            version,
            op,
            Some(content),
            Some(&facts.frontmatter),
            ctx,
        )
        .await?;
        index::index_page(&mut tx, project, id, path.as_str(), &facts).await?;
        index::relink(&mut tx, project, &[id], &[path.to_string()]).await?;
        tx.commit().await?;
        Ok(Meta {
            path: path.clone(),
            kind: EntryKind::File,
            size: content.len() as u64,
            modified: updated_at,
            version: Some(version as u32),
        })
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use sqlx::postgres::PgPoolOptions;

    use super::*;
    use crate::LinkTarget;
    use crate::conformance::{self, Harness};

    struct PgHarness {
        files: PgFiles,
    }

    #[async_trait]
    impl Harness for PgHarness {
        fn store(&self) -> &dyn ProjectFiles {
            &self.files
        }

        async fn new_project(&self) -> ProjectId {
            let id: uuid::Uuid = sqlx::query_scalar(
                "INSERT INTO projects (name) VALUES ('conformance') RETURNING id",
            )
            .fetch_one(&self.files.pool)
            .await
            .unwrap();
            ProjectId(id)
        }

        fn root(&self) -> &'static str {
            "wiki"
        }

        fn versioned(&self) -> bool {
            true
        }
    }

    /// Needs a database the `llm_wiki` role owns, e.g. `make test-pg`.
    async fn harness() -> Option<PgHarness> {
        let url = std::env::var("LLM_WIKI_TEST_DATABASE_URL").ok()?;
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .connect(&url)
            .await
            .unwrap();
        migrate(&pool).await.unwrap();
        sqlx::query("DELETE FROM projects WHERE name = 'conformance' AND created_at < now() - interval '1 hour'")
            .execute(&pool)
            .await
            .unwrap();
        Some(PgHarness {
            files: PgFiles::new(pool),
        })
    }

    conformance::conformance_tests!(harness().await);

    async fn put(h: &PgHarness, p: ProjectId, path: &str, text: &str) {
        h.files
            .write(
                p,
                &RelPath::parse(path).unwrap(),
                Body::Text(text.into()),
                Precondition::Any,
                &WriteCtx::new("test"),
            )
            .await
            .unwrap();
    }

    fn rel(path: &str) -> RelPath {
        RelPath::parse(path).unwrap()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn writing_a_page_stores_its_frontmatter_title_and_type() {
        let Some(h) = harness().await else {
            eprintln!("skipped: LLM_WIKI_TEST_DATABASE_URL is not set");
            return;
        };
        let p = h.new_project().await;
        put(&h, p, "wiki/a.md", "---\ntype: concept\ntitle: Alpha\ntags: [x, y]\n---\nbody").await;
        let row = sqlx::query(
            "SELECT frontmatter, title, page_type, derived_rev FROM pages
             WHERE project_id = $1 AND path = 'wiki/a.md'",
        )
        .bind(p.0)
        .fetch_one(&h.files.pool)
        .await
        .unwrap();
        let fm: serde_json::Value = row.get("frontmatter");
        assert_eq!(fm["tags"], serde_json::json!(["x", "y"]));
        assert_eq!(row.get::<String, _>("title"), "Alpha");
        assert_eq!(row.get::<String, _>("page_type"), "concept");
        assert_eq!(row.get::<i16, _>("derived_rev"), DERIVE_REV);
        // The version log carries the frontmatter too.
        let logged: serde_json::Value = sqlx::query_scalar(
            "SELECT v.frontmatter FROM page_versions v JOIN pages g ON g.id = v.page_id
             WHERE g.project_id = $1 AND g.path = 'wiki/a.md'",
        )
        .bind(p.0)
        .fetch_one(&h.files.pool)
        .await
        .unwrap();
        assert_eq!(logged["title"], "Alpha");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn links_resolve_when_the_target_appears_and_break_when_it_goes() {
        let Some(h) = harness().await else {
            eprintln!("skipped: LLM_WIKI_TEST_DATABASE_URL is not set");
            return;
        };
        let p = h.new_project().await;
        put(&h, p, "wiki/a.md", "see [[b]] and [[wiki/deep/c.md]] and [[a]]").await;
        let broken = h.files.page_links(p, &rel("wiki/a.md")).await.unwrap();
        // A page linking to itself is not an outgoing link.
        assert_eq!(
            broken.outgoing,
            [("b", None), ("wiki/deep/c.md", None)]
                .map(|(r, path): (&str, Option<&str>)| LinkTarget {
                    target_ref: r.into(),
                    path: path.map(Into::into)
                })
        );

        put(&h, p, "wiki/entities/b.md", "# B").await;
        put(&h, p, "wiki/deep/c.md", "# C [[b]]").await;
        let links = h.files.page_links(p, &rel("wiki/a.md")).await.unwrap();
        assert_eq!(links.outgoing[0].path.as_deref(), Some("wiki/entities/b.md"));
        assert_eq!(links.outgoing[1].path.as_deref(), Some("wiki/deep/c.md"));
        let b = h.files.page_links(p, &rel("wiki/entities/b.md")).await.unwrap();
        assert_eq!(b.backlinks, ["wiki/a.md", "wiki/deep/c.md"]);

        // Rewriting a page replaces its links.
        put(&h, p, "wiki/a.md", "only [[c]]").await;
        let b = h.files.page_links(p, &rel("wiki/entities/b.md")).await.unwrap();
        assert_eq!(b.backlinks, ["wiki/deep/c.md"]);

        // Deleting the target breaks the link instead of dropping it.
        h.files
            .delete(p, &rel("wiki/deep/c.md"), &WriteCtx::new("test"))
            .await
            .unwrap();
        let a = h.files.page_links(p, &rel("wiki/a.md")).await.unwrap();
        assert_eq!(
            a.outgoing,
            [LinkTarget { target_ref: "c".into(), path: None }]
        );
        // The deleted page no longer counts as a source of links.
        let b = h.files.page_links(p, &rel("wiki/entities/b.md")).await.unwrap();
        assert!(b.backlinks.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn renaming_a_page_moves_the_links_with_it() {
        let Some(h) = harness().await else {
            eprintln!("skipped: LLM_WIKI_TEST_DATABASE_URL is not set");
            return;
        };
        let p = h.new_project().await;
        put(&h, p, "wiki/a.md", "[[b]] [[wiki/b.md]]").await;
        put(&h, p, "wiki/b.md", "# B [[a]]").await;
        h.files
            .rename(p, &rel("wiki/b.md"), &rel("wiki/old/b2.md"), &WriteCtx::new("test"))
            .await
            .unwrap();
        let a = h.files.page_links(p, &rel("wiki/a.md")).await.unwrap();
        assert!(a.outgoing.iter().all(|l| l.path.is_none()), "{a:?}");
        let b2 = h.files.page_links(p, &rel("wiki/old/b2.md")).await.unwrap();
        assert_eq!(b2.outgoing[0].path.as_deref(), Some("wiki/a.md"));
        assert_eq!(h.files.page_links(p, &rel("wiki/a.md")).await.unwrap().backlinks, ["wiki/old/b2.md"]);
        // Moving a page out of wiki/ takes it out of the graph.
        h.files
            .rename(p, &rel("wiki/old/b2.md"), &rel("elsewhere.md"), &WriteCtx::new("test"))
            .await
            .unwrap();
        assert!(h.files.page_links(p, &rel("wiki/a.md")).await.unwrap().backlinks.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn pages_citing_a_source_are_found_by_path_or_file_name() {
        let Some(h) = harness().await else {
            eprintln!("skipped: LLM_WIKI_TEST_DATABASE_URL is not set");
            return;
        };
        let p = h.new_project().await;
        put(&h, p, "wiki/a.md", "---\nsources: [paper.pdf]\n---\n").await;
        put(&h, p, "wiki/b.md", "---\nsources:\n  - raw/sources/dir/notes.md\n  - paper.pdf\n---\n").await;
        put(&h, p, "wiki/c.md", "---\ntitle: paper.pdf\n---\nmentions paper.pdf").await;
        let cite = |s: &str| {
            let files = h.files.clone();
            let s = rel(s);
            async move { files.pages_citing(p, &s).await.unwrap() }
        };
        assert_eq!(cite("raw/sources/paper.pdf").await, ["wiki/a.md", "wiki/b.md"]);
        assert_eq!(cite("raw/sources/dir/notes.md").await, ["wiki/b.md"]);
        assert!(cite("raw/sources/other.pdf").await.is_empty());
        h.files
            .delete(p, &rel("wiki/a.md"), &WriteCtx::new("test"))
            .await
            .unwrap();
        assert_eq!(cite("raw/sources/paper.pdf").await, ["wiki/b.md"]);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn pages_outside_the_wiki_have_no_links_and_stale_pages_are_reindexed() {
        let Some(h) = harness().await else {
            eprintln!("skipped: LLM_WIKI_TEST_DATABASE_URL is not set");
            return;
        };
        let p = h.new_project().await;
        put(&h, p, "purpose.md", "[[a]]").await;
        put(&h, p, "wiki/a.md", "---\ntitle: A\n---\n[[b]]").await;
        put(&h, p, "wiki/b.md", "# B").await;
        let count = |table: &'static str| {
            let pool = h.files.pool.clone();
            async move {
                sqlx::query_scalar::<_, i64>(&format!(
                    "SELECT count(*) FROM {table} WHERE project_id = $1"
                ))
                .bind(p.0)
                .fetch_one(&pool)
                .await
                .unwrap()
            }
        };
        assert_eq!(count("page_links").await, 1);

        // A page that predates the derived columns.
        sqlx::query("DELETE FROM page_links WHERE project_id = $1").bind(p.0).execute(&h.files.pool).await.unwrap();
        sqlx::query("UPDATE pages SET derived_rev = 0, frontmatter = '{}', title = NULL WHERE project_id = $1")
            .bind(p.0)
            .execute(&h.files.pool)
            .await
            .unwrap();
        assert!(h.files.reindex_stale().await.unwrap() >= 3);
        assert_eq!(count("page_links").await, 1);
        let a = h.files.page_links(p, &rel("wiki/a.md")).await.unwrap();
        assert_eq!(a.outgoing[0].path.as_deref(), Some("wiki/b.md"));
        let title: String = sqlx::query_scalar("SELECT title FROM pages WHERE project_id = $1 AND path = 'wiki/a.md'")
            .bind(p.0)
            .fetch_one(&h.files.pool)
            .await
            .unwrap();
        assert_eq!(title, "A");
        assert_eq!(h.files.reindex_stale().await.unwrap(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn only_the_newest_eight_versions_of_a_page_are_kept() {
        let Some(h) = harness().await else {
            eprintln!("skipped: LLM_WIKI_TEST_DATABASE_URL is not set");
            return;
        };
        let p = h.new_project().await;
        let path = RelPath::parse("wiki/a.md").unwrap();
        let ctx = WriteCtx::new("test");
        for n in 1..=12 {
            h.files
                .write(
                    p,
                    &path,
                    Body::Text(format!("v{n}")),
                    Precondition::Any,
                    &ctx,
                )
                .await
                .unwrap();
        }
        let kept: Vec<i32> = sqlx::query_scalar(
            "SELECT v.version FROM page_versions v JOIN pages g ON g.id = v.page_id
             WHERE g.project_id = $1 AND g.path = 'wiki/a.md' ORDER BY v.version",
        )
        .bind(p.0)
        .fetch_all(&h.files.pool)
        .await
        .unwrap();
        assert_eq!(kept, (5..=12).collect::<Vec<_>>());
        // The page itself is untouched by pruning.
        assert_eq!(
            h.files.read(p, &path).await.unwrap().0,
            Body::Text("v12".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_retention_limit_is_configurable_and_never_below_one() {
        let Some(h) = harness().await else {
            eprintln!("skipped: LLM_WIKI_TEST_DATABASE_URL is not set");
            return;
        };
        let files = h.files.clone().with_max_versions(0);
        let p = h.new_project().await;
        let path = RelPath::parse("wiki/a.md").unwrap();
        for n in 1..=3 {
            files
                .write(
                    p,
                    &path,
                    Body::Text(format!("v{n}")),
                    Precondition::Any,
                    &WriteCtx::new("test"),
                )
                .await
                .unwrap();
        }
        let kept: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM page_versions v JOIN pages g ON g.id = v.page_id
             WHERE g.project_id = $1",
        )
        .bind(p.0)
        .fetch_one(&files.pool)
        .await
        .unwrap();
        assert_eq!(kept, 1);
    }
}
