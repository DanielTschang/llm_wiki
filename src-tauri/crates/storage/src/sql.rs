//! Helpers shared by the two Postgres-backed stores.

use sqlx::{Postgres, Transaction};

use crate::store::ProjectId;
use crate::{RelPath, StoreError};

pub(crate) fn backend(error: impl std::fmt::Display) -> StoreError {
    StoreError::Backend(error.to_string())
}

/// `LIKE` treats `\`, `%` and `_` specially; in a path they are plain characters.
pub(crate) fn like_escape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if matches!(ch, '\\' | '%' | '_') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// Matches everything strictly beneath `dir` (`dir/%`), or everything for the root.
pub(crate) fn below(dir: &RelPath) -> String {
    if dir.is_root() {
        "%".to_string()
    } else {
        format!("{}/%", like_escape(dir.as_str()))
    }
}

pub(crate) fn is_unique_violation(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .is_some_and(|db| db.code().as_deref() == Some("23505"))
}

pub(crate) enum WriteFailure {
    Db(sqlx::Error),
    Store(StoreError),
}

impl From<sqlx::Error> for WriteFailure {
    fn from(error: sqlx::Error) -> Self {
        Self::Db(error)
    }
}

impl From<StoreError> for WriteFailure {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}

/// True when a live row of `table` (`pages` or `blobs`) sits at one of the
/// `exact` paths or anywhere below `below_pattern`.
pub(crate) async fn any_live(
    tx: &mut Transaction<'_, Postgres>,
    table: &'static str,
    project: ProjectId,
    exact: &[String],
    below_pattern: Option<&str>,
) -> Result<bool, StoreError> {
    let found = sqlx::query(&format!(
        "SELECT 1 FROM {table}
         WHERE project_id = $1 AND deleted_at IS NULL
           AND (path = ANY($2) OR ($3::text IS NOT NULL AND path LIKE $3 ESCAPE '\\'))
         LIMIT 1"
    ))
    .bind(project.0)
    .bind(exact)
    .bind(below_pattern)
    .fetch_optional(&mut **tx)
    .await
    .map_err(backend)?;
    Ok(found.is_some())
}

/// Paths a new file at `path` must not collide with: its ancestors (a file
/// cannot sit beneath a file) and anything beneath it (a file cannot replace
/// a directory).
pub(crate) async fn collides(
    tx: &mut Transaction<'_, Postgres>,
    table: &'static str,
    project: ProjectId,
    path: &RelPath,
) -> Result<bool, StoreError> {
    let ancestors: Vec<String> = path.ancestors().iter().map(ToString::to_string).collect();
    any_live(tx, table, project, &ancestors, Some(&below(path))).await
}
