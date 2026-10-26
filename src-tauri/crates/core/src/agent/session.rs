use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

const MAX_SESSION_MESSAGES: usize = 40;
// Bound only the in-memory cache. Session files stay on disk so API/MCP callers
// can resume old conversations without the desktop UI keeping every session hot.
const MAX_CACHED_SESSIONS: usize = 128;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentSessionMessage {
    pub role: String,
    pub content: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSession {
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub project_id: String,
    pub messages: Vec<AgentSessionMessage>,
    pub updated_at: u64,
}

#[derive(Debug, Default)]
pub struct AgentSessionStore {
    inner: Mutex<BTreeMap<String, AgentSession>>,
}

impl AgentSessionStore {
    pub fn append_turn(
        &self,
        project_path: &str,
        project_id: &str,
        session_id: &str,
        user: &str,
        assistant: &str,
    ) {
        let now = now_ms();
        let Ok(mut guard) = self.inner.lock() else {
            return;
        };
        let cache_key = session_cache_key(project_path, session_id);
        let session = guard
            .entry(cache_key)
            .or_default();
        session.session_id = session_id.to_string();
        session.project_id = project_id.to_string();
        session.messages.push(AgentSessionMessage {
            role: "user".to_string(),
            content: user.to_string(),
            timestamp: now,
        });
        session.messages.push(AgentSessionMessage {
            role: "assistant".to_string(),
            content: assistant.to_string(),
            timestamp: now,
        });
        if session.messages.len() > MAX_SESSION_MESSAGES {
            let drop_count = session.messages.len() - MAX_SESSION_MESSAGES;
            session.messages.drain(0..drop_count);
        }
        session.updated_at = now;
        trim_session_cache(&mut guard);
    }

    pub fn recent_messages(
        &self,
        project_path: &str,
        session_id: &str,
        limit: usize,
    ) -> Vec<AgentSessionMessage> {
        let session = self
            .inner
            .lock()
            .ok()
            .and_then(|guard| {
                guard
                    .get(&session_cache_key(project_path, session_id))
                    .cloned()
            });
        let Some(session) = session else {
            return Vec::new();
        };
        let start = session.messages.len().saturating_sub(limit);
        session.messages[start..].to_vec()
    }

    pub fn list_sessions(&self, project_path: &str) -> Vec<AgentSession> {
        // Sessions live in memory only, until `agent_sessions` has a table.
        let _ = project_path;
        Vec::new()
    }
}

fn session_cache_key(project_path: &str, session_id: &str) -> String {
    format!("{}::{session_id}", normalize_project_path(project_path))
}

fn normalize_project_path(path: &str) -> String {
    path.replace('\\', "/").trim_end_matches('/').to_string()
}

fn trim_session_cache(cache: &mut BTreeMap<String, AgentSession>) {
    if cache.len() <= MAX_CACHED_SESSIONS {
        return;
    }
    let mut entries = cache
        .iter()
        .map(|(key, session)| (key.clone(), session.updated_at))
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    let remove_count = cache.len().saturating_sub(MAX_CACHED_SESSIONS);
    for (key, _) in entries.into_iter().take(remove_count) {
        cache.remove(&key);
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}





#[cfg(test)]
mod tests {
    

    








}
