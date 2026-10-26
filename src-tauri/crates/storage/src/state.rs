use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};

use crate::listing::{dir_meta, list_entries, FileInfo};
use crate::pg::{file_meta, rel_from_row};
use crate::sql::{backend, below, is_unique_violation, WriteFailure};
use crate::store::{Body, EntryKind, Meta, Precondition, ProjectFiles, ProjectId, WriteCtx};
use crate::{RelPath, StoreError};

/// The largest state file accepted. Chat transcripts are the big ones.
pub const MAX_STATE_BYTES: usize = 32 * 1024 * 1024;

/// Folders under `.llm-wiki/` that hold something the platform keeps
/// elsewhere: page backups are `page_versions`, the vector index is pgvector,
/// skills get their own table. Writes there are refused, reads find nothing.
const REFUSED_DIRS: &[&str] = &[
    "page-history",
    "history",
    "lancedb",
    "embedding-revisions",
    "skills",
];

/// Files the platform no longer keeps: the folder watcher is gone, the review
/// inbox is written straight to `review_items`, and the old chat history
/// format is not migrated.
const REFUSED_FILES: &[&str] = &[
    "review-inbox.json",
    "file-snapshot.json",
    "file-change-queue.json",
    "scheduled-import-db.json",
    "chat-history.json",
];

/// The `.llm-wiki/*` state files of every Project, as rows of `state_files`.
/// Serves exactly the paths [`route`](crate::route) sends to the state store.
#[derive(Clone)]
pub struct PgState {
    pool: PgPool,
}

/// Why a path may not be written, or `Ok` if it may.
fn check_writable(path: &RelPath) -> Result<(), StoreError> {
    let refuse = |reason| StoreError::InvalidPath {
        path: path.to_string(),
        reason,
    };
    let Some(rest) = path.as_str().strip_prefix(".llm-wiki/") else {
        return Err(refuse("not a state file"));
    };
    let first = rest.split('/').next().unwrap_or_default();
    let is_dir_entry = rest.contains('/');
    if (is_dir_entry && REFUSED_DIRS.contains(&first))
        || (!is_dir_entry && (REFUSED_DIRS.contains(&first) || REFUSED_FILES.contains(&first)))
    {
        return Err(refuse("is not kept as a state file on the platform"));
    }
    Ok(())
}

impl PgState {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn write_tx(
        &self,
        project: ProjectId,
        path: &RelPath,
        content: &str,
        precondition: Precondition,
    ) -> Result<Meta, WriteFailure> {
        let mut tx = self.pool.begin().await?;
        // A file cannot sit beneath a file, or take the place of a directory.
        let ancestors: Vec<String> = path.ancestors().iter().map(ToString::to_string).collect();
        let blocked = sqlx::query(
            "SELECT 1 FROM state_files
             WHERE project_id = $1 AND (path = ANY($2) OR path LIKE $3 ESCAPE '\\') LIMIT 1",
        )
        .bind(project.0)
        .bind(&ancestors)
        .bind(below(path))
        .fetch_optional(&mut *tx)
        .await?;
        if blocked.is_some() {
            return Err(StoreError::already_exists(path).into());
        }
        let current: Option<i32> = sqlx::query_scalar(
            "SELECT version FROM state_files WHERE project_id = $1 AND path = $2 FOR UPDATE",
        )
        .bind(project.0)
        .bind(path.as_str())
        .fetch_optional(&mut *tx)
        .await?;
        match (precondition, current) {
            (Precondition::CreateOnly, Some(_)) => {
                return Err(StoreError::already_exists(path).into())
            }
            (Precondition::Version(expected), Some(actual)) if expected != actual as u32 => {
                return Err(StoreError::VersionConflict {
                    path: path.to_string(),
                    current: actual as u32,
                }
                .into())
            }
            (Precondition::Version(_), None) => return Err(StoreError::not_found(path).into()),
            _ => {}
        }
        let row = if current.is_some() {
            sqlx::query(
                "UPDATE state_files SET content = $3, version = version + 1, updated_at = now()
                 WHERE project_id = $1 AND path = $2 RETURNING version, updated_at",
            )
        } else {
            sqlx::query(
                "INSERT INTO state_files (project_id, path, content) VALUES ($1, $2, $3)
                 RETURNING version, updated_at",
            )
        }
        .bind(project.0)
        .bind(path.as_str())
        .bind(content)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Meta {
            path: path.clone(),
            kind: EntryKind::File,
            size: content.len() as u64,
            modified: row.try_get::<DateTime<Utc>, _>("updated_at")?,
            version: Some(row.try_get::<i32, _>("version")? as u32),
        })
    }
}

#[async_trait]
impl ProjectFiles for PgState {
    async fn read(&self, project: ProjectId, path: &RelPath) -> Result<(Body, Meta), StoreError> {
        let row = sqlx::query(
            "SELECT content, version, octet_length(content)::bigint AS size, updated_at
             FROM state_files WHERE project_id = $1 AND path = $2",
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
        _ctx: &WriteCtx,
    ) -> Result<Meta, StoreError> {
        check_writable(path)?;
        let Body::Text(content) = body else {
            return Err(StoreError::Backend(format!(
                "{path} is a state file and must be text"
            )));
        };
        if content.len() > MAX_STATE_BYTES {
            return Err(StoreError::Backend(format!("{path} is too large")));
        }
        // Two writers creating the same file race to the primary key; the
        // loser re-runs and then sees the winner's row.
        for attempt in 0..3 {
            match self.write_tx(project, path, &content, precondition).await {
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
        _ctx: &WriteCtx,
    ) -> Result<usize, StoreError> {
        let removed = sqlx::query(
            "DELETE FROM state_files
             WHERE project_id = $1 AND (path = $2 OR path LIKE $3 ESCAPE '\\')",
        )
        .bind(project.0)
        .bind(path.as_str())
        .bind(below(path))
        .execute(&self.pool)
        .await
        .map_err(backend)?
        .rows_affected() as usize;
        if removed == 0 {
            return Err(StoreError::not_found(path));
        }
        Ok(removed)
    }

    async fn rename(
        &self,
        project: ProjectId,
        from: &RelPath,
        to: &RelPath,
        _ctx: &WriteCtx,
    ) -> Result<(), StoreError> {
        check_writable(to)?;
        if to.is_within(from) {
            return Err(StoreError::InvalidPath {
                path: to.to_string(),
                reason: "destination is inside the source",
            });
        }
        let mut tx = self.pool.begin().await.map_err(backend)?;
        let moving: Vec<String> = sqlx::query_scalar(
            "SELECT path FROM state_files
             WHERE project_id = $1 AND (path = $2 OR path LIKE $3 ESCAPE '\\') FOR UPDATE",
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
        let taken = sqlx::query(
            "SELECT 1 FROM state_files
             WHERE project_id = $1 AND (path = ANY($2) OR path LIKE $3 ESCAPE '\\') LIMIT 1",
        )
        .bind(project.0)
        .bind(&occupied)
        .bind(below(to))
        .fetch_optional(&mut *tx)
        .await
        .map_err(backend)?;
        if taken.is_some() {
            return Err(StoreError::already_exists(to));
        }
        sqlx::query(
            "UPDATE state_files
             SET path = $3 || substr(path, $4), version = version + 1, updated_at = now()
             WHERE project_id = $1 AND path = ANY($2)",
        )
        .bind(project.0)
        .bind(&moving)
        .bind(to.as_str())
        .bind(from.as_str().chars().count() as i32 + 1)
        .execute(&mut *tx)
        .await
        .map_err(backend)?;
        tx.commit().await.map_err(backend)?;
        Ok(())
    }

    async fn stat(&self, project: ProjectId, path: &RelPath) -> Result<Option<Meta>, StoreError> {
        let file = sqlx::query(
            "SELECT version, octet_length(content)::bigint AS size, updated_at
             FROM state_files WHERE project_id = $1 AND path = $2",
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
            "SELECT max(updated_at) FROM state_files
             WHERE project_id = $1 AND path LIKE $2 ESCAPE '\\'",
        )
        .bind(project.0)
        .bind(below(path))
        .fetch_one(&self.pool)
        .await
        .map_err(backend)?;
        Ok(newest.map(|modified| dir_meta(path.clone(), modified)))
    }

    async fn list(
        &self,
        project: ProjectId,
        dir: &RelPath,
        recursive: bool,
    ) -> Result<Vec<Meta>, StoreError> {
        let rows = sqlx::query(
            "SELECT path, version, octet_length(content)::bigint AS size, updated_at
             FROM state_files WHERE project_id = $1 AND path LIKE $2 ESCAPE '\\'",
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

    async fn read_tree(
        &self,
        project: ProjectId,
        dir: &RelPath,
        limit: usize,
    ) -> Result<Vec<(Body, Meta)>, StoreError> {
        let rows = sqlx::query(
            "SELECT path, content, version, octet_length(content)::bigint AS size, updated_at
             FROM state_files WHERE project_id = $1 AND path LIKE $2 ESCAPE '\\'
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
}

#[cfg(test)]
mod tests {
    use sqlx::postgres::PgPoolOptions;

    use super::*;
    use crate::conformance::{self, Harness};

    struct StateHarness {
        state: PgState,
    }

    #[async_trait]
    impl Harness for StateHarness {
        fn store(&self) -> &dyn ProjectFiles {
            &self.state
        }

        async fn new_project(&self) -> ProjectId {
            ProjectId(new_project(&self.state.pool).await)
        }

        fn root(&self) -> &'static str {
            ".llm-wiki/data"
        }

        fn versioned(&self) -> bool {
            true
        }
    }

    async fn new_project(pool: &PgPool) -> uuid::Uuid {
        sqlx::query_scalar("INSERT INTO projects (name) VALUES ('conformance') RETURNING id")
            .fetch_one(pool)
            .await
            .unwrap()
    }

    /// Needs a database the `llm_wiki` role owns, e.g. `make test-pg`.
    async fn harness() -> Option<StateHarness> {
        let url = std::env::var("LLM_WIKI_TEST_DATABASE_URL").ok()?;
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .connect(&url)
            .await
            .unwrap();
        crate::migrate(&pool).await.unwrap();
        Some(StateHarness {
            state: PgState::new(pool),
        })
    }

    conformance::conformance_tests!(harness().await);

    #[tokio::test(flavor = "multi_thread")]
    async fn refused_paths_and_binary_bodies_are_not_stored() {
        let Some(h) = harness().await else { return };
        let p = ProjectId(new_project(&h.state.pool).await);
        let ctx = WriteCtx::new("test");
        let write = |raw: &str, body: Body| {
            let path = path(raw);
            let state = h.state.clone();
            let ctx = ctx.clone();
            async move { state.write(p, &path, body, Precondition::Any, &ctx).await }
        };
        let text = |s: &str| Body::Text(s.to_string());

        assert!(matches!(
            write(".llm-wiki/page-history/a-1.md", text("x")).await,
            Err(StoreError::InvalidPath { .. })
        ));
        assert!(matches!(
            write(".llm-wiki/review.json", Body::Bytes(vec![1, 2])).await,
            Err(StoreError::Backend(_))
        ));
        let huge = "x".repeat(MAX_STATE_BYTES + 1);
        assert!(write(".llm-wiki/chats/big.json", Body::Text(huge))
            .await
            .is_err());
        // Nothing was stored by any of those, and what is allowed is.
        assert!(h
            .state
            .list(p, &path(".llm-wiki"), true)
            .await
            .unwrap()
            .is_empty());
        write(".llm-wiki/review.json", text("[]")).await.unwrap();
        // A move may not carry a file into a folder the platform keeps elsewhere.
        let moved = h
            .state
            .rename(
                p,
                &path(".llm-wiki/review.json"),
                &path(".llm-wiki/history/r.json"),
                &ctx,
            )
            .await;
        assert!(matches!(moved, Err(StoreError::InvalidPath { .. })));
        assert_eq!(
            h.state
                .read(p, &path(".llm-wiki/review.json"))
                .await
                .unwrap()
                .0,
            text("[]")
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn state_files_of_two_projects_never_mix_and_go_with_their_project() {
        let Some(h) = harness().await else { return };
        let a = ProjectId(new_project(&h.state.pool).await);
        let b = ProjectId(new_project(&h.state.pool).await);
        let ctx = WriteCtx::new("test");
        let file = path(".llm-wiki/conversations.json");
        for (project, content) in [(a, "[\"a\"]"), (b, "[\"b\"]")] {
            h.state
                .write(
                    project,
                    &file,
                    Body::Text(content.into()),
                    Precondition::Any,
                    &ctx,
                )
                .await
                .unwrap();
        }
        assert_eq!(
            h.state.read(a, &file).await.unwrap().0,
            Body::Text("[\"a\"]".into())
        );
        assert_eq!(
            h.state.read(b, &file).await.unwrap().0,
            Body::Text("[\"b\"]".into())
        );
        // Deleting a Project removes its state files with it.
        sqlx::query("DELETE FROM projects WHERE id = $1")
            .bind(a.0)
            .execute(&h.state.pool)
            .await
            .unwrap();
        assert!(matches!(
            h.state.read(a, &file).await,
            Err(StoreError::NotFound(_))
        ));
        assert!(h.state.read(b, &file).await.is_ok());
    }

    fn path(raw: &str) -> RelPath {
        RelPath::parse(raw).unwrap()
    }

    #[test]
    fn state_files_are_writable_except_where_the_platform_keeps_them_elsewhere() {
        for ok in [
            ".llm-wiki/review.json",
            ".llm-wiki/chats/c1.json",
            ".llm-wiki/ingest-progress/a-b.json",
            ".llm-wiki/ingest-warnings.log",
            ".llm-wiki/agent-sessions/s1.json",
        ] {
            assert!(check_writable(&path(ok)).is_ok(), "{ok}");
        }
        for refused in [
            ".llm-wiki/page-history/a-2026.md",
            ".llm-wiki/history/abc.json",
            ".llm-wiki/lancedb/data.lance",
            ".llm-wiki/embedding-revisions/x.json",
            ".llm-wiki/skills/s/SKILL.md",
            ".llm-wiki/skills",
            ".llm-wiki/review-inbox.json",
            ".llm-wiki/chat-history.json",
            ".llm-wiki",
            "wiki/a.md",
        ] {
            assert!(check_writable(&path(refused)).is_err(), "{refused}");
        }
        // Only the exact file name is refused, not a lookalike.
        assert!(check_writable(&path(".llm-wiki/chats/chat-history.json")).is_ok());
    }
}
