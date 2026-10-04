//! `POST /rpc/{command}`: the HTTP form of a Tauri `invoke`.

use std::path::Path as FsPath;

use axum::body::Bytes;
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::Credential;
use crate::dispatch::{self, DispatchError};
use crate::paths::PROJECT_SETTING_KEYS;
use crate::worker::PUBLISH_PREFIX;
use crate::AppState;
use llm_wiki_core::EventSink;

const AGENT_TURN_COMMANDS: &[&str] = &["agent_start_turn", "agent_start_turn_stream"];

pub async fn handle(
    State(state): State<AppState>,
    Extension(credential): Extension<Credential>,
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
    match call(&state, credential, &command, args).await {
        Ok(value) => Json(value).into_response(),
        Err(DispatchError::Unknown) => {
            error(StatusCode::NOT_FOUND, format!("Unknown command: {command}"))
        }
        Err(DispatchError::Failed(message)) => error(StatusCode::BAD_REQUEST, message),
    }
}

async fn call(
    state: &AppState,
    credential: Credential,
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
    if dispatch::SERVER_DISABLED.contains(&command) {
        return Err(DispatchError::Failed(format!(
            "`{command}` is not available in server mode"
        )));
    }
    state
        .roots
        .check_args(&args)
        .map_err(DispatchError::Failed)?;
    if AGENT_TURN_COMMANDS.contains(&command) && !state.allow_shell {
        strip_shell_approvals(&mut args);
    }
    match command {
        "app_store_get" | "app_store_set" | "app_store_delete" => {
            app_store(state, command, args).await
        }
        "ingest_command" => ingest_command(state, args).await,
        "review_inbox_append" | "review_inbox_take" => review_inbox(state, command, args).await,
        _ => dispatch::dispatch(&state.core, command, args).await,
    }
}

#[derive(Deserialize)]
struct IngestCommand {
    op: String,
    #[serde(default)]
    args: Value,
}

/// A browser tab's call into the ingest queue, which lives in the worker.
async fn ingest_command(state: &AppState, args: Value) -> Result<Value, DispatchError> {
    let command: IngestCommand = serde_json::from_value(args)
        .map_err(|e| DispatchError::Failed(format!("Invalid arguments: {e}")))?;
    state
        .worker
        .command(&command.op, command.args)
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
#[serde(rename_all = "camelCase")]
struct InboxArgs {
    project_path: String,
    #[serde(default)]
    items: Vec<Value>,
}

/// Review items produced by the worker wait here until a tab with the
/// project open adopts them. Only the tab writes `review.json`, so the
/// worker can never overwrite the owner's review decisions.
async fn review_inbox(
    state: &AppState,
    command: &str,
    args: Value,
) -> Result<Value, DispatchError> {
    let args: InboxArgs = serde_json::from_value(args)
        .map_err(|e| DispatchError::Failed(format!("Invalid arguments: {e}")))?;
    let lock = state.inbox_lock.clone();
    let append = command == "review_inbox_append";
    let project_path = args.project_path.clone();
    let result = tokio::task::spawn_blocking(move || {
        let _guard = lock
            .lock()
            .map_err(|_| "review inbox lock poisoned".to_string())?;
        let path = FsPath::new(&args.project_path).join(".llm-wiki/review-inbox.json");
        let mut items: Vec<Value> = match std::fs::read_to_string(&path) {
            Ok(raw) if !raw.trim().is_empty() => serde_json::from_str(&raw)
                .map_err(|e| format!("{} is corrupt: {e}", path.display()))?,
            _ => Vec::new(),
        };
        let write = |items: &[Value]| -> Result<(), String> {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            }
            let tmp = path.with_extension("json.tmp");
            std::fs::write(&tmp, serde_json::to_vec(items).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
        };
        if append {
            items.extend(args.items);
            write(&items)?;
            Ok(Value::Null)
        } else {
            if !items.is_empty() {
                write(&[])?;
            }
            Ok(Value::Array(items))
        }
    })
    .await
    .map_err(|e| DispatchError::Failed(e.to_string()))?
    .map_err(DispatchError::Failed)?;
    if append {
        let _ = state.events.emit_value(
            "ingest://review-inbox",
            json!({ "projectPath": project_path }),
        );
    }
    Ok(result)
}

/// Without `--allow-shell`, approvals sent by the browser are ignored so
/// the agent can never run a command on the server.
fn strip_shell_approvals(args: &mut Value) {
    if let Some(request) = args.get_mut("request").and_then(Value::as_object_mut) {
        request.insert("approvedShellCommands".into(), json!([]));
        request.remove("shellCommand");
    }
}

#[derive(Deserialize)]
struct StoreArgs {
    store: String,
    key: String,
    #[serde(default)]
    value: Value,
}

async fn app_store(state: &AppState, command: &str, args: Value) -> Result<Value, DispatchError> {
    let args: StoreArgs = serde_json::from_value(args)
        .map_err(|e| DispatchError::Failed(format!("Invalid arguments: {e}")))?;
    if command == "app_store_set" && PROJECT_SETTING_KEYS.contains(&args.key.as_str()) {
        state
            .roots
            .check_project_setting(&args.value)
            .map_err(DispatchError::Failed)?;
    }
    let store = state.store.clone();
    let command = command.to_string();
    let (store_name, key) = (args.store.clone(), args.key.clone());
    let changes = command != "app_store_get";
    let result = tokio::task::spawn_blocking(move || match command.as_str() {
        "app_store_get" => store.get(&args.store, &args.key),
        "app_store_set" => store
            .set(&args.store, &args.key, args.value)
            .map(|()| Value::Null),
        _ => store.delete(&args.store, &args.key).map(Value::Bool),
    })
    .await
    .map_err(|e| DispatchError::Failed(e.to_string()))?
    .map_err(DispatchError::Failed)?;
    if changes {
        // Lets the ingest worker pick up settings edited in a tab.
        let _ = state.events.emit_value(
            "app-store://changed",
            json!({ "store": store_name, "key": key }),
        );
    }
    Ok(result)
}

fn error(status: StatusCode, message: String) -> Response {
    (status, Json(json!({ "error": message }))).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_browser_supplied_shell_approvals() {
        let mut args = json!({
            "projectId": "p",
            "request": { "message": "hi", "approvedShellCommands": ["rm -rf /"], "shellCommand": "rm -rf /" }
        });
        strip_shell_approvals(&mut args);
        assert_eq!(
            args["request"],
            json!({ "message": "hi", "approvedShellCommands": [] })
        );
    }
}
