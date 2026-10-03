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
    response
}
