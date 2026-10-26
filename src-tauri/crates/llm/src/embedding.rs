//! Text embeddings from an OpenAI-style, Google, or Volcengine endpoint:
//! requests, batching, retries, and shrinking input a model rejects as too
//! long.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// How long one embedding request may take before it counts as failed.
pub const SEARCH_EMBEDDING_TIMEOUT_SECS: u64 = 8;

/// How to reach an embedding model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchEmbeddingConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub api_key: String,
    pub model: String,
    pub output_dimensionality: Option<f64>,
    /// Extra HTTP headers to send with every embedding request, e.g.
    /// `X-Model-Provider-Id: siliconflow` for the mify gateway.
    /// Reserved names (Authorization, Content-Type, Host,
    /// Content-Length, x-goog-api-key) are skipped — they're managed
    /// by the client.
    #[serde(default)]
    pub extra_headers: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub max_chunk_chars: Option<usize>,
    #[serde(default)]
    pub overlap_chunk_chars: Option<usize>,
    /// Whom the server charges this model's calls to. See [`crate::usage`].
    #[serde(default)]
    pub usage: Option<crate::UsageTag>,
}

pub async fn fetch_embedding_with_retry(
    text: &str,
    cfg: &SearchEmbeddingConfig,
    max_retries: usize,
) -> Result<Vec<f32>, String> {
    let started = std::time::Instant::now();
    let result = fetch_embedding_with_retry_untracked(text, cfg, max_retries).await;
    crate::usage::report(
        cfg.usage.as_ref(),
        crate::CallKind::Embedding,
        result.is_ok(),
        started.elapsed(),
    );
    result
}

async fn fetch_embedding_with_retry_untracked(
    text: &str,
    cfg: &SearchEmbeddingConfig,
    max_retries: usize,
) -> Result<Vec<f32>, String> {
    let mut current = text.to_string();
    let mut attempts = 0usize;
    loop {
        attempts += 1;
        match fetch_embedding_once(&current, cfg).await {
            Ok(embedding) => return Ok(embedding),
            Err(EmbeddingFetchError::Oversize(message)) => {
                if attempts <= max_retries
                    && current.len() > 64
                    && halve_text_on_char_boundary(&mut current)
                {
                    eprintln!(
                        "[Embedding] auto-halving after oversize error at {} chars; retrying at {} chars ({attempts}/{})",
                        text.chars().count(),
                        current.chars().count(),
                        max_retries + 1
                    );
                    continue;
                }
                return Err(format!(
                    "Endpoint rejected input even at {} chars. Lower Settings -> Embedding -> Max Chunk Chars. {message}",
                    current.len()
                ));
            }
            Err(EmbeddingFetchError::Other(message)) => return Err(message),
        }
    }
}

pub async fn fetch_embedding_batch(
    texts: &[String],
    cfg: &SearchEmbeddingConfig,
) -> Result<Vec<Vec<f32>>, String> {
    let started = std::time::Instant::now();
    let result = fetch_embedding_batch_untracked(texts, cfg).await;
    crate::usage::report(
        cfg.usage.as_ref(),
        crate::CallKind::Embedding,
        result.is_ok(),
        started.elapsed(),
    );
    result
}

async fn fetch_embedding_batch_untracked(
    texts: &[String],
    cfg: &SearchEmbeddingConfig,
) -> Result<Vec<Vec<f32>>, String> {
    if texts.is_empty() || texts.len() > 64 {
        return Err("Embedding batch must contain between 1 and 64 inputs".to_string());
    }
    if is_google_embedding_config(cfg) || is_doubao_multimodal_embedding_config(cfg) {
        return Err(
            "This embedding provider does not use the OpenAI-compatible batch format".to_string(),
        );
    }

    let endpoint = volcengine_embedding_endpoint(cfg);
    let mut req = crate::proxy::configure_http_client(reqwest::Client::builder())
        .timeout(std::time::Duration::from_secs(
            SEARCH_EMBEDDING_TIMEOUT_SECS,
        ))
        .build()
        .map_err(|e| format!("Embedding HTTP client error: {e}"))?
        .post(&endpoint)
        .header("Content-Type", "application/json");
    if is_local_or_private_http_endpoint(&endpoint) {
        req = req.header("Origin", "http://localhost");
    }
    if !cfg.api_key.trim().is_empty() {
        req = req.bearer_auth(cfg.api_key.trim());
    }
    if let Some(extra) = cfg.extra_headers.as_ref() {
        for (name, value) in extra {
            let name = name.trim();
            let value = value.trim();
            if !name.is_empty()
                && !value.is_empty()
                && is_safe_extra_header_name(name)
                && !is_reserved_extra_header_name(name)
            {
                req = req.header(name, value);
            }
        }
    }
    let response = req
        .json(&json!({ "model": cfg.model, "input": texts }))
        .send()
        .await
        .map_err(|e| format!("Embedding batch request failed: {e}"))?;
    let status = response.status();
    let response_text = response
        .text()
        .await
        .map_err(|e| format!("Embedding batch response read failed: {e}"))?;
    if !status.is_success() {
        return Err(format!(
            "Embedding batch API HTTP {status}: {}",
            response_text.chars().take(200).collect::<String>()
        ));
    }
    let data: Value = serde_json::from_str(&response_text).map_err(|e| {
        format!(
            "Embedding batch response parse failed: {e}: {}",
            response_text.chars().take(200).collect::<String>()
        )
    })?;
    parse_embedding_batch_values(&data, texts.len())
}


fn parse_embedding_batch_values(data: &Value, expected: usize) -> Result<Vec<Vec<f32>>, String> {
    let entries = data
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| "Embedding batch response missing data array".to_string())?;
    if entries.len() != expected {
        return Err(format!(
            "Embedding batch returned {} vectors for {expected} inputs",
            entries.len()
        ));
    }
    let mut indexed = Vec::with_capacity(entries.len());
    for (position, entry) in entries.iter().enumerate() {
        let index = entry
            .get("index")
            .and_then(Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or(position);
        if index >= expected {
            return Err("Embedding batch response contains an out-of-range index".to_string());
        }
        let values = entry
            .get("embedding")
            .and_then(Value::as_array)
            .ok_or_else(|| "Embedding batch response missing vector".to_string())?;
        let mut vector = Vec::with_capacity(values.len());
        for value in values {
            let number = value
                .as_f64()
                .ok_or_else(|| "Embedding batch response contains non-number values".to_string())?;
            if !number.is_finite() {
                return Err("Embedding batch response contains non-finite values".to_string());
            }
            vector.push(number as f32);
        }
        if vector.is_empty() {
            return Err("Embedding batch response vector is empty".to_string());
        }
        indexed.push((index, vector));
    }
    indexed.sort_by_key(|(index, _)| *index);
    if indexed.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err("Embedding batch response contains duplicate indexes".to_string());
    }
    let dimension = indexed.first().map(|(_, vector)| vector.len()).unwrap_or(0);
    if indexed.iter().any(|(_, vector)| vector.len() != dimension) {
        return Err("Embedding batch response contains inconsistent vector dimensions".to_string());
    }
    Ok(indexed.into_iter().map(|(_, vector)| vector).collect())
}

fn halve_text_on_char_boundary(text: &mut String) -> bool {
    let char_count = text.chars().count();
    if char_count <= 1 {
        return false;
    }
    let keep = (char_count / 2).max(1);
    *text = text.chars().take(keep).collect();
    true
}

enum EmbeddingFetchError {
    Oversize(String),
    Other(String),
}

async fn fetch_embedding_once(
    text: &str,
    cfg: &SearchEmbeddingConfig,
) -> Result<Vec<f32>, EmbeddingFetchError> {
    let is_google = is_google_embedding_config(cfg);
    let is_doubao_multimodal = is_doubao_multimodal_embedding_config(cfg);
    let endpoint = if is_google {
        google_embedding_endpoint(cfg)
    } else {
        volcengine_embedding_endpoint(cfg)
    };
    let mut req = crate::proxy::configure_http_client(reqwest::Client::builder())
        .timeout(std::time::Duration::from_secs(
            SEARCH_EMBEDDING_TIMEOUT_SECS,
        ))
        .build()
        .map_err(|e| EmbeddingFetchError::Other(format!("Embedding HTTP client error: {e}")))?
        .post(&endpoint)
        .header("Content-Type", "application/json");
    // Browser-based local model servers often require a browser-like
    // Origin even when the request is routed through Rust. Keep this
    // reserved so user-supplied extra headers cannot override it.
    if is_local_or_private_http_endpoint(&endpoint) {
        req = req.header("Origin", "http://localhost");
    }
    if !cfg.api_key.trim().is_empty() {
        if is_google {
            req = req.header("x-goog-api-key", cfg.api_key.trim());
        } else {
            req = req.bearer_auth(cfg.api_key.trim());
        }
    }
    if let Some(extra) = cfg.extra_headers.as_ref() {
        for (name, value) in extra {
            let trimmed = name.trim();
            let value = value.trim();
            if trimmed.is_empty() || value.is_empty() || !is_safe_extra_header_name(trimmed) {
                continue;
            }
            if is_reserved_extra_header_name(trimmed) {
                continue;
            }
            req = req.header(trimmed, value);
        }
    }
    let body = if is_google {
        google_embedding_body(&cfg.model, text, cfg.output_dimensionality)
    } else if is_doubao_multimodal {
        doubao_multimodal_embedding_body(&cfg.model, text)
    } else {
        json!({ "model": cfg.model, "input": text })
    };
    let resp = req
        .json(&body)
        .send()
        .await
        .map_err(|e| EmbeddingFetchError::Other(format!("Embedding request failed: {e}")))?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| EmbeddingFetchError::Other(format!("Embedding response read failed: {e}")))?;
    if !status.is_success() {
        let preview = text.chars().take(200).collect::<String>();
        if looks_like_oversize_error(status.as_u16(), &text) {
            return Err(EmbeddingFetchError::Oversize(format!(
                "Embedding API HTTP {status}: {preview}"
            )));
        }
        return Err(EmbeddingFetchError::Other(format!(
            "Embedding API HTTP {status}: {preview}"
        )));
    }
    let data: Value = serde_json::from_str(&text).map_err(|e| {
        EmbeddingFetchError::Other(format!(
            "Embedding response parse failed: {e}: {}",
            text.chars().take(200).collect::<String>()
        ))
    })?;
    parse_embedding_values(&data, is_google, is_doubao_multimodal)
        .map_err(EmbeddingFetchError::Other)
}

fn looks_like_oversize_error(status: u16, body: &str) -> bool {
    if status == 413 {
        return true;
    }
    let lower = body.to_lowercase();
    lower.contains("too long")
        || lower.contains("maximum context")
        || lower.contains("max_tokens")
        || lower.contains("max tokens")
        || lower.contains("context length")
        || lower.contains("token limit")
        || lower.contains("exceeds")
        || lower.contains("input length")
}

fn parse_embedding_values(
    data: &Value,
    is_google: bool,
    is_doubao_multimodal: bool,
) -> Result<Vec<f32>, String> {
    let values = if is_google {
        data.get("embedding")
            .and_then(|v| v.get("values"))
            .and_then(Value::as_array)
    } else if is_doubao_multimodal {
        data.get("data")
            .and_then(|v| v.get("embedding"))
            .and_then(Value::as_array)
    } else {
        data.get("data")
            .and_then(Value::as_array)
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("embedding"))
            .and_then(Value::as_array)
    }
    .ok_or_else(|| "Embedding response missing vector".to_string())?;
    let mut out = Vec::with_capacity(values.len());
    for value in values {
        let n = value
            .as_f64()
            .ok_or_else(|| "Embedding response contains non-number values".to_string())?;
        if !n.is_finite() {
            return Err("Embedding response contains non-finite values".to_string());
        }
        out.push(n as f32);
    }
    if out.is_empty() {
        return Err("Embedding response vector is empty".to_string());
    }
    Ok(out)
}

fn is_safe_extra_header_name(name: &str) -> bool {
    !name.is_empty()
        && name.bytes().all(|b| {
            matches!(
                b,
                b'!' | b'#'
                    | b'$'
                    | b'%'
                    | b'&'
                    | b'\''
                    | b'*'
                    | b'+'
                    | b'-'
                    | b'.'
                    | b'^'
                    | b'_'
                    | b'`'
                    | b'|'
                    | b'~'
                    | b'0'..=b'9'
                    | b'A'..=b'Z'
                    | b'a'..=b'z'
            )
        })
}

fn is_reserved_extra_header_name(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "authorization" | "content-type" | "host" | "content-length" | "origin" | "x-goog-api-key"
    )
}

fn is_google_embedding_config(cfg: &SearchEmbeddingConfig) -> bool {
    let endpoint = cfg.endpoint.to_lowercase();
    endpoint.contains("generativelanguage.googleapis.com") || endpoint.contains(":embedcontent")
}

fn is_local_or_private_http_endpoint(endpoint: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(endpoint) else {
        return false;
    };
    if !matches!(url.scheme(), "http" | "https") {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    let host = host
        .trim_matches('[')
        .trim_matches(']')
        .to_ascii_lowercase();
    if matches!(host.as_str(), "localhost" | "127.0.0.1" | "::1") {
        return true;
    }
    let octets = host
        .split('.')
        .map(str::parse::<u8>)
        .collect::<Result<Vec<_>, _>>();
    let Ok(octets) = octets else {
        return false;
    };
    if octets.len() != 4 {
        return false;
    }
    octets[0] == 10
        || (octets[0] == 172 && (16..=31).contains(&octets[1]))
        || (octets[0] == 192 && octets[1] == 168)
        || octets[0] == 127
}

fn is_volcengine_embedding_endpoint(endpoint: &str) -> bool {
    let host = reqwest::Url::parse(endpoint)
        .ok()
        .and_then(|url| url.host_str().map(|host| host.to_lowercase()))
        .unwrap_or_else(|| {
            let trimmed = endpoint.trim();
            trimmed
                .split_once("://")
                .map(|(_, rest)| rest)
                .unwrap_or(trimmed)
                .split(['/', '?', '#'])
                .next()
                .unwrap_or("")
                .to_lowercase()
        });
    host == "volces.com" || host.ends_with(".volces.com") || host.contains("volcengine")
}

fn is_doubao_multimodal_embedding_config(cfg: &SearchEmbeddingConfig) -> bool {
    cfg.model
        .trim()
        .to_lowercase()
        .contains("doubao-embedding-vision")
}

fn volcengine_embedding_endpoint(cfg: &SearchEmbeddingConfig) -> String {
    let raw = cfg.endpoint.trim();
    if !is_volcengine_embedding_endpoint(raw) {
        return raw.to_string();
    }
    let suffix = if is_doubao_multimodal_embedding_config(cfg) {
        "/embeddings/multimodal"
    } else {
        "/embeddings"
    };
    append_endpoint_path(raw, suffix)
}

fn append_endpoint_path(endpoint: &str, target_suffix: &str) -> String {
    let suffix = target_suffix.trim_start_matches('/');
    match reqwest::Url::parse(endpoint) {
        Ok(mut url) => {
            let path = url.path().trim_end_matches('/').to_string();
            let lower_path = path.to_lowercase();
            let lower_suffix = format!("/{}", suffix.to_lowercase());
            if lower_path.ends_with(&lower_suffix) {
                url.set_path(if path.is_empty() { "/" } else { &path });
                return url.to_string();
            }
            if lower_path.ends_with("/embeddings/multimodal") && lower_suffix == "/embeddings" {
                let base = path.trim_end_matches("/multimodal");
                url.set_path(if base.is_empty() { "/" } else { base });
                return url.to_string();
            }
            if lower_path.ends_with("/embeddings") && lower_suffix == "/embeddings/multimodal" {
                url.set_path(&format!("{path}/multimodal"));
                return url.to_string();
            }
            let next = format!("{path}/{suffix}").replace("//", "/");
            url.set_path(&next);
            url.to_string()
        }
        Err(_) => {
            let (base, query) = endpoint.split_once('?').unwrap_or((endpoint, ""));
            let trimmed = base.trim_end_matches('/');
            let lower = trimmed.to_lowercase();
            let lower_suffix = format!("/{}", suffix.to_lowercase());
            let next = if lower.ends_with(&lower_suffix) {
                trimmed.to_string()
            } else if lower.ends_with("/embeddings/multimodal") && lower_suffix == "/embeddings" {
                trimmed.trim_end_matches("/multimodal").to_string()
            } else if lower.ends_with("/embeddings") && lower_suffix == "/embeddings/multimodal" {
                format!("{trimmed}/multimodal")
            } else {
                format!("{trimmed}/{suffix}")
            };
            if query.is_empty() {
                next
            } else {
                format!("{next}?{query}")
            }
        }
    }
}

fn google_embedding_endpoint(cfg: &SearchEmbeddingConfig) -> String {
    let raw = strip_google_api_key_query(cfg.endpoint.trim())
        .trim_end_matches('/')
        .to_string();
    if raw.to_lowercase().contains(":batchembedcontents") {
        return raw
            .replace(":batchEmbedContents", ":embedContent")
            .replace(":batchembedcontents", ":embedContent");
    }
    if raw.to_lowercase().contains(":embedcontent") {
        return raw;
    }
    let model = cfg.model.trim().trim_start_matches("models/");
    if raw.to_lowercase().contains("/models/") {
        format!("{raw}:embedContent")
    } else {
        format!("{raw}/models/{model}:embedContent")
    }
}

fn strip_google_api_key_query(endpoint: &str) -> String {
    if !endpoint.contains('?') {
        return endpoint.to_string();
    }
    match reqwest::Url::parse(endpoint) {
        Ok(mut url) => {
            let kept = url
                .query_pairs()
                .filter(|(key, _)| !key.eq_ignore_ascii_case("key"))
                .map(|(key, value)| (key.into_owned(), value.into_owned()))
                .collect::<Vec<_>>();
            url.query_pairs_mut().clear().extend_pairs(kept);
            url.to_string().trim_end_matches('?').to_string()
        }
        Err(_) => endpoint
            .split_once('?')
            .map(|(base, query)| {
                let kept = query
                    .split('&')
                    .filter(|pair| {
                        let key = pair.split_once('=').map(|(k, _)| k).unwrap_or(*pair);
                        !key.eq_ignore_ascii_case("key")
                    })
                    .collect::<Vec<_>>();
                if kept.is_empty() {
                    base.to_string()
                } else {
                    format!("{base}?{}", kept.join("&"))
                }
            })
            .unwrap_or_else(|| endpoint.to_string()),
    }
}

fn google_embedding_body(model: &str, text: &str, output_dimensionality: Option<f64>) -> Value {
    let model_path = if model.trim().starts_with("models/") {
        model.trim().to_string()
    } else {
        format!("models/{}", model.trim())
    };
    let mut body = json!({
        "model": model_path,
        "content": { "parts": [{ "text": text }] },
    });
    if let Some(dim) = output_dimensionality
        .filter(|dim| dim.is_finite() && *dim >= 1.0)
        .map(|dim| dim.floor() as u32)
    {
        body["output_dimensionality"] = json!(dim);
    }
    body
}

fn doubao_multimodal_embedding_body(model: &str, text: &str) -> Value {
    json!({
        "model": model,
        "encoding_format": "float",
        "input": [{ "type": "text", "text": text }],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn google_embedding_endpoint_strips_key_and_normalizes_batch_endpoint() {
        let cfg = SearchEmbeddingConfig {
            enabled: true,
            endpoint: "https://generativelanguage.googleapis.com/v1beta/models/gemini-embedding-001:batchEmbedContents?key=URL_KEY&alt=json".to_string(),
            api_key: "HEADER_KEY".to_string(),
            model: "gemini-embedding-001".to_string(),
            output_dimensionality: Some(768.0),
            extra_headers: None,
            max_chunk_chars: None,
            overlap_chunk_chars: None,
            usage: None,
        };

        let endpoint = google_embedding_endpoint(&cfg);
        assert!(endpoint.contains(":embedContent"));
        assert!(!endpoint.contains(":batchEmbedContents"));
        assert!(!endpoint.contains("URL_KEY"));
        assert!(endpoint.contains("alt=json"));

        let body = google_embedding_body("gemini-embedding-001", "hello", Some(768.0));
        assert_eq!(body["model"], "models/gemini-embedding-001");
        assert_eq!(body["output_dimensionality"], 768);
    }

    #[test]
    fn volcengine_embedding_endpoint_only_appends_for_volcengine_hosts() {
        let cfg = SearchEmbeddingConfig {
            enabled: true,
            endpoint: "https://ark.cn-beijing.volces.com/api/v3".to_string(),
            api_key: "ARK_KEY".to_string(),
            model: "doubao-embedding-text-240715".to_string(),
            output_dimensionality: None,
            extra_headers: None,
            max_chunk_chars: None,
            overlap_chunk_chars: None,
            usage: None,
        };
        assert_eq!(
            volcengine_embedding_endpoint(&cfg),
            "https://ark.cn-beijing.volces.com/api/v3/embeddings"
        );

        let custom = SearchEmbeddingConfig {
            endpoint: "https://gateway.example.com/proxy/volcengine?upstream=volces.com"
                .to_string(),
            ..cfg.clone()
        };
        assert_eq!(
            volcengine_embedding_endpoint(&custom),
            "https://gateway.example.com/proxy/volcengine?upstream=volces.com"
        );
    }

    #[test]
    fn doubao_multimodal_endpoint_and_body_use_vision_shape() {
        let cfg = SearchEmbeddingConfig {
            enabled: true,
            endpoint: "https://ark.cn-beijing.volces.com/api/v3/embeddings?trace=1".to_string(),
            api_key: "ARK_KEY".to_string(),
            model: "doubao-embedding-vision".to_string(),
            output_dimensionality: None,
            extra_headers: None,
            max_chunk_chars: None,
            overlap_chunk_chars: None,
            usage: None,
        };

        assert_eq!(
            volcengine_embedding_endpoint(&cfg),
            "https://ark.cn-beijing.volces.com/api/v3/embeddings/multimodal?trace=1"
        );

        let body = doubao_multimodal_embedding_body("doubao-embedding-vision", "hello");
        assert_eq!(body["model"], "doubao-embedding-vision");
        assert_eq!(body["encoding_format"], "float");
        assert_eq!(body["input"][0]["type"], "text");
        assert_eq!(body["input"][0]["text"], "hello");
    }

    #[test]
    fn volcengine_embedding_endpoint_does_not_duplicate_existing_suffixes() {
        let cfg = SearchEmbeddingConfig {
            enabled: true,
            endpoint: "https://ARK.cn-beijing.volces.com/api/v3/embeddings".to_string(),
            api_key: "ARK_KEY".to_string(),
            model: "doubao-embedding-text-240715".to_string(),
            output_dimensionality: None,
            extra_headers: None,
            max_chunk_chars: None,
            overlap_chunk_chars: None,
            usage: None,
        };
        assert_eq!(
            volcengine_embedding_endpoint(&cfg),
            "https://ark.cn-beijing.volces.com/api/v3/embeddings"
        );

        let vision = SearchEmbeddingConfig {
            endpoint: "https://ark.cn-beijing.volces.com/api/v3/embeddings/multimodal".to_string(),
            model: "doubao-embedding-vision".to_string(),
            ..cfg.clone()
        };
        assert_eq!(
            volcengine_embedding_endpoint(&vision),
            "https://ark.cn-beijing.volces.com/api/v3/embeddings/multimodal"
        );

        let text_on_multimodal_endpoint = SearchEmbeddingConfig {
            model: "doubao-embedding-text-240715".to_string(),
            ..vision
        };
        assert_eq!(
            volcengine_embedding_endpoint(&text_on_multimodal_endpoint),
            "https://ark.cn-beijing.volces.com/api/v3/embeddings"
        );
    }

    #[test]
    fn doubao_multimodal_detection_is_model_driven_for_custom_gateways() {
        let cfg = SearchEmbeddingConfig {
            enabled: true,
            endpoint: "https://gateway.example.com/ark/embeddings/multimodal".to_string(),
            api_key: "KEY".to_string(),
            model: "doubao-embedding-vision".to_string(),
            output_dimensionality: None,
            extra_headers: None,
            max_chunk_chars: None,
            overlap_chunk_chars: None,
            usage: None,
        };

        assert!(is_doubao_multimodal_embedding_config(&cfg));
        assert_eq!(
            volcengine_embedding_endpoint(&cfg),
            "https://gateway.example.com/ark/embeddings/multimodal"
        );

        let body = doubao_multimodal_embedding_body(&cfg.model, "hello");
        assert_eq!(body["input"][0]["type"], "text");
        assert_eq!(body["input"][0]["text"], "hello");
    }

    #[test]
    fn doubao_multimodal_response_shape_is_distinct_from_openai_shape() {
        let values =
            parse_embedding_values(&json!({ "data": { "embedding": [0.1, 0.2] } }), false, true)
                .expect("vision response should parse");
        assert_eq!(values, vec![0.1, 0.2]);

        assert!(
            parse_embedding_values(&json!({ "data": [{ "embedding": [0.1] }] }), false, true,)
                .is_err()
        );
    }

    #[test]
    fn extra_embedding_header_names_are_validated_and_reserved_names_are_skipped() {
        assert!(is_safe_extra_header_name("X-Model-Provider-Id"));
        assert!(is_safe_extra_header_name("x_trace.id"));
        assert!(!is_safe_extra_header_name(""));
        assert!(!is_safe_extra_header_name("Bad Header"));
        assert!(!is_safe_extra_header_name("中文"));

        assert!(is_reserved_extra_header_name("Authorization"));
        assert!(is_reserved_extra_header_name("content-type"));
        assert!(is_reserved_extra_header_name("Origin"));
        assert!(is_reserved_extra_header_name("X-Goog-Api-Key"));
        assert!(!is_reserved_extra_header_name("X-Model-Provider-Id"));
    }

    #[test]
    fn embedding_origin_header_is_limited_to_local_or_private_endpoints() {
        assert!(is_local_or_private_http_endpoint(
            "http://127.0.0.1:1234/v1/embeddings"
        ));
        assert!(is_local_or_private_http_endpoint(
            "http://192.168.1.20:11434/v1/embeddings"
        ));
        assert!(is_local_or_private_http_endpoint(
            "http://172.16.0.5/v1/embeddings"
        ));
        assert!(!is_local_or_private_http_endpoint(
            "https://api.openai.com/v1/embeddings"
        ));
    }

    #[test]
    fn embedding_halving_never_splits_cjk_codepoints() {
        let mut text = "默会知识库".to_string();
        assert!(halve_text_on_char_boundary(&mut text));
        assert_eq!(text, "默会");
    }

    #[test]
    fn google_embedding_body_floors_positive_dimensions_and_omits_invalid_values() {
        let body = google_embedding_body("gemini-embedding-001", "hello", Some(1.9));
        assert_eq!(body["output_dimensionality"], 1);

        let zero = google_embedding_body("gemini-embedding-001", "hello", Some(0.0));
        assert!(zero.get("output_dimensionality").is_none());

        let negative = google_embedding_body("gemini-embedding-001", "hello", Some(-4.0));
        assert!(negative.get("output_dimensionality").is_none());
    }

    #[test]
    fn parses_openai_batch_vectors_in_input_order() {
        let response = json!({
            "data": [
                { "index": 1, "embedding": [2.0, 2.5] },
                { "index": 0, "embedding": [1.0, 1.5] }
            ]
        });
        let vectors = parse_embedding_batch_values(&response, 2).unwrap();
        assert_eq!(vectors, vec![vec![1.0, 1.5], vec![2.0, 2.5]]);
    }

    #[test]
    fn rejects_duplicate_openai_batch_indexes() {
        let response = json!({
            "data": [
                { "index": 0, "embedding": [1.0] },
                { "index": 0, "embedding": [2.0] }
            ]
        });
        assert!(parse_embedding_batch_values(&response, 2)
            .unwrap_err()
            .contains("duplicate"));
    }
}
