//! The link and citation tables behind a page: `page_links` and
//! `page_sources`, kept in step with the page text inside the transaction that
//! writes it, plus the queries that read them.

use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::derive::{derive, is_wiki_page, PageFacts, DERIVE_REV};
use crate::sql::backend;
use crate::store::ProjectId;
use crate::{RelPath, StoreError};

/// A `[[link]]` found in a page and the page it currently resolves to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkTarget {
    /// The link as written, normalised.
    pub target_ref: String,
    /// `None` when no page matches: a broken link.
    pub path: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PageLinks {
    pub outgoing: Vec<LinkTarget>,
    /// Paths of the pages that link here, sorted.
    pub backlinks: Vec<String>,
}

/// Replaces the link and source rows of one page with those in `facts`.
/// Pages outside the link graph (anything but `wiki/**.md`) end up with none.
/// Links are stored unresolved; [`relink`] fills in their targets.
pub(crate) async fn index_page(
    tx: &mut Transaction<'_, Postgres>,
    project: ProjectId,
    id: Uuid,
    path: &str,
    facts: &PageFacts,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM page_links WHERE source_page_id = $1")
        .bind(id)
        .execute(&mut **tx)
        .await?;
    sqlx::query("DELETE FROM page_sources WHERE page_id = $1")
        .bind(id)
        .execute(&mut **tx)
        .await?;
    if !is_wiki_page(path) {
        return Ok(());
    }
    if !facts.links.is_empty() {
        sqlx::query(
            "INSERT INTO page_links (project_id, source_page_id, target_ref)
             SELECT $1, $2, ref FROM unnest($3::text[]) AS ref",
        )
        .bind(project.0)
        .bind(id)
        .bind(&facts.links)
        .execute(&mut **tx)
        .await?;
    }
    if !facts.sources.is_empty() {
        sqlx::query(
            "INSERT INTO page_sources (page_id, project_id, source_ref)
             SELECT $1, $2, ref FROM unnest($3::text[]) AS ref",
        )
        .bind(id)
        .bind(project.0)
        .bind(&facts.sources)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

/// Every form a link to a page at `path` can take: the path, the file name
/// and the file name without `.md`.
fn refs_to(paths: &[String]) -> Vec<String> {
    let mut refs = Vec::new();
    for path in paths {
        let name = path.rsplit('/').next().unwrap_or(path);
        refs.push(path.clone());
        refs.push(name.to_string());
        refs.push(name.trim_end_matches(".md").to_string());
    }
    refs.sort();
    refs.dedup();
    refs
}

const RELINK: &str = r#"
UPDATE page_links l SET target_page_id = (
    SELECT p.id FROM pages p, (SELECT CASE WHEN right(l.target_ref, 3) = '.md'
                                           THEN l.target_ref ELSE l.target_ref || '.md' END AS name) f
    WHERE p.project_id = l.project_id AND p.deleted_at IS NULL
      AND p.path LIKE 'wiki/%' AND right(p.path, 3) = '.md'
      AND CASE WHEN position('/' in l.target_ref) > 0
               THEN p.path = l.target_ref
               ELSE p.path = f.name OR right(p.path, length(f.name) + 1) = '/' || f.name END
    ORDER BY p.path COLLATE "C" LIMIT 1)
WHERE l.project_id = $1
  AND ($4 OR l.source_page_id = ANY($2) OR l.target_page_id = ANY($2) OR l.target_ref = ANY($3))
"#;

/// Re-resolves the links a change to the pages `ids` (found at `paths`, old
/// and new) can affect: their own, those pointing at them, and any that name
/// one of those paths.
pub(crate) async fn relink(
    tx: &mut Transaction<'_, Postgres>,
    project: ProjectId,
    ids: &[Uuid],
    paths: &[String],
) -> Result<(), sqlx::Error> {
    sqlx::query(RELINK)
        .bind(project.0)
        .bind(ids)
        .bind(refs_to(paths))
        .bind(false)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn relink_project(
    tx: &mut Transaction<'_, Postgres>,
    project: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(RELINK)
        .bind(project)
        .bind(Vec::<Uuid>::new())
        .bind(Vec::<String>::new())
        .bind(true)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// Drops the rows of pages that were deleted, then re-resolves whatever
/// pointed at them.
pub(crate) async fn forget_pages(
    tx: &mut Transaction<'_, Postgres>,
    project: ProjectId,
    ids: &[Uuid],
    paths: &[String],
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM page_links WHERE source_page_id = ANY($1)")
        .bind(ids)
        .execute(&mut **tx)
        .await?;
    sqlx::query("DELETE FROM page_sources WHERE page_id = ANY($1)")
        .bind(ids)
        .execute(&mut **tx)
        .await?;
    relink(tx, project, ids, paths).await
}

/// Pages written before the derived columns existed, or under older rules,
/// are re-derived in batches; returns how many.
pub(crate) async fn reindex_stale(pool: &PgPool) -> Result<usize, StoreError> {
    let mut total = 0;
    loop {
        let mut tx = pool.begin().await.map_err(backend)?;
        let rows = sqlx::query(
            "SELECT id, project_id, path, content FROM pages
             WHERE deleted_at IS NULL AND derived_rev < $1
             ORDER BY project_id, id LIMIT 200 FOR UPDATE SKIP LOCKED",
        )
        .bind(DERIVE_REV)
        .fetch_all(&mut *tx)
        .await
        .map_err(backend)?;
        if rows.is_empty() {
            return Ok(total);
        }
        let mut projects = Vec::new();
        for row in &rows {
            let id: Uuid = row.try_get("id").map_err(backend)?;
            let project: Uuid = row.try_get("project_id").map_err(backend)?;
            let path: String = row.try_get("path").map_err(backend)?;
            let content: String = row.try_get("content").map_err(backend)?;
            let facts = derive(&path, &content);
            sqlx::query(
                "UPDATE pages SET frontmatter = $2, title = $3, page_type = $4, derived_rev = $5
                 WHERE id = $1",
            )
            .bind(id)
            .bind(sqlx::types::Json(&facts.frontmatter))
            .bind(&facts.title)
            .bind(&facts.page_type)
            .bind(DERIVE_REV)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
            index_page(&mut tx, ProjectId(project), id, &path, &facts)
                .await
                .map_err(backend)?;
            if !projects.contains(&project) {
                projects.push(project);
            }
        }
        for project in projects {
            relink_project(&mut tx, project).await.map_err(backend)?;
        }
        tx.commit().await.map_err(backend)?;
        total += rows.len();
    }
}

pub(crate) async fn page_links(
    pool: &PgPool,
    project: ProjectId,
    path: &RelPath,
) -> Result<PageLinks, StoreError> {
    let outgoing = sqlx::query(
        "SELECT l.target_ref, t.path AS target_path
         FROM page_links l
         JOIN pages s ON s.id = l.source_page_id
         LEFT JOIN pages t ON t.id = l.target_page_id
         WHERE s.project_id = $1 AND s.path = $2 AND s.deleted_at IS NULL
           AND t.id IS DISTINCT FROM s.id
         ORDER BY l.target_ref COLLATE \"C\"",
    )
    .bind(project.0)
    .bind(path.as_str())
    .fetch_all(pool)
    .await
    .map_err(backend)?
    .iter()
    .map(|row| {
        Ok(LinkTarget {
            target_ref: row.try_get("target_ref").map_err(backend)?,
            path: row.try_get("target_path").map_err(backend)?,
        })
    })
    .collect::<Result<_, StoreError>>()?;
    let backlinks = sqlx::query_scalar(
        "SELECT s.path FROM page_links l
         JOIN pages t ON t.id = l.target_page_id
         JOIN pages s ON s.id = l.source_page_id
         WHERE t.project_id = $1 AND t.path = $2 AND t.deleted_at IS NULL
           AND s.deleted_at IS NULL AND s.id <> t.id
         GROUP BY s.path ORDER BY s.path COLLATE \"C\"",
    )
    .bind(project.0)
    .bind(path.as_str())
    .fetch_all(pool)
    .await
    .map_err(backend)?;
    Ok(PageLinks { outgoing, backlinks })
}

/// Paths of the live pages whose `sources:` cite the Source stored at
/// `source` (`raw/sources/...`). A citation is a path, or a file name, with
/// or without the `raw/sources/` prefix.
pub(crate) async fn pages_citing(
    pool: &PgPool,
    project: ProjectId,
    source: &RelPath,
) -> Result<Vec<String>, StoreError> {
    sqlx::query_scalar(
        "SELECT p.path FROM page_sources s
         JOIN pages p ON p.id = s.page_id
         WHERE s.project_id = $1 AND p.deleted_at IS NULL
           AND (s.source_ref = $2 OR 'raw/sources/' || s.source_ref = $2
                OR right($2, length(s.source_ref) + 1) = '/' || s.source_ref)
         GROUP BY p.path ORDER BY p.path COLLATE \"C\"",
    )
    .bind(project.0)
    .bind(source.as_str())
    .fetch_all(pool)
    .await
    .map_err(backend)
}
