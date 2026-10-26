//! `GET /files?path=…`: the web build's `convertFileSrc`.

use axum::body::Body;
use axum::extract::{Extension, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use futures::StreamExt;
use llm_wiki_core::platform::Platform;
use llm_wiki_storage::{route, Backend, EntryKind, Role, StoreError};
use serde::Deserialize;
use serde_json::json;

use crate::principal::Principal;
use crate::{authz, AppState};

/// Extensions a browser may execute script from. Served under a CSP
/// sandbox (opaque origin) so an agent-generated or imported HTML file
/// cannot call the API with the owner's session.
const ACTIVE_CONTENT_EXTENSIONS: &[&str] = &["html", "htm", "xhtml", "svg", "xml", "xsl"];

#[derive(Deserialize)]
pub struct FileQuery {
    path: String,
    /// Ask the browser to save the file (project export) instead of
    /// displaying it.
    #[serde(default)]
    download: bool,
}

pub async fn serve(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Query(query): Query<FileQuery>,
) -> Response {
    let Some(platform) = llm_wiki_core::platform::get() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "the project file store is not installed" })),
        )
            .into_response();
    };
    // The caller needs a role in the file's Project before anything is read.
    let Some(Extension(principal)) = principal else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Not signed in" })),
        )
            .into_response();
    };
    if let Ok((project, _)) = platform.locator.resolve(&query.path) {
        if let Err(denied) =
            authz::require_role(&state.platform, &principal, project, Role::Viewer).await
        {
            return (denied.status, Json(json!({ "error": denied.message }))).into_response();
        }
    }
    serve_from_store(platform, &query).await
}

/// Headers every served file gets: no content sniffing, a sandbox for
/// content a browser could run, and a download name when one is asked for.
fn harden(headers: &mut HeaderMap, active: bool, download_as: Option<&str>) {
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    if active {
        headers.insert(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static("sandbox"),
        );
    }
    if let Some(name) = download_as {
        if let Ok(value) = HeaderValue::from_str(&content_disposition(name)) {
            headers.insert(header::CONTENT_DISPOSITION, value);
        }
    }
}

/// The content type for the common file kinds a Project holds.
fn content_type(extension: &str) -> &'static str {
    match extension {
        "html" | "htm" | "xhtml" => "text/html; charset=utf-8",
        "svg" => "image/svg+xml",
        "xml" | "xsl" => "application/xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "pdf" => "application/pdf",
        "md" | "markdown" | "txt" | "org" | "csv" => "text/plain; charset=utf-8",
        "json" => "application/json",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
}

/// `GET /files` in platform mode: the file comes out of the store, blobs as a
/// stream so a large PDF never sits in memory whole.
async fn serve_from_store(platform: &Platform, query: &FileQuery) -> Response {
    let failure = |status: StatusCode, message: String| {
        (status, Json(json!({ "error": message }))).into_response()
    };
    let (project, rel) = match platform.locator.resolve(&query.path) {
        Ok(target) => target,
        Err(error) => return failure(StatusCode::FORBIDDEN, error.to_string()),
    };
    let from_store = |error: StoreError| match error {
        StoreError::NotFound(_) => failure(StatusCode::NOT_FOUND, error.to_string()),
        other => failure(StatusCode::INTERNAL_SERVER_ERROR, other.to_string()),
    };
    let meta = match platform.files.stat(project, &rel).await {
        Ok(Some(meta)) if meta.kind == EntryKind::File => meta,
        Ok(_) => return failure(StatusCode::NOT_FOUND, format!("not found: {rel}")),
        Err(error) => return from_store(error),
    };
    let body = match (route(&rel), &platform.streams) {
        (Backend::Blobs, Some(streams)) => match streams.get_stream(project, &rel).await {
            Ok((stream, _)) => Body::from_stream(
                stream.map(|part| part.map_err(|error| std::io::Error::other(error.to_string()))),
            ),
            Err(error) => return from_store(error),
        },
        _ => match platform.files.read(project, &rel).await {
            Ok((content, _)) => Body::from(content.into_bytes()),
            Err(error) => return from_store(error),
        },
    };
    let extension = rel.extension().unwrap_or("").to_ascii_lowercase();
    let mut response = Response::new(body);
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(content_type(&extension)),
    );
    headers.insert(header::CONTENT_LENGTH, HeaderValue::from(meta.size));
    harden(
        headers,
        ACTIVE_CONTENT_EXTENSIONS.contains(&extension.as_str()),
        query
            .download
            .then(|| rel.file_name().unwrap_or("download")),
    );
    response
}

/// `attachment` with an ASCII fallback name plus the RFC 5987 UTF-8 form,
/// so non-ASCII project names (e.g. Chinese) survive the download.
fn content_disposition(name: &str) -> String {
    let ascii: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_graphic() && !matches!(c, '"' | '\\') || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let encoded: String = name
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect();
    format!("attachment; filename=\"{ascii}\"; filename*=UTF-8''{encoded}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_disposition_keeps_unicode_names() {
        assert_eq!(
            content_disposition("光學.llmwiki.zip"),
            "attachment; filename=\"__.llmwiki.zip\"; filename*=UTF-8''%E5%85%89%E5%AD%B8.llmwiki.zip"
        );
        assert_eq!(
            content_disposition("a\"b.zip"),
            "attachment; filename=\"a_b.zip\"; filename*=UTF-8''a%22b.zip"
        );
    }
}
