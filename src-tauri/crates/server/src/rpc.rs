//! `POST /rpc/{command}`: the HTTP form of a Tauri `invoke`.

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::dispatch::{self, DispatchError};
use crate::paths::PROJECT_SETTING_KEYS;
use crate::AppState;

const AGENT_TURN_COMMANDS: &[&str] = &["agent_start_turn", "agent_start_turn_stream"];

pub async fn handle(
    State(state): State<AppState>,
    Path(command): Path<String>,
    body: Bytes,
) -> Response {
    let args = if body.is_empty() {
        json!({})
    } else {
        match serde_json::from_slice::<Value>(&body) {
            Ok(args) => args,
            Err(e) => return error(StatusCode::BAD_REQUEST, format!("Invalid JSON body: {e}")),
        }
    };
    match call(&state, &command, args).await {
        Ok(value) => Json(value).into_response(),
        Err(DispatchError::Unknown) => {
            error(StatusCode::NOT_FOUND, format!("Unknown command: {command}"))
        }
        Err(DispatchError::Failed(message)) => error(StatusCode::BAD_REQUEST, message),
    }
}

async fn call(state: &AppState, command: &str, mut args: Value) -> Result<Value, DispatchError> {
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
        _ => dispatch::dispatch(&state.core, command, args).await,
    }
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
    tokio::task::spawn_blocking(move || match command.as_str() {
        "app_store_get" => store.get(&args.store, &args.key),
        "app_store_set" => store
            .set(&args.store, &args.key, args.value)
            .map(|()| Value::Null),
        _ => store.delete(&args.store, &args.key).map(Value::Bool),
    })
    .await
    .map_err(|e| DispatchError::Failed(e.to_string()))?
    .map_err(DispatchError::Failed)
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
