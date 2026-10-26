//! `POST /rpc/{command}`: the HTTP form of a Tauri `invoke`.


use axum::body::Bytes;
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::Credential;
use crate::dispatch::{self, DispatchError};
use crate::principal::Principal;
use crate::worker::PUBLISH_PREFIX;
use crate::AppState;
use crate::{accounts_rpc, authz};
use llm_wiki_core::EventSink;

/// The commands of the shared backend a platform server serves. Everything
/// else still works on host folders, so on a platform server it answers that
/// it is unavailable rather than touching the host disk. A command joins this
/// list when it has been moved onto the storage layer (or never needed files),
/// which keeps a newly added command off by default.
const PLATFORM_COMMANDS: &[&str] = &[
    // Files, served from Postgres and the object store.
    "read_file",
    "write_file",
    "write_file_base64",
    "write_file_atomic",
    "list_directory",
    "copy_file",
    "copy_directory",
    "preprocess_file",
    "delete_file",
    "create_directory",
    "file_exists",
    "get_file_modified_time",
    "get_file_size",
    "get_file_md5",
    "read_file_as_base64",
    // Retrieval and the agent, over the stored pages.
    "search_project",
    "get_page_links",
    "find_related_wiki_pages",
    // Embeddings in pgvector, for semantic search.
    "vector_upsert_chunks",
    "vector_search_chunks",
    "vector_delete_page",
    "vector_count_chunks",
    "vector_clear_chunks",
    "vector_optimize_chunks",
    "vector_legacy_row_count",
    "vector_drop_legacy",
    "agent_start_turn",
    "agent_start_turn_stream",
    "agent_cancel_turn",
    "agent_get_session",
    "agent_list_sessions",
    "agent_list_skills",
    // No Project files involved.
    "embedding_fetch",
    "embedding_fetch_batch",
    "web_search",
    "set_proxy_env",
];

fn available_on_platform(command: &str) -> bool {
    PLATFORM_COMMANDS.contains(&command)
}

pub async fn handle(
    State(state): State<AppState>,
    Extension(credential): Extension<Credential>,
    principal: Option<Extension<Principal>>,
    Path(command): Path<String>,
    body: Bytes,
) -> Response {
    if credential == Credential::Worker {
        state.worker.touch();
    }
    let args = if body.is_empty() {
        json!({})
    } else {
        match serde_json::from_slice::<Value>(&body) {
            Ok(args) => args,
            Err(e) => return error(StatusCode::BAD_REQUEST, format!("Invalid JSON body: {e}")),
        }
    };
    let principal = principal.map(|Extension(principal)| principal);
    match call(&state, credential, principal, &command, args).await {
        Ok(value) => Json(value).into_response(),
        Err(DispatchError::Unknown) => {
            error(StatusCode::NOT_FOUND, format!("Unknown command: {command}"))
        }
        Err(DispatchError::Failed(message)) => error(StatusCode::BAD_REQUEST, message),
        Err(DispatchError::Status(status, message)) => error(status, message),
    }
}

async fn call(
    state: &AppState,
    credential: Credential,
    principal: Option<Principal>,
    command: &str,
    mut args: Value,
) -> Result<Value, DispatchError> {
    if command.starts_with("worker_") {
        if credential != Credential::Worker {
            return Err(DispatchError::Failed(format!(
                "`{command}` is reserved for the ingest worker"
            )));
        }
        return worker_call(state, command, args).map_err(DispatchError::Failed);
    }
    let platform_files = llm_wiki_core::platform::require().map_err(DispatchError::Failed)?;
    crate::paths::check_platform_args(&platform_files.locator, &args)
        .map_err(DispatchError::Failed)?;
    // The caller is a known principal, and what they ask for is checked
    // against their roles before anything runs.
    if let Some(principal) = &principal {
        let platform = &state.platform;
        if let Some(reply) = accounts_rpc::handle(platform, principal, command, args.clone()).await
        {
            return reply.map_err(|denied| DispatchError::Status(denied.status, denied.message));
        }
        if let Some(reply) = crate::llm::handle_rpc(platform, principal, command, &args).await {
            return reply.map_err(|denied| DispatchError::Status(denied.status, denied.message));
        }
        authz::authorize_rpc(platform, principal, command, &mut args)
            .await
            .map_err(|denied| DispatchError::Status(denied.status, denied.message))?;
        crate::llm::apply_models(platform, principal, command, &mut args)
            .await
            .map_err(|denied| DispatchError::Status(denied.status, denied.message))?;
    }
    match command {
        "app_store_get" | "app_store_set" | "app_store_delete" => {
            app_store(state, principal.as_ref(), command, args).await
        }
        "ingest_command" => ingest_command(state, principal.as_ref(), args).await,
        // Where the web file picker may browse and stage uploads: nothing on
        // the host is browsable.
        "server_locations" => Ok(json!({ "roots": [], "uploads": "" })),
        _ if !available_on_platform(command) => Err(
            DispatchError::Failed(llm_wiki_core::platform::unsupported(command)),
        ),
        _ => dispatch::dispatch(&state.core, command, args).await,
    }
}

#[derive(Deserialize)]
struct IngestCommand {
    op: String,
    #[serde(default)]
    args: Value,
    /// The project the caller has open. Required on a platform server.
    #[serde(default, rename = "projectId")]
    project_id: Option<String>,
}

/// A browser tab's call into the ingest queue, which lives in the worker.
async fn ingest_command(
    state: &AppState,
    principal: Option<&Principal>,
    args: Value,
) -> Result<Value, DispatchError> {
    let command: IngestCommand = serde_json::from_value(args)
        .map_err(|e| DispatchError::Failed(format!("Invalid arguments: {e}")))?;
    // The queue acts on a Project's files, so the caller must be able to
    // change them there; looking at the queue takes a viewer.
    if let Some(principal) = principal {
        let platform = &state.platform;
        if !matches!(principal, Principal::Worker) {
            let project = command
                .project_id
                .as_deref()
                .and_then(|id| uuid::Uuid::parse_str(id).ok())
                .map(llm_wiki_storage::ProjectId)
                .ok_or_else(|| DispatchError::Failed("`projectId` is required".into()))?;
            let need = if command.op == "snapshot" {
                llm_wiki_storage::Role::Viewer
            } else {
                llm_wiki_storage::Role::Editor
            };
            authz::require_role(platform, principal, project, need)
                .await
                .map_err(|denied| DispatchError::Status(denied.status, denied.message))?;
        }
    }
    state
        .worker
        .command(&command.op, command.args, command.project_id.as_deref())
        .await
        .map_err(DispatchError::Failed)
}

#[derive(Deserialize)]
struct WorkerReply {
    id: String,
    #[serde(default)]
    result: Value,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Deserialize)]
struct WorkerPublish {
    event: String,
    #[serde(default)]
    payload: Value,
}

fn worker_call(state: &AppState, command: &str, args: Value) -> Result<Value, String> {
    let invalid = |e: serde_json::Error| format!("Invalid arguments: {e}");
    match command {
        "worker_heartbeat" => Ok(Value::Null),
        "worker_reply" => {
            let reply: WorkerReply = serde_json::from_value(args).map_err(invalid)?;
            let outcome = match reply.error {
                Some(error) => Err(error),
                None => Ok(reply.result),
            };
            state.worker.reply(&reply.id, outcome);
            Ok(Value::Null)
        }
        "worker_publish" => {
            let publish: WorkerPublish = serde_json::from_value(args).map_err(invalid)?;
            if !publish.event.starts_with(PUBLISH_PREFIX) {
                return Err(format!(
                    "The worker may only publish {PUBLISH_PREFIX}* events"
                ));
            }
            state.events.emit_value(&publish.event, publish.payload)?;
            Ok(Value::Null)
        }
        _ => Err(format!("Unknown worker command: {command}")),
    }
}




#[derive(Deserialize)]
struct StoreArgs {
    store: String,
    key: String,
    #[serde(default)]
    value: Value,
}

/// Settings that belong to the person, not the organisation: where they left
/// off, how they like the screen. Everything else in the store is configuration
/// the whole platform shares, such as models and endpoints.
const PER_USER_KEYS: &[&str] = &[
    "lastProject",
    "recentProjects",
    "projectRegistry",
    "zoomLevel",
    "updateCheckState",
    "outputLanguage",
    "generalConfig",
    "activeServerModelId",
];

/// Settings that carry API keys or tokens. On a platform server the models
/// and their keys belong to the server, so none of these is stored in, or read
/// from, the settings store.
const SERVER_OWNED_KEYS: &[&str] = &[
    "llmConfig",
    "providerConfigs",
    "customLlmPresets",
    "embeddingConfig",
    "multimodalConfig",
    "searchApiConfig",
    "mineruConfig",
    "apiConfig",
    "proxyConfig",
];

/// A signed-in user's own copy of a personal setting, in their preferences.
async fn user_setting(
    platform: &crate::principal::PlatformState,
    user: &llm_wiki_storage::User,
    command: &str,
    args: StoreArgs,
) -> Result<Value, DispatchError> {
    let failed = |error: llm_wiki_storage::StoreError| DispatchError::Failed(error.to_string());
    let key = format!("{}:{}", args.store, args.key);
    match command {
        "app_store_get" => Ok(platform
            .accounts
            .preference(user.id, &key)
            .await
            .map_err(failed)?
            .unwrap_or(Value::Null)),
        "app_store_set" => {
            platform
                .accounts
                .set_preference(user.id, &key, &args.value)
                .await
                .map_err(failed)?;
            Ok(Value::Null)
        }
        _ => platform
            .accounts
            .delete_preference(user.id, &key)
            .await
            .map(Value::Bool)
            .map_err(failed),
    }
}

async fn app_store(
    state: &AppState,
    principal: Option<&Principal>,
    command: &str,
    args: Value,
) -> Result<Value, DispatchError> {
    let args: StoreArgs = serde_json::from_value(args)
        .map_err(|e| DispatchError::Failed(format!("Invalid arguments: {e}")))?;
    let platform = &state.platform;
    let failed = |error: llm_wiki_storage::StoreError| DispatchError::Failed(error.to_string());
    // A person's own settings live in their preferences. The organisation's
    // are read by everyone and changed only by a Platform Admin.
    if SERVER_OWNED_KEYS.contains(&args.key.as_str()) {
        return match command {
            "app_store_get" => Ok(Value::Null),
            "app_store_delete" => Ok(Value::Bool(false)),
            _ => Err(DispatchError::Status(
                StatusCode::FORBIDDEN,
                "model and search settings are kept on the server; an administrator sets them in Settings → Models"
                    .into(),
            )),
        };
    }
    if let Some(Principal::User(user)) = principal {
        if PER_USER_KEYS.contains(&args.key.as_str()) {
            return user_setting(platform, user, command, args).await;
        }
    }
    if command != "app_store_get" && !principal.is_some_and(Principal::is_platform_admin) {
        return Err(DispatchError::Status(
            StatusCode::FORBIDDEN,
            "only a Platform Admin can change these settings".into(),
        ));
    }
    let name = shared_setting_key(&args.store, &args.key).map_err(DispatchError::Failed)?;
    let actor = principal.and_then(|principal| principal.actor()).map(|user| user.id);
    let result = match command {
        "app_store_get" => platform
            .settings
            .get(&name)
            .await
            .map_err(failed)?
            .unwrap_or(Value::Null),
        "app_store_set" => {
            platform
                .settings
                .set(&name, &args.value, actor)
                .await
                .map_err(failed)?;
            Value::Null
        }
        _ => Value::Bool(platform.settings.delete(&name).await.map_err(failed)?),
    };
    if command != "app_store_get" {
        // Lets the ingest worker pick up settings edited in a tab.
        let _ = state.events.emit_value(
            "app-store://changed",
            json!({ "store": args.store, "key": args.key }),
        );
    }
    Ok(result)
}

/// Where the organisation's copy of a setting is kept: `server_settings`,
/// under the store and key the frontend names.
fn shared_setting_key(store: &str, key: &str) -> Result<String, String> {
    let valid = store.ends_with(".json")
        && !store.starts_with('.')
        && store
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'));
    if !valid {
        return Err(format!("Invalid store name: {store}"));
    }
    Ok(format!("app:{store}:{key}"))
}

fn error(status: StatusCode, message: String) -> Response {
    (status, Json(json!({ "error": message }))).into_response()
}

#[cfg(test)]
mod platform_tests {
    use super::*;

    #[test]
    fn a_platform_server_serves_only_the_commands_moved_onto_storage() {
        for command in [
            "read_file",
            "write_file",
            "search_project",
            "agent_start_turn",
        ] {
            assert!(available_on_platform(command), "{command}");
        }
        // Still working on host folders.
        for command in [
            "create_project",
            "open_project",
            "export_project_archive",
            "list_file_history",
            "vector_upsert",
            "vector_search",
            "extract_and_save_pdf_images_cmd",
            "apply_text_selection_edit",
            "anytxt_search",
            "an_unknown_command",
        ] {
            assert!(!available_on_platform(command), "{command}");
        }
    }
}
