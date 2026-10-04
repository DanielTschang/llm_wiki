//! `GET /files?path=…`: the web build's `convertFileSrc`.

use std::path::Path;

use axum::body::Body;
use axum::extract::{Query, Request, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use tower::ServiceExt;
use tower_http::services::ServeFile;

use crate::AppState;

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
    Query(query): Query<FileQuery>,
    request: Request,
) -> Response {
    let path = match state.roots.check(Path::new(&query.path)) {
        Ok(path) => path,
        Err(message) => {
            return (StatusCode::FORBIDDEN, Json(json!({ "error": message }))).into_response()
        }
    };
    let active = path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ACTIVE_CONTENT_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()));
    let response = match ServeFile::new(&path).oneshot(request).await {
        Ok(response) => response.map(Body::new),
        Err(never) => match never {},
    };
    let mut response = response.into_response();
    let headers = response.headers_mut();
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
    if query.download {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "download".into());
        if let Ok(value) = HeaderValue::from_str(&content_disposition(&name)) {
            headers.insert(header::CONTENT_DISPOSITION, value);
        }
    }
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
