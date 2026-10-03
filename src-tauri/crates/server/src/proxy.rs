//! `/proxy?url=…`: the web build's replacement for Tauri's plugin-http.
//! Requests to user-configured endpoints (LLM, embedding, search) leave
//! from the server, so providers that reject browser CORS still work and
//! the user's proxy settings apply.

use axum::body::{to_bytes, Body};
use axum::extract::Query;
use axum::http::{HeaderMap, HeaderName, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::auth::CLIENT_HEADER;

pub const MAX_PROXY_BODY_BYTES: usize = 64 * 1024 * 1024;

#[derive(Deserialize)]
pub struct ProxyQuery {
    url: String,
}

/// Headers that describe this hop (or this server's session) rather
/// than the upstream request/response.
fn is_local_header(name: &HeaderName) -> bool {
    matches!(
        name.as_str(),
        "host"
            | "connection"
            | "keep-alive"
            | "proxy-connection"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
            | "content-length"
            | "cookie"
            | "set-cookie"
            | "origin"
            | "referer"
    ) || name.as_str() == CLIENT_HEADER
}

pub async fn forward(
    Query(query): Query<ProxyQuery>,
    method: Method,
    headers: HeaderMap,
    body: Body,
) -> Response {
    let url = match reqwest::Url::parse(&query.url) {
        Ok(url) if matches!(url.scheme(), "http" | "https") => url,
        _ => {
            return error(
                StatusCode::BAD_REQUEST,
                format!("Unsupported proxy URL: {}", query.url),
            )
        }
    };
    let body = match to_bytes(body, MAX_PROXY_BODY_BYTES).await {
        Ok(body) => body,
        Err(e) => {
            return error(
                StatusCode::PAYLOAD_TOO_LARGE,
                format!("Request body rejected: {e}"),
            )
        }
    };
    // Built per request: proxy settings can change at runtime (set_proxy_env).
    let client =
        match llm_wiki_core::proxy::configure_http_client(reqwest::Client::builder()).build() {
            Ok(client) => client,
            Err(e) => {
                return error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("HTTP client error: {e}"),
                )
            }
        };
    let mut request = client.request(method, url);
    for (name, value) in headers.iter().filter(|(name, _)| !is_local_header(name)) {
        request = request.header(name, value);
    }
    if !body.is_empty() {
        request = request.body(body);
    }
    let upstream = match request.send().await {
        Ok(upstream) => upstream,
        Err(e) => {
            return error(
                StatusCode::BAD_GATEWAY,
                format!("Upstream request failed: {e}"),
            )
        }
    };

    let mut response = Response::builder().status(upstream.status());
    for (name, value) in upstream
        .headers()
        .iter()
        .filter(|(name, _)| !is_local_header(name))
    {
        response = response.header(name, value);
    }
    // Streamed through unchanged (including any Content-Encoding), so LLM
    // token streams reach the browser as they arrive.
    response
        .body(Body::from_stream(upstream.bytes_stream()))
        .unwrap_or_else(|e| error(StatusCode::BAD_GATEWAY, e.to_string()))
}

fn error(status: StatusCode, message: String) -> Response {
    (status, Json(json!({ "error": message }))).into_response()
}
