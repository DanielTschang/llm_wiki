//! `POST /upload?batch=…&name=…`: files from the user's device, for a
//! browser that is not on the server machine.
//!
//! Uploads land in a staging area inside the projects root
//! (`<data-dir>/projects/.uploads/<batch>/<name>`), and the response is the
//! server path. The web build's file picker hands those paths to the same
//! import code the desktop uses, which copies them into `raw/sources/`.
//! Staged files older than a day are removed at startup.

use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime};

use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use futures::StreamExt;
use serde::Deserialize;
use serde_json::json;
use tokio::io::AsyncWriteExt;

use llm_wiki_core::platform::Platform;

use crate::AppState;

pub const UPLOAD_DIR: &str = ".uploads";
const MAX_UPLOAD_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const STALE_AFTER: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Deserialize)]
pub struct UploadQuery {
    /// Groups the files of one pick into one folder.
    batch: String,
    /// File name, optionally with sub-folders (folder uploads).
    name: String,
    /// The Project the file is staged in. Required in platform mode, where
    /// there is no shared staging folder on disk.
    #[serde(default)]
    project: Option<String>,
}

pub async fn receive(
    State(state): State<AppState>,
    Query(query): Query<UploadQuery>,
    body: Body,
) -> Response {
    if let Some(platform) = llm_wiki_core::platform::get() {
        return receive_into_store(platform, &query, body).await;
    }
    let target = match staged_path(&state.upload_root, &query.batch, &query.name) {
        Ok(target) => target,
        Err(message) => return error(StatusCode::BAD_REQUEST, message),
    };
    match write_stream(&target, body).await {
        Ok(()) => Json(json!({ "path": target.to_string_lossy() })).into_response(),
        Err(message) => {
            let _ = tokio::fs::remove_file(&target).await;
            error(StatusCode::BAD_REQUEST, message)
        }
    }
}

/// Platform mode: the upload streams straight into the Project's object
/// store under `.uploads/<batch>/<name>`, and the response is that Project
/// path. The import step then copies it into `raw/sources`.
async fn receive_into_store(platform: &Platform, query: &UploadQuery, body: Body) -> Response {
    use llm_wiki_storage::{ByteStream, Precondition, ProjectId, RelPath, StoreError, WriteCtx};

    let Some(streams) = platform.streams.as_deref() else {
        return error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "uploads are not configured".into(),
        );
    };
    let Some(project) = query.project.as_deref() else {
        return error(StatusCode::BAD_REQUEST, "`project` is required".into());
    };
    let staged = match clean_name(&query.batch, &query.name) {
        Ok(relative) => relative.to_string_lossy().replace('\\', "/"),
        Err(message) => return error(StatusCode::BAD_REQUEST, message),
    };
    let Ok(project_id) = uuid::Uuid::parse_str(project).map(ProjectId) else {
        return error(
            StatusCode::BAD_REQUEST,
            "`project` is not a Project id".into(),
        );
    };
    let rel = match RelPath::parse(&format!("{UPLOAD_DIR}/{staged}")) {
        Ok(rel) => rel,
        Err(message) => return error(StatusCode::BAD_REQUEST, message.to_string()),
    };
    let mut written: u64 = 0;
    let stream: ByteStream = Box::pin(body.into_data_stream().map(move |chunk| {
        let chunk = chunk.map_err(|e| StoreError::Backend(format!("Upload interrupted: {e}")))?;
        written += chunk.len() as u64;
        if written > MAX_UPLOAD_BYTES {
            return Err(StoreError::Backend("Upload exceeds the 4 GB limit".into()));
        }
        Ok(chunk)
    }));
    match streams
        .put_stream(
            project_id,
            &rel,
            stream,
            Precondition::Any,
            &WriteCtx::new("upload"),
        )
        .await
    {
        Ok(_) => {
            Json(json!({ "path": platform.locator.project_path(project_id) + "/" + rel.as_str() }))
                .into_response()
        }
        Err(message) => error(StatusCode::BAD_REQUEST, message.to_string()),
    }
}

/// Validates the client-chosen batch id and relative name and maps them
/// into the staging area. Never lets either escape it.
fn staged_path(upload_root: &Path, batch: &str, name: &str) -> Result<PathBuf, String> {
    clean_name(batch, name).map(|relative| upload_root.join(relative))
}

/// `<batch>/<name>` after checking neither can climb out of the staging area.
fn clean_name(batch: &str, name: &str) -> Result<PathBuf, String> {
    let batch_ok = !batch.is_empty()
        && batch.len() <= 64
        && batch.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    if !batch_ok {
        return Err("Invalid upload batch id".into());
    }
    let relative = Path::new(name);
    let mut clean = PathBuf::new();
    for component in relative.components() {
        match component {
            Component::Normal(part) if !part.to_string_lossy().starts_with('.') => clean.push(part),
            _ => return Err(format!("Invalid upload file name: {name}")),
        }
    }
    if clean.as_os_str().is_empty() {
        return Err("Upload file name is empty".into());
    }
    Ok(PathBuf::from(batch).join(clean))
}

async fn write_stream(target: &Path, body: Body) -> Result<(), String> {
    if let Some(parent) = target.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Cannot create {}: {e}", parent.display()))?;
    }
    let mut file = tokio::fs::File::create(target)
        .await
        .map_err(|e| format!("Cannot create {}: {e}", target.display()))?;
    let mut stream = body.into_data_stream();
    let mut written: u64 = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Upload interrupted: {e}"))?;
        written += chunk.len() as u64;
        if written > MAX_UPLOAD_BYTES {
            return Err("Upload exceeds the 4 GB limit".into());
        }
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Cannot write {}: {e}", target.display()))?;
    }
    file.flush().await.map_err(|e| e.to_string())
}

/// Removes upload batches older than a day. Imports copy what they need,
/// so a staged file is only useful for the few seconds of one pick.
pub fn sweep_stale(upload_root: &Path) {
    let Ok(entries) = std::fs::read_dir(upload_root) else {
        return;
    };
    let now = SystemTime::now();
    for entry in entries.flatten() {
        let stale = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|modified| now.duration_since(modified).ok())
            .is_some_and(|age| age > STALE_AFTER);
        if stale {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

fn error(status: StatusCode, message: String) -> Response {
    (status, Json(json!({ "error": message }))).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_names_into_the_batch_folder() {
        let root = Path::new("/data/projects/.uploads");
        assert_eq!(
            staged_path(root, "b-1", "paper.pdf").unwrap(),
            root.join("b-1/paper.pdf")
        );
        assert_eq!(
            staged_path(root, "b-1", "notes/2026/a.md").unwrap(),
            root.join("b-1/notes/2026/a.md")
        );
    }

    #[test]
    fn rejects_names_and_batches_that_escape() {
        let root = Path::new("/data/projects/.uploads");
        for name in [
            "../x",
            "/etc/passwd",
            "a/../../x",
            ".hidden",
            "a/.git/config",
            "",
        ] {
            assert!(staged_path(root, "b", name).is_err(), "{name}");
        }
        for batch in ["", "../b", "b/c", "b.c"] {
            assert!(staged_path(root, batch, "a.md").is_err(), "{batch}");
        }
    }
}
