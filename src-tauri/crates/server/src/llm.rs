//! The LLM gateway.
//!
//! The organisation's models are a catalogue kept by the server: which models
//! exist, where they are reached, and which environment variable holds each
//! one's API key. Browsers and the ingest worker never see a key. They ask the
//! gateway to relay a request they built for a catalogue model; the gateway
//! checks where it is going, puts the key in, streams the reply back, and
//! counts the call.

use std::collections::{BTreeMap, HashSet};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Instant;

use axum::body::{to_bytes, Body, Bytes};
use axum::extract::{Extension, Path, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use futures::stream::BoxStream;
use futures::{Stream, StreamExt};
use llm_wiki_storage::{ProjectId, Role, UsageRecord, LLM_MODELS_KEY};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::authz::{require_role, Denied};
use crate::principal::{PlatformState, Principal};
use crate::proxy::MAX_PROXY_BODY_BYTES;
use crate::AppState;

/// Environment variables holding a model's key must start with this, so a
/// catalogue entry cannot be pointed at the server's own secrets.
pub const KEY_ENV_PREFIX: &str = "LLM_WIKI_KEY_";

const MAX_MODELS: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Purpose {
    #[default]
    Chat,
    Embedding,
}

/// One model the organisation offers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelEntry {
    /// What a request names: lower-case letters, digits, `.`, `_`, `-`.
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub purpose: Purpose,
    /// The wire the app speaks: openai, anthropic, google, azure, ollama,
    /// minimax or custom.
    pub provider: String,
    /// The model (or, on Azure, the deployment) name.
    pub model: String,
    /// Where the provider is. Omitted for the providers that have one place.
    /// For an embedding model, the endpoint itself.
    #[serde(default)]
    pub base_url: Option<String>,
    /// The environment variable holding the API key (`LLM_WIKI_KEY_…`).
    /// Absent for endpoints that need none, such as a local model.
    #[serde(default)]
    pub api_key_env: Option<String>,
    /// For `custom`: `chat_completions` or `anthropic_messages`.
    #[serde(default)]
    pub api_mode: Option<String>,
    #[serde(default)]
    pub max_context_size: Option<u64>,
    #[serde(default)]
    pub azure_api_version: Option<String>,
    #[serde(default)]
    pub azure_model_family: Option<String>,
    #[serde(default)]
    pub streaming: Option<bool>,
    /// Can read images, so it may caption them.
    #[serde(default)]
    pub vision: bool,
    /// Sent with every request to this model. Not secret: members can see them.
    #[serde(default)]
    pub custom_headers: BTreeMap<String, String>,
    #[serde(default)]
    pub output_dimensionality: Option<f64>,
    /// The model used when a caller does not choose, for its purpose.
    #[serde(default)]
    pub default: bool,
}

const PROVIDERS: &[&str] = &[
    "openai",
    "anthropic",
    "google",
    "azure",
    "ollama",
    "minimax",
    "custom",
];

/// Headers the browser carries a key in; the gateway replaces their values.
const AUTH_HEADERS: &[&str] = &["authorization", "x-api-key", "api-key", "x-goog-api-key"];

/// Headers a caller may pass on to the model. Everything else it sends is
/// dropped, so it cannot add routing or credentials of its own.
const FORWARDED_HEADERS: &[&str] = &[
    "content-type",
    "accept",
    "anthropic-version",
    "anthropic-beta",
    "openai-beta",
];

fn default_base(provider: &str) -> Option<&'static str> {
    match provider {
        "openai" => Some("https://api.openai.com"),
        "anthropic" => Some("https://api.anthropic.com"),
        "google" => Some("https://generativelanguage.googleapis.com"),
        _ => None,
    }
}

impl ModelEntry {
    /// Where this model lives: its own base URL or its provider's.
    pub fn base(&self) -> Result<url::Url, String> {
        let raw = match (&self.base_url, default_base(&self.provider)) {
            (Some(url), _) => url.as_str(),
            (None, Some(default)) => default,
            (None, None) => return Err(format!("{} needs a baseUrl", self.id)),
        };
        let url = url::Url::parse(raw).map_err(|e| format!("{}: bad baseUrl: {e}", self.id))?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            return Err(format!("{}: baseUrl must be an http(s) address", self.id));
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(format!("{}: baseUrl must not contain credentials", self.id));
        }
        Ok(url)
    }

    /// The API key from the server's environment, if this model uses one.
    pub fn key(&self) -> Result<Option<String>, String> {
        let Some(name) = &self.api_key_env else {
            return Ok(None);
        };
        match std::env::var(name) {
            Ok(value) if !value.trim().is_empty() => Ok(Some(value.trim().to_string())),
            _ => Err(format!(
                "the server has no API key for \"{}\" (environment variable {name} is not set)",
                self.id
            )),
        }
    }

    /// What every signed-in user may see. `admin` adds how the key is found
    /// and whether the server has it.
    pub fn public_json(&self, admin: bool) -> Value {
        let mut value = serde_json::to_value(self).unwrap_or(Value::Null);
        if let Some(object) = value.as_object_mut() {
            if admin {
                let configured = self.api_key_env.is_none() || self.key().is_ok();
                object.insert("keyConfigured".into(), Value::Bool(configured));
            } else {
                object.remove("apiKeyEnv");
            }
        }
        value
    }
}

fn valid_id(id: &str) -> bool {
    (1..=64).contains(&id.len())
        && id.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit())
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'))
}

fn valid_env_name(name: &str) -> bool {
    name.starts_with(KEY_ENV_PREFIX)
        && name.len() > KEY_ENV_PREFIX.len()
        && name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// Checks a catalogue an administrator submitted.
pub fn validate_catalog(models: &[ModelEntry]) -> Result<(), String> {
    if models.len() > MAX_MODELS {
        return Err(format!("at most {MAX_MODELS} models"));
    }
    let mut ids = HashSet::new();
    let mut defaults: BTreeMap<&'static str, usize> = BTreeMap::new();
    for entry in models {
        if !valid_id(&entry.id) {
            return Err(format!(
                "\"{}\" is not a usable id: use lower-case letters, digits, '.', '_' or '-'",
                entry.id
            ));
        }
        if !ids.insert(entry.id.as_str()) {
            return Err(format!("the id \"{}\" is used twice", entry.id));
        }
        if entry.label.trim().is_empty() || entry.model.trim().is_empty() {
            return Err(format!("{}: a label and a model are required", entry.id));
        }
        if !PROVIDERS.contains(&entry.provider.as_str()) {
            return Err(format!(
                "{}: unknown provider \"{}\" (use one of {})",
                entry.id,
                entry.provider,
                PROVIDERS.join(", ")
            ));
        }
        let base = entry.base()?;
        // The built-in providers are reached where the app expects them; to
        // reach one elsewhere, use `custom`.
        if let (Some(default), Some(_)) = (default_base(&entry.provider), &entry.base_url) {
            let expected = url::Url::parse(default).map_err(|e| e.to_string())?;
            if base.origin() != expected.origin() || !matches!(base.path(), "" | "/") {
                return Err(format!(
                    "{}: {} is always reached at {default}; use the custom provider for another address",
                    entry.id, entry.provider
                ));
            }
        }
        if let Some(name) = &entry.api_key_env {
            if !valid_env_name(name) {
                return Err(format!(
                    "{}: the key variable must be named {KEY_ENV_PREFIX}… in capitals",
                    entry.id
                ));
            }
        }
        if let Some(mode) = &entry.api_mode {
            if !matches!(mode.as_str(), "chat_completions" | "anthropic_messages") {
                return Err(format!("{}: unknown apiMode \"{mode}\"", entry.id));
            }
        }
        for name in entry.custom_headers.keys() {
            let lower = name.to_ascii_lowercase();
            if HeaderName::from_bytes(lower.as_bytes()).is_err()
                || AUTH_HEADERS.contains(&lower.as_str())
                || matches!(
                    lower.as_str(),
                    "host" | "content-length" | "cookie" | "connection"
                )
            {
                return Err(format!(
                    "{}: the header \"{name}\" cannot be set here",
                    entry.id
                ));
            }
        }
        if entry.default {
            let purpose = match entry.purpose {
                Purpose::Chat => "chat",
                Purpose::Embedding => "embedding",
            };
            *defaults.entry(purpose).or_default() += 1;
        }
    }
    if let Some((purpose, _)) = defaults.iter().find(|(_, count)| **count > 1) {
        return Err(format!("only one {purpose} model can be the default"));
    }
    Ok(())
}

/// The catalogue as the administrator last saved it.
pub async fn load_catalog(platform: &PlatformState) -> Result<Vec<ModelEntry>, String> {
    let stored = platform
        .settings
        .get(LLM_MODELS_KEY)
        .await
        .map_err(|e| e.to_string())?;
    match stored {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(value) => {
            serde_json::from_value(value).map_err(|e| format!("the model list is damaged: {e}"))
        }
    }
}

pub fn find<'a>(catalog: &'a [ModelEntry], id: &str) -> Option<&'a ModelEntry> {
    catalog.iter().find(|entry| entry.id == id)
}

/// The model a caller gets when it does not name one.
pub fn default_for(catalog: &[ModelEntry], purpose: Purpose) -> Option<&ModelEntry> {
    let of_purpose = || catalog.iter().filter(|entry| entry.purpose == purpose);
    of_purpose()
        .find(|entry| entry.default)
        .or_else(|| of_purpose().next())
}

/// Whether `target` is somewhere this model may be called: the same origin as
/// its base, under the base's path, with no credentials of its own.
pub fn check_target(entry: &ModelEntry, target: &str) -> Result<url::Url, String> {
    let base = entry.base()?;
    let url = url::Url::parse(target).map_err(|_| "the target is not a URL".to_string())?;
    if url.origin() != base.origin() || !url.username().is_empty() || url.password().is_some() {
        return Err(format!(
            "the target is not an address of the model \"{}\"",
            entry.id
        ));
    }
    let base_path = base.path().trim_end_matches('/');
    let path = url.path();
    let under_base =
        base_path.is_empty() || path == base_path || path.starts_with(&format!("{base_path}/"));
    if !under_base {
        return Err(format!(
            "the target is outside the model \"{}\"'s address",
            entry.id
        ));
    }
    Ok(url)
}

/// The headers to send upstream: the few a caller may pass on, the model's
/// own, and the key in whichever of the key-carrying headers the caller used.
pub fn upstream_headers(incoming: &HeaderMap, entry: &ModelEntry, key: Option<&str>) -> HeaderMap {
    let mut out = HeaderMap::new();
    for name in FORWARDED_HEADERS {
        if let Some(value) = incoming.get(*name) {
            out.insert(HeaderName::from_static(name), value.clone());
        }
    }
    for (name, value) in &entry.custom_headers {
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(name.to_ascii_lowercase().as_bytes()),
            HeaderValue::from_str(value),
        ) {
            out.insert(name, value);
        }
    }
    if let Some(key) = key {
        for name in AUTH_HEADERS {
            let Some(sent) = incoming.get(*name) else {
                continue;
            };
            // Keep the scheme the caller used (`Bearer …`), swap the secret.
            let bearer = sent
                .to_str()
                .map(|v| v.len() > 6 && v[..7.min(v.len())].eq_ignore_ascii_case("bearer "))
                .unwrap_or(false);
            let value = if bearer {
                format!("Bearer {key}")
            } else {
                key.to_string()
            };
            if let Ok(value) = HeaderValue::from_str(&value) {
                out.insert(HeaderName::from_static(name), value);
            }
        }
    }
    out
}

/// Token counts a provider put in its reply, wherever in the stream.
#[derive(Default)]
pub struct UsageScanner {
    head: Vec<u8>,
    tail: Vec<u8>,
}

const SCAN_HEAD: usize = 8 * 1024;
const SCAN_TAIL: usize = 16 * 1024;

impl UsageScanner {
    pub fn push(&mut self, chunk: &[u8]) {
        if self.head.len() < SCAN_HEAD {
            let room = SCAN_HEAD - self.head.len();
            self.head.extend_from_slice(&chunk[..chunk.len().min(room)]);
        }
        self.tail.extend_from_slice(chunk);
        if self.tail.len() > SCAN_TAIL {
            let drop = self.tail.len() - SCAN_TAIL;
            self.tail.drain(..drop);
        }
    }

    /// (input, output) tokens: the last value of each field the provider
    /// reports under any of the common names.
    pub fn tokens(&self) -> (Option<u64>, Option<u64>) {
        let mut text = String::from_utf8_lossy(&self.head).into_owned();
        text.push('\n');
        text.push_str(&String::from_utf8_lossy(&self.tail));
        let last = |names: &[&str]| -> Option<u64> {
            names
                .iter()
                .filter_map(|name| {
                    let needle = format!("\"{name}\"");
                    let mut found = None;
                    let mut from = 0;
                    while let Some(at) = text[from..].find(&needle) {
                        let after = from + at + needle.len();
                        let rest = text[after..].trim_start();
                        if let Some(rest) = rest.strip_prefix(':') {
                            let digits: String = rest
                                .trim_start()
                                .chars()
                                .take_while(char::is_ascii_digit)
                                .collect();
                            if let Ok(n) = digits.parse() {
                                found = Some(n);
                            }
                        }
                        from = after;
                    }
                    found
                })
                .next()
        };
        (
            last(&["input_tokens", "prompt_tokens", "promptTokenCount"]),
            last(&["output_tokens", "completion_tokens", "candidatesTokenCount"]),
        )
    }
}

/// Everything the gateway records about a call except how it ended.
struct CallInfo {
    platform: PlatformState,
    user_id: Option<uuid::Uuid>,
    project_id: Option<uuid::Uuid>,
    model_id: String,
    status: u16,
    started: Instant,
    bytes_in: u64,
}

/// The reply body, passed through unchanged while it is counted. The call is
/// recorded when the body ends or is dropped, so a cancelled reply counts too.
struct Metered {
    inner: BoxStream<'static, Result<Bytes, reqwest::Error>>,
    scanner: UsageScanner,
    bytes_out: u64,
    info: Option<CallInfo>,
}

impl Stream for Metered {
    type Item = Result<Bytes, reqwest::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let poll = self.inner.poll_next_unpin(cx);
        if let Poll::Ready(Some(Ok(chunk))) = &poll {
            self.bytes_out += chunk.len() as u64;
            self.scanner.push(chunk);
        }
        poll
    }
}

impl Drop for Metered {
    fn drop(&mut self) {
        let Some(info) = self.info.take() else { return };
        let (input_tokens, output_tokens) = self.scanner.tokens();
        let record = UsageRecord {
            user_id: info.user_id,
            project_id: info.project_id,
            model_id: info.model_id,
            status: info.status,
            duration_ms: info.started.elapsed().as_millis() as u64,
            bytes_in: info.bytes_in,
            bytes_out: self.bytes_out,
            input_tokens,
            output_tokens,
        };
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            let usage = info.platform.usage.clone();
            runtime.spawn(async move {
                if let Err(error) = usage.record(&record).await {
                    eprintln!("[llm] could not record usage: {error}");
                }
            });
        }
    }
}

fn refuse(status: StatusCode, message: impl Into<String>) -> Response {
    (status, Json(json!({ "error": message.into() }))).into_response()
}

/// `POST /llm/relay/{model}`: relays a request the caller built for a
/// catalogue model. `X-LLM-Target` is the address it would have called
/// directly; `X-LLM-Project` says which Project it is for.
pub async fn relay(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(model_id): Path<String>,
    method: Method,
    headers: HeaderMap,
    body: Body,
) -> Response {
    let (Some(platform), Some(Extension(principal))) = (&state.platform, principal) else {
        return refuse(
            StatusCode::NOT_FOUND,
            "the LLM gateway is only on a platform server",
        );
    };
    if method != Method::POST {
        return refuse(
            StatusCode::METHOD_NOT_ALLOWED,
            "the gateway relays POST requests only",
        );
    }
    let catalog = match load_catalog(platform).await {
        Ok(catalog) => catalog,
        Err(message) => return refuse(StatusCode::INTERNAL_SERVER_ERROR, message),
    };
    let Some(entry) = find(&catalog, &model_id) else {
        return refuse(
            StatusCode::NOT_FOUND,
            format!("no model \"{model_id}\" is offered"),
        );
    };
    let project = match headers.get("x-llm-project").and_then(|v| v.to_str().ok()) {
        None | Some("") => None,
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(id) => Some(ProjectId(id)),
            Err(_) => return refuse(StatusCode::BAD_REQUEST, "X-LLM-Project is not a project id"),
        },
    };
    if let Some(project) = project {
        if let Err(Denied { status, message }) =
            require_role(platform, &principal, project, Role::Viewer).await
        {
            return refuse(status, message);
        }
    }
    let Some(target) = headers.get("x-llm-target").and_then(|v| v.to_str().ok()) else {
        return refuse(StatusCode::BAD_REQUEST, "X-LLM-Target is required");
    };
    let url = match check_target(entry, target) {
        Ok(url) => url,
        Err(message) => return refuse(StatusCode::FORBIDDEN, message),
    };
    let key = match entry.key() {
        Ok(key) => key,
        Err(message) => return refuse(StatusCode::SERVICE_UNAVAILABLE, message),
    };
    let body = match to_bytes(body, MAX_PROXY_BODY_BYTES).await {
        Ok(body) => body,
        Err(e) => {
            return refuse(
                StatusCode::PAYLOAD_TOO_LARGE,
                format!("request body rejected: {e}"),
            )
        }
    };
    let client = match llm_wiki_core::proxy::configure_http_client(
        reqwest::Client::builder().connect_timeout(std::time::Duration::from_secs(30)),
    )
    .build()
    {
        Ok(client) => client,
        Err(e) => {
            return refuse(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("HTTP client error: {e}"),
            )
        }
    };

    let started = Instant::now();
    let bytes_in = body.len() as u64;
    let upstream = client
        .post(url)
        .headers(upstream_headers(&headers, entry, key.as_deref()))
        .body(body)
        .send()
        .await;
    let call = |status: u16| CallInfo {
        platform: platform.clone(),
        user_id: principal.actor().map(|user| user.id),
        project_id: project.map(|p| p.0),
        model_id: entry.id.clone(),
        status,
        started,
        bytes_in,
    };
    let upstream = match upstream {
        Ok(upstream) => upstream,
        Err(e) => {
            // The key is never in the message: reqwest reports the address
            // and the cause, not the headers.
            drop(Metered {
                inner: futures::stream::empty().boxed(),
                scanner: UsageScanner::default(),
                bytes_out: 0,
                info: Some(call(502)),
            });
            return refuse(
                StatusCode::BAD_GATEWAY,
                format!("the model could not be reached: {e}"),
            );
        }
    };

    let status = upstream.status();
    let mut response = Response::builder().status(status);
    for (name, value) in upstream.headers() {
        if matches!(
            name.as_str(),
            "content-type" | "cache-control" | "retry-after" | "content-encoding"
        ) {
            response = response.header(name, value);
        }
    }
    let metered = Metered {
        inner: upstream.bytes_stream().boxed(),
        scanner: UsageScanner::default(),
        bytes_out: 0,
        info: Some(call(status.as_u16())),
    };
    response
        .body(Body::from_stream(metered))
        .unwrap_or_else(|e| refuse(StatusCode::BAD_GATEWAY, e.to_string()))
}

type Reply = Result<Value, Denied>;

fn problem(status: StatusCode, message: impl Into<String>) -> Denied {
    Denied::new(status, message)
}

/// Answers the model-catalogue commands. `None` for any other command.
pub async fn handle_rpc(
    platform: &PlatformState,
    principal: &Principal,
    command: &str,
    args: &Value,
) -> Option<Reply> {
    Some(match command {
        "list_llm_models" => list_models(platform, principal).await,
        "set_llm_models" => set_models(platform, principal, args).await,
        "llm_usage_summary" => usage_summary(platform, principal, args).await,
        _ => return None,
    })
}

async fn list_models(platform: &PlatformState, principal: &Principal) -> Reply {
    let catalog = load_catalog(platform)
        .await
        .map_err(|m| problem(StatusCode::INTERNAL_SERVER_ERROR, m))?;
    let admin = principal.is_platform_admin();
    Ok(Value::Array(
        catalog.iter().map(|m| m.public_json(admin)).collect(),
    ))
}

async fn set_models(platform: &PlatformState, principal: &Principal, args: &Value) -> Reply {
    if !principal.is_platform_admin() {
        return Err(Denied::forbidden(
            "only a Platform Admin can change the models",
        ));
    }
    let models: Vec<ModelEntry> = serde_json::from_value(
        args.get("models").cloned().unwrap_or(Value::Null),
    )
    .map_err(|e| {
        problem(
            StatusCode::BAD_REQUEST,
            format!("the model list is not valid: {e}"),
        )
    })?;
    validate_catalog(&models).map_err(|m| problem(StatusCode::BAD_REQUEST, m))?;
    let value = serde_json::to_value(&models)
        .map_err(|e| problem(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    platform
        .settings
        .set(
            LLM_MODELS_KEY,
            &value,
            principal.actor().map(|user| user.id),
        )
        .await
        .map_err(|e| problem(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Value::Null)
}

async fn usage_summary(platform: &PlatformState, principal: &Principal, args: &Value) -> Reply {
    if !principal.is_platform_admin() {
        return Err(Denied::forbidden("only a Platform Admin can see usage"));
    }
    let days = args
        .get("days")
        .and_then(Value::as_u64)
        .unwrap_or(30)
        .clamp(1, 366);
    let since = chrono::Utc::now() - chrono::Duration::days(days as i64);
    let totals = platform
        .usage
        .totals_since(since)
        .await
        .map_err(|e| problem(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Value::Array(
        totals
            .iter()
            .map(|t| {
                json!({
                    "userId": t.user_id.map(|id| id.to_string()),
                    "displayName": t.display_name,
                    "modelId": t.model_id,
                    "calls": t.calls,
                    "failed": t.failed,
                    "inputTokens": t.input_tokens,
                    "outputTokens": t.output_tokens,
                })
            })
            .collect(),
    ))
}

/// The chat configuration the agent runs with: the browser's reasoning and
/// token limits, on the model the catalogue says, with the server's key.
fn chat_config(entry: &ModelEntry, key: Option<String>, browser: Option<&Value>) -> Value {
    let mut config = browser
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    // Whatever key or address the browser sent is replaced, not trusted.
    for stale in [
        "apiKey",
        "ollamaUrl",
        "customEndpoint",
        "customHeaders",
        "serverModelId",
    ] {
        config.remove(stale);
    }
    let base = entry.base_url.clone().unwrap_or_default();
    let mut set = |name: &str, value: Value| {
        config.insert(name.into(), value);
    };
    set("provider", json!(entry.provider));
    set("model", json!(entry.model));
    set("apiKey", json!(key.unwrap_or_default()));
    set(
        "ollamaUrl",
        json!(if entry.provider == "ollama" {
            base.clone()
        } else {
            String::new()
        }),
    );
    set(
        "customEndpoint",
        json!(if entry.provider == "ollama" {
            String::new()
        } else {
            base
        }),
    );
    set("customHeaders", json!(entry.custom_headers));
    if let Some(mode) = &entry.api_mode {
        set("apiMode", json!(mode));
    }
    if let Some(size) = entry.max_context_size {
        set("maxContextSize", json!(size));
    }
    if let Some(version) = &entry.azure_api_version {
        set("azureApiVersion", json!(version));
    }
    if let Some(family) = &entry.azure_model_family {
        set("azureModelFamily", json!(family));
    }
    if let Some(streaming) = entry.streaming {
        set("streamingEnabled", json!(streaming));
    }
    Value::Object(config)
}

/// The embedding configuration for a catalogue embedding model, keeping the
/// browser's chunking choices.
fn embedding_config(entry: &ModelEntry, key: Option<String>, browser: Option<&Value>) -> Value {
    let mut config = browser
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    for stale in ["apiKey", "endpoint", "extraHeaders", "serverModelId"] {
        config.remove(stale);
    }
    config.insert("enabled".into(), json!(true));
    config.insert(
        "endpoint".into(),
        json!(entry.base_url.clone().unwrap_or_default()),
    );
    config.insert("apiKey".into(), json!(key.unwrap_or_default()));
    config.insert("model".into(), json!(entry.model));
    config.insert("extraHeaders".into(), json!(entry.custom_headers));
    if let Some(dimensions) = entry.output_dimensionality {
        config.insert("outputDimensionality".into(), json!(dimensions));
    }
    Value::Object(config)
}

/// On a platform server the commands that call a model take its name, not an
/// address and a key: this swaps the model's real settings into the request.
pub async fn apply_models(
    platform: &PlatformState,
    command: &str,
    args: &mut Value,
) -> Result<(), Denied> {
    let (wants_chat, embedding_arg, embedding_required) = match command {
        "embedding_fetch" | "embedding_fetch_batch" => (false, Some("cfg"), true),
        "search_project" => (false, Some("embeddingConfig"), false),
        "agent_start_turn" | "agent_start_turn_stream" => (true, Some("embeddingConfig"), false),
        _ => return Ok(()),
    };
    let catalog = load_catalog(platform)
        .await
        .map_err(|m| problem(StatusCode::INTERNAL_SERVER_ERROR, m))?;
    let Some(object) = args.as_object_mut() else {
        return Ok(());
    };
    let key_of = |entry: &ModelEntry| {
        entry
            .key()
            .map_err(|m| problem(StatusCode::SERVICE_UNAVAILABLE, m))
    };

    if wants_chat {
        let browser = object.get("llmConfig").cloned();
        let named = browser
            .as_ref()
            .and_then(|c| c.get("serverModelId"))
            .and_then(Value::as_str)
            .map(str::to_string);
        let entry = match &named {
            Some(id) => find(&catalog, id).filter(|e| e.purpose == Purpose::Chat),
            None => default_for(&catalog, Purpose::Chat),
        }
        .ok_or_else(|| {
            problem(
                StatusCode::SERVICE_UNAVAILABLE,
                "the server has no chat model configured",
            )
        })?;
        let config = chat_config(entry, key_of(entry)?, browser.as_ref());
        object.insert("llmConfig".into(), config);
    }

    if let Some(name) = embedding_arg {
        let browser = object.get(name).cloned();
        let named = browser
            .as_ref()
            .and_then(|c| c.get("serverModelId"))
            .and_then(Value::as_str)
            .map(str::to_string);
        match named {
            Some(id) => {
                let entry = find(&catalog, &id)
                    .filter(|e| e.purpose == Purpose::Embedding)
                    .ok_or_else(|| {
                        problem(
                            StatusCode::BAD_REQUEST,
                            format!("no embedding model \"{id}\" is offered"),
                        )
                    })?;
                let config = embedding_config(entry, key_of(entry)?, browser.as_ref());
                object.insert(name.into(), config);
            }
            None if embedding_required => {
                return Err(problem(
                    StatusCode::BAD_REQUEST,
                    "choose an embedding model the server offers",
                ))
            }
            // Embeddings off for this call: nothing of the browser's goes through.
            None => {
                object.remove(name);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(provider: &str, base: Option<&str>) -> ModelEntry {
        ModelEntry {
            id: "m1".into(),
            label: "Model".into(),
            purpose: Purpose::Chat,
            provider: provider.into(),
            model: "some-model".into(),
            base_url: base.map(str::to_string),
            api_key_env: Some("LLM_WIKI_KEY_M1".into()),
            api_mode: None,
            max_context_size: None,
            azure_api_version: None,
            azure_model_family: None,
            streaming: None,
            vision: false,
            custom_headers: BTreeMap::new(),
            output_dimensionality: None,
            default: false,
        }
    }

    #[test]
    fn a_catalogue_must_name_models_sensibly() {
        assert!(validate_catalog(&[entry("openai", None), {
            let mut second = entry("custom", Some("http://localhost:11434/v1"));
            second.id = "local".into();
            second
        }])
        .is_ok());

        let mut bad_id = entry("openai", None);
        bad_id.id = "Has Space".into();
        assert!(validate_catalog(&[bad_id])
            .unwrap_err()
            .contains("not a usable id"));
        assert!(
            validate_catalog(&[entry("openai", None), entry("openai", None)])
                .unwrap_err()
                .contains("used twice")
        );
        assert!(validate_catalog(&[entry("nonsense", Some("http://x"))])
            .unwrap_err()
            .contains("unknown provider"));
        assert!(validate_catalog(&[entry("custom", None)])
            .unwrap_err()
            .contains("needs a baseUrl"));
        assert!(validate_catalog(&[entry("custom", Some("ftp://x"))]).is_err());
        assert!(
            validate_catalog(&[entry("custom", Some("https://user:pw@x.example"))])
                .unwrap_err()
                .contains("credentials")
        );
    }

    #[test]
    fn the_key_variable_cannot_name_the_servers_own_secrets() {
        for env in [
            "DATABASE_URL",
            "LLM_WIKI_S3_SECRET_KEY",
            "llm_wiki_key_lower",
            "LLM_WIKI_KEY_",
            "LLM_WIKI_KEY_A B",
        ] {
            let mut model = entry("openai", None);
            model.api_key_env = Some(env.into());
            assert!(validate_catalog(&[model]).is_err(), "{env}");
        }
        let mut ok = entry("openai", None);
        ok.api_key_env = Some("LLM_WIKI_KEY_ANTHROPIC_2".into());
        assert!(validate_catalog(&[ok]).is_ok());
    }

    #[test]
    fn built_in_providers_stay_where_the_app_expects_them() {
        assert!(validate_catalog(&[entry("anthropic", Some("https://api.anthropic.com"))]).is_ok());
        let err =
            validate_catalog(&[entry("anthropic", Some("https://evil.example"))]).unwrap_err();
        assert!(err.contains("custom provider"), "{err}");
    }

    #[test]
    fn reserved_headers_and_two_defaults_are_refused() {
        let mut with_auth = entry("custom", Some("http://x"));
        with_auth
            .custom_headers
            .insert("Authorization".into(), "Bearer leaked".into());
        assert!(validate_catalog(&[with_auth])
            .unwrap_err()
            .contains("cannot be set"));

        let (mut a, mut b) = (entry("openai", None), entry("custom", Some("http://x")));
        b.id = "other".into();
        a.default = true;
        b.default = true;
        assert!(validate_catalog(&[a.clone(), b.clone()])
            .unwrap_err()
            .contains("only one chat"));
        b.purpose = Purpose::Embedding;
        assert!(validate_catalog(&[a, b]).is_ok());
    }

    #[test]
    fn a_request_may_only_go_where_the_model_lives() {
        let model = entry("custom", Some("https://gateway.example/llm/v1"));
        assert!(check_target(
            &model,
            "https://gateway.example/llm/v1/chat/completions?x=1"
        )
        .is_ok());
        assert!(check_target(&model, "https://gateway.example/llm/v1").is_ok());
        for bad in [
            "https://other.example/llm/v1/chat/completions",
            "http://gateway.example/llm/v1/chat/completions",
            "https://gateway.example:8443/llm/v1/chat",
            "https://gateway.example/admin",
            "https://gateway.example/llm/v10/chat",
            "https://gateway.example/llm/v1/../admin",
            "https://user:pw@gateway.example/llm/v1/chat",
            "not a url",
        ] {
            assert!(check_target(&model, bad).is_err(), "{bad}");
        }
        let openai = entry("openai", None);
        assert!(check_target(&openai, "https://api.openai.com/v1/chat/completions").is_ok());
        assert!(check_target(
            &openai,
            "https://api.openai.com.evil.example/v1/chat/completions"
        )
        .is_err());
    }

    #[test]
    fn the_key_replaces_whatever_the_caller_put_in_the_auth_headers() {
        let model = entry("custom", Some("http://x"));
        let mut incoming = HeaderMap::new();
        incoming.insert("authorization", "Bearer placeholder".parse().unwrap());
        incoming.insert("x-api-key", "placeholder".parse().unwrap());
        incoming.insert("content-type", "application/json".parse().unwrap());
        incoming.insert("anthropic-version", "2023-06-01".parse().unwrap());
        incoming.insert("x-forwarded-for", "9.9.9.9".parse().unwrap());
        incoming.insert("cookie", "llm_wiki_session=secret".parse().unwrap());
        let out = upstream_headers(&incoming, &model, Some("sk-real"));
        assert_eq!(out["authorization"], "Bearer sk-real");
        assert_eq!(out["x-api-key"], "sk-real");
        assert_eq!(out["content-type"], "application/json");
        assert_eq!(out["anthropic-version"], "2023-06-01");
        assert!(out.get("x-forwarded-for").is_none() && out.get("cookie").is_none());
        // A header the caller did not use is not invented.
        assert!(out.get("api-key").is_none());
    }

    #[test]
    fn without_a_key_the_callers_credentials_are_dropped_not_forwarded() {
        let model = entry("ollama", Some("http://localhost:11434"));
        let mut incoming = HeaderMap::new();
        incoming.insert("authorization", "Bearer smuggled".parse().unwrap());
        let out = upstream_headers(&incoming, &model, None);
        assert!(out.get("authorization").is_none());
    }

    #[test]
    fn a_models_own_headers_are_sent_and_cannot_be_overridden_by_the_caller() {
        let mut model = entry("custom", Some("http://x"));
        model
            .custom_headers
            .insert("X-Model-Provider-Id".into(), "siliconflow".into());
        let mut incoming = HeaderMap::new();
        incoming.insert("x-model-provider-id", "attacker".parse().unwrap());
        let out = upstream_headers(&incoming, &model, None);
        assert_eq!(out["x-model-provider-id"], "siliconflow");
    }

    #[test]
    fn token_counts_are_read_from_either_end_of_a_stream() {
        let mut anthropic = UsageScanner::default();
        anthropic.push(
            br#"event: message_start
data: {"type":"message_start","message":{"usage":{"input_tokens": 25,"output_tokens": 1}}}

"#,
        );
        for _ in 0..40 {
            anthropic.push(b"data: {\"type\":\"content_block_delta\",\"delta\":{\"text\":\"filler filler filler filler\"}}\n\n");
        }
        anthropic.push(br#"data: {"type":"message_delta","usage":{"output_tokens": 61}}"#);
        assert_eq!(anthropic.tokens(), (Some(25), Some(61)));

        let mut openai = UsageScanner::default();
        openai.push(br#"{"choices":[],"usage":{"prompt_tokens":12,"completion_tokens":7,"total_tokens":19}}"#);
        assert_eq!(openai.tokens(), (Some(12), Some(7)));

        let mut google = UsageScanner::default();
        google.push(br#"{"usageMetadata":{"promptTokenCount":3,"candidatesTokenCount":9}}"#);
        assert_eq!(google.tokens(), (Some(3), Some(9)));

        let mut none = UsageScanner::default();
        none.push(b"{\"id\":\"x\"}");
        assert_eq!(none.tokens(), (None, None));
    }

    #[test]
    fn the_default_model_is_the_marked_one_or_the_first_of_its_purpose() {
        let mut a = entry("openai", None);
        let mut b = entry("custom", Some("http://x"));
        b.id = "b".into();
        let mut e = entry("custom", Some("http://x/embed"));
        e.id = "e".into();
        e.purpose = Purpose::Embedding;
        assert_eq!(
            default_for(&[a.clone(), b.clone(), e.clone()], Purpose::Chat)
                .unwrap()
                .id,
            "m1"
        );
        b.default = true;
        assert_eq!(
            default_for(&[a.clone(), b.clone()], Purpose::Chat)
                .unwrap()
                .id,
            "b"
        );
        assert_eq!(
            default_for(&[a.clone(), b, e], Purpose::Embedding)
                .unwrap()
                .id,
            "e"
        );
        a.purpose = Purpose::Embedding;
        assert!(default_for(&[a], Purpose::Chat).is_none());
    }

    #[test]
    fn what_members_see_hides_how_the_key_is_found() {
        let model = entry("openai", None);
        let member = model.public_json(false);
        assert!(member.get("apiKeyEnv").is_none());
        let admin = model.public_json(true);
        assert_eq!(admin["apiKeyEnv"], "LLM_WIKI_KEY_M1");
    }

    #[test]
    fn the_agents_chat_config_comes_from_the_catalogue_not_the_browser() {
        let mut model = entry("custom", Some("https://gw.example/v1"));
        model.api_mode = Some("anthropic_messages".into());
        model.max_context_size = Some(100_000);
        model.custom_headers.insert("X-Team".into(), "wiki".into());
        let browser = json!({
            "provider": "openai", "apiKey": "sk-stolen", "model": "gpt-x",
            "customEndpoint": "https://evil.example", "reasoning": { "mode": "high" },
            "customHeaders": { "Authorization": "Bearer evil" }, "serverModelId": "m1"
        });
        let config = chat_config(&model, Some("sk-real".into()), Some(&browser));
        assert_eq!(config["provider"], "custom");
        assert_eq!(config["model"], "some-model");
        assert_eq!(config["apiKey"], "sk-real");
        assert_eq!(config["customEndpoint"], "https://gw.example/v1");
        assert_eq!(config["apiMode"], "anthropic_messages");
        assert_eq!(config["maxContextSize"], 100_000);
        assert_eq!(config["customHeaders"], json!({ "X-Team": "wiki" }));
        // What the browser may still choose is kept.
        assert_eq!(config["reasoning"], json!({ "mode": "high" }));
        assert!(config.get("serverModelId").is_none());

        let ollama = chat_config(&entry("ollama", Some("http://localhost:11434")), None, None);
        assert_eq!(ollama["ollamaUrl"], "http://localhost:11434");
        assert_eq!(ollama["customEndpoint"], "");
        assert_eq!(ollama["apiKey"], "");
    }

    #[test]
    fn an_embedding_config_takes_address_and_key_from_the_catalogue_and_keeps_chunking() {
        let mut model = entry("custom", Some("https://gw.example/v1/embeddings"));
        model.purpose = Purpose::Embedding;
        model.output_dimensionality = Some(768.0);
        let browser = json!({
            "enabled": false, "endpoint": "https://evil.example", "apiKey": "sk-stolen",
            "model": "x", "maxChunkChars": 900, "overlapChunkChars": 90, "serverModelId": "m1"
        });
        let config = embedding_config(&model, Some("sk-real".into()), Some(&browser));
        assert_eq!(config["enabled"], true);
        assert_eq!(config["endpoint"], "https://gw.example/v1/embeddings");
        assert_eq!(config["apiKey"], "sk-real");
        assert_eq!(config["model"], "some-model");
        assert_eq!(config["outputDimensionality"], 768.0);
        assert_eq!(
            (
                config["maxChunkChars"].clone(),
                config["overlapChunkChars"].clone()
            ),
            (json!(900), json!(90))
        );
        assert!(config.get("serverModelId").is_none());
    }
}
