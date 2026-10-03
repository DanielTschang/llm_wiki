//! Agent chat commands invoked by the UI (desktop IPC or server RPC).
//! The external HTTP API has its own entry points in `api_server.rs`.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::context::CoreContext;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentProjectEntry {
    id: String,
    name: String,
    path: String,
    current: bool,
}

#[derive(Debug, Clone, Default)]
struct AgentRuntimeConfig {
    embedding: Option<crate::commands::search::SearchEmbeddingConfig>,
    llm: Option<crate::agent::provider::LlmConfig>,
    web_search: Option<crate::agent::tools::WebSearchConfig>,
    anytxt: Option<crate::agent::tools::AnyTxtConfig>,
}

pub async fn agent_start_turn(
    ctx: &CoreContext,
    project_id: String,
    mut request: crate::agent::AgentChatRequest,
    llm_config: Option<crate::agent::provider::LlmConfig>,
) -> Result<crate::agent::types::AgentChatResponse, String> {
    let project = resolve_agent_project(ctx, &project_id)?;
    if request
        .session_id
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .is_empty()
    {
        request.session_id = Some(format!("ui_{}", Uuid::new_v4()));
    }
    let active_session_id = request.session_id.clone().unwrap_or_default();
    if request
        .run_id
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .is_empty()
    {
        request.run_id = Some(format!("run_{}", Uuid::new_v4()));
    }
    let active_run_id = request.run_id.clone().unwrap_or_default();
    if let Some(session_id) = request.session_id.clone() {
        if request.history.is_empty() && !request.history_explicit {
            request.history = ctx
                .agent_sessions
                .recent_messages(&project.path, &session_id, 12)
                .into_iter()
                .map(|message| crate::agent::types::AgentConversationMessage {
                    role: message.role,
                    content: message.content,
                })
                .collect();
        }
    }
    let mut runtime_config = load_agent_runtime_config(ctx);
    runtime_config.llm = llm_config.or(runtime_config.llm);
    let runtime = crate::agent::AgentRuntime::new(
        project.id.clone(),
        project.path.clone(),
        runtime_config.embedding,
        runtime_config.llm,
        runtime_config.web_search,
        runtime_config.anytxt,
    );
    let user_message = request.message.clone();
    let persist_session = request.persist_session;
    let cancellation =
        ctx.agent_cancellation
            .start(&project.id, &active_session_id, &active_run_id);
    let result = runtime
        .run_once_with_cancel(request, Some(cancellation))
        .await;
    ctx.agent_cancellation
        .finish(&project.id, &active_session_id, &active_run_id);
    let response = result?;
    if persist_session {
        ctx.agent_sessions.append_turn(
            &project.path,
            &project.id,
            &response.session_id,
            &user_message,
            &response.message,
        );
    }
    Ok(response)
}

pub fn agent_cancel_turn(
    ctx: &CoreContext,
    project_id: String,
    session_id: String,
    run_id: Option<String>,
) -> Result<bool, String> {
    let project = resolve_agent_project(ctx, &project_id)?;
    Ok(ctx
        .agent_cancellation
        .cancel(&project.id, &session_id, run_id.as_deref()))
}

pub async fn agent_start_turn_stream(
    ctx: Arc<CoreContext>,
    project_id: String,
    mut request: crate::agent::AgentChatRequest,
    llm_config: Option<crate::agent::provider::LlmConfig>,
) -> Result<String, String> {
    let project = resolve_agent_project(&ctx, &project_id)?;
    if request
        .session_id
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .is_empty()
    {
        request.session_id = Some(format!("ui_{}", Uuid::new_v4()));
    }
    let active_session_id = request.session_id.clone().unwrap_or_default();
    if request
        .run_id
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .is_empty()
    {
        request.run_id = Some(format!("run_{}", Uuid::new_v4()));
    }
    let active_run_id = request.run_id.clone().unwrap_or_default();
    if request.history.is_empty() && !request.history_explicit {
        request.history = ctx
            .agent_sessions
            .recent_messages(&project.path, &active_session_id, 12)
            .into_iter()
            .map(|message| crate::agent::types::AgentConversationMessage {
                role: message.role,
                content: message.content,
            })
            .collect();
    }
    let mut runtime_config = load_agent_runtime_config(&ctx);
    runtime_config.llm = llm_config.or(runtime_config.llm);
    let runtime = crate::agent::AgentRuntime::new(
        project.id.clone(),
        project.path.clone(),
        runtime_config.embedding,
        runtime_config.llm,
        runtime_config.web_search,
        runtime_config.anytxt,
    );
    let ctx_for_task = Arc::clone(&ctx);
    let project_for_task = project.clone();
    let session_for_task = active_session_id.clone();
    let run_for_task = active_run_id.clone();
    let user_message = request.message.clone();
    let persist_session = request.persist_session;
    let cancellation =
        ctx.agent_cancellation
            .start(&project.id, &active_session_id, &active_run_id);
    crate::runtime::spawn(async move {
        let events = ctx_for_task.events().clone();
        let emit_session = session_for_task.clone();
        let emit_run = run_for_task.clone();
        let sink: crate::agent::runtime::AgentEventSink = std::sync::Arc::new(move |event| {
            let _ = events.emit(
                "agent-event",
                serde_json::json!({
                    "sessionId": emit_session.clone(),
                    "runId": emit_run.clone(),
                    "event": event,
                }),
            );
        });
        let result = runtime
            .run_once_with_cancel_and_events(request, Some(cancellation), Some(sink))
            .await;
        ctx_for_task.agent_cancellation.finish(
            &project_for_task.id,
            &session_for_task,
            &run_for_task,
        );
        match result {
            Ok(response) => {
                if persist_session {
                    ctx_for_task.agent_sessions.append_turn(
                        &project_for_task.path,
                        &project_for_task.id,
                        &response.session_id,
                        &user_message,
                        &response.message,
                    );
                }
            }
            Err(err) => {
                let _ = ctx_for_task.events().emit(
                    "agent-event",
                    serde_json::json!({
                        "sessionId": session_for_task,
                        "runId": run_for_task,
                        "event": { "type": "error", "message": err },
                    }),
                );
            }
        }
    });
    Ok(active_session_id)
}

pub fn agent_get_session(
    ctx: &CoreContext,
    project_id: String,
    session_id: String,
    limit: Option<usize>,
) -> Result<Vec<crate::agent::session::AgentSessionMessage>, String> {
    let project = resolve_agent_project(ctx, &project_id)?;
    Ok(ctx.agent_sessions.recent_messages(
        &project.path,
        &session_id,
        limit.unwrap_or(40).clamp(1, 200),
    ))
}

pub fn agent_list_sessions(
    ctx: &CoreContext,
    project_id: String,
) -> Result<Vec<crate::agent::session::AgentSession>, String> {
    let project = resolve_agent_project(ctx, &project_id)?;
    Ok(ctx.agent_sessions.list_sessions(&project.path))
}

fn resolve_agent_project(ctx: &CoreContext, project_id: &str) -> Result<AgentProjectEntry, String> {
    let decoded = percent_decode(project_id);
    let wants_current = decoded.eq_ignore_ascii_case("current");
    load_agent_projects(ctx)
        .into_iter()
        .find(|project| {
            project.id == decoded
                || project_path_matches(&project.path, &decoded)
                || (wants_current && project.current)
        })
        .ok_or_else(|| format!("Unknown project: {decoded}"))
}

fn load_agent_projects(ctx: &CoreContext) -> Vec<AgentProjectEntry> {
    let current = normalize_path(&crate::clip_server::current_project_path());
    let mut projects = Vec::new();
    if let Some(parsed) = load_agent_app_state(ctx) {
        if let Some(registry) = parsed.get("projectRegistry").and_then(Value::as_object) {
            for (id, value) in registry {
                let path = value.get("path").and_then(Value::as_str).unwrap_or("");
                if path.is_empty() {
                    continue;
                }
                let path = normalize_path(path);
                let name = value
                    .get("name")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| project_name_from_path(&path));
                projects.push(AgentProjectEntry {
                    id: id.clone(),
                    name,
                    current: path == current,
                    path,
                });
            }
        }
        if let Some(recents) = parsed.get("recentProjects").and_then(Value::as_array) {
            for value in recents {
                let path = value.get("path").and_then(Value::as_str).unwrap_or("");
                if path.is_empty() {
                    continue;
                }
                let path = normalize_path(path);
                if projects.iter().any(|project| project.path == path) {
                    continue;
                }
                let name = value
                    .get("name")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| project_name_from_path(&path));
                projects.push(AgentProjectEntry {
                    id: read_project_id(&path).unwrap_or_else(|| path.clone()),
                    name,
                    current: path == current,
                    path,
                });
            }
        }
    }
    if !current.is_empty() && !projects.iter().any(|project| project.path == current) {
        projects.push(AgentProjectEntry {
            id: read_project_id(&current).unwrap_or_else(|| current.clone()),
            name: project_name_from_path(&current),
            current: true,
            path: current,
        });
    }
    projects
}

fn load_agent_app_state(ctx: &CoreContext) -> Option<Value> {
    let path = ctx.app_state_path()?;
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn load_agent_runtime_config(ctx: &CoreContext) -> AgentRuntimeConfig {
    let Some(parsed) = load_agent_app_state(ctx) else {
        return AgentRuntimeConfig::default();
    };
    AgentRuntimeConfig {
        embedding: parsed
            .get("embeddingConfig")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok()),
        llm: parsed
            .get("llmConfig")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok()),
        web_search: parsed
            .get("searchApiConfig")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok()),
        anytxt: parsed
            .get("searchApiConfig")
            .and_then(|value| value.get("anyTxt"))
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok()),
    }
}

fn read_project_id(path: &str) -> Option<String> {
    let raw = std::fs::read_to_string(
        std::path::Path::new(path)
            .join(".llm-wiki")
            .join("project.json"),
    )
    .ok()?;
    serde_json::from_str::<Value>(&raw)
        .ok()?
        .get("id")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn project_name_from_path(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("Project")
        .to_string()
}

fn project_path_matches(stored_path: &str, candidate: &str) -> bool {
    let stored = normalize_path(stored_path);
    let candidate = normalize_path(candidate);
    if cfg!(windows) {
        stored.eq_ignore_ascii_case(&candidate)
    } else {
        stored == candidate
    }
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/").trim_end_matches('/').to_string()
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push((hi << 4) | lo);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| input.to_string())
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}
