//! Host-provided services the core needs from whichever shell runs it:
//! where app settings live, how to push events to the UI, and the
//! long-lived state shared between commands and background servers.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;
use serde_json::Value;

use crate::agent::cancel::AgentCancellationRegistry;
use crate::agent::session::AgentSessionStore;
use crate::commands::claude_cli::ClaudeCliState;
use crate::commands::codex_cli::CodexCliState;
use crate::commands::file_sync::FileSyncState;

/// Delivers a named event to every connected UI (Tauri webview events on
/// desktop, the SSE stream in server mode).
pub trait EventSink: Send + Sync + 'static {
    fn emit_value(&self, event: &str, payload: Value) -> Result<(), String>;
}

/// Cheaply cloneable handle to the host's [`EventSink`].
#[derive(Clone)]
pub struct EventEmitter(Arc<dyn EventSink>);

impl EventEmitter {
    pub fn new(sink: impl EventSink) -> Self {
        Self(Arc::new(sink))
    }

    /// Drops every event. For tests and hosts without a UI.
    pub fn noop() -> Self {
        struct Noop;
        impl EventSink for Noop {
            fn emit_value(&self, _event: &str, _payload: Value) -> Result<(), String> {
                Ok(())
            }
        }
        Self::new(Noop)
    }

    pub fn emit<S: Serialize>(&self, event: &str, payload: S) -> Result<(), String> {
        let payload = serde_json::to_value(payload).map_err(|e| e.to_string())?;
        self.0.emit_value(event, payload)
    }
}

pub struct CoreContext {
    app_data_dir: Option<PathBuf>,
    app_version: String,
    events: EventEmitter,
    pub file_sync: FileSyncState,
    pub claude_cli: ClaudeCliState,
    pub codex_cli: CodexCliState,
    pub agent_sessions: AgentSessionStore,
    pub agent_cancellation: AgentCancellationRegistry,
}

impl CoreContext {
    /// `app_data_dir` holds `app-state.json`, the settings file the
    /// frontend writes through its key-value store.
    pub fn new(
        app_data_dir: Option<PathBuf>,
        app_version: impl Into<String>,
        events: EventEmitter,
    ) -> Self {
        Self {
            app_data_dir,
            app_version: app_version.into(),
            events,
            file_sync: FileSyncState::default(),
            claude_cli: ClaudeCliState::default(),
            codex_cli: CodexCliState::default(),
            agent_sessions: AgentSessionStore::default(),
            agent_cancellation: AgentCancellationRegistry::default(),
        }
    }

    pub fn app_data_dir(&self) -> Option<&Path> {
        self.app_data_dir.as_deref()
    }

    pub fn app_state_path(&self) -> Option<PathBuf> {
        self.app_data_dir
            .as_ref()
            .map(|dir| dir.join("app-state.json"))
    }

    /// Version of the host application (not of this crate).
    pub fn app_version(&self) -> &str {
        &self.app_version
    }

    pub fn events(&self) -> &EventEmitter {
        &self.events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Recorder(Arc<Mutex<Vec<(String, Value)>>>);

    impl EventSink for Recorder {
        fn emit_value(&self, event: &str, payload: Value) -> Result<(), String> {
            self.0.lock().unwrap().push((event.to_string(), payload));
            Ok(())
        }
    }

    #[test]
    fn emitter_serializes_payload_for_the_sink() {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Payload {
            project_id: &'static str,
        }
        let log = Arc::new(Mutex::new(Vec::new()));
        let emitter = EventEmitter::new(Recorder(Arc::clone(&log)));

        emitter
            .emit("file-sync://changed", Payload { project_id: "p1" })
            .unwrap();

        assert_eq!(
            *log.lock().unwrap(),
            vec![(
                "file-sync://changed".to_string(),
                serde_json::json!({ "projectId": "p1" })
            )]
        );
    }

    #[test]
    fn app_state_path_lives_in_app_data_dir() {
        let ctx = CoreContext::new(Some(PathBuf::from("/data")), "1.2.3", EventEmitter::noop());
        assert_eq!(
            ctx.app_state_path(),
            Some(PathBuf::from("/data/app-state.json"))
        );
        assert_eq!(ctx.app_version(), "1.2.3");

        let headless = CoreContext::new(None, "1.2.3", EventEmitter::noop());
        assert_eq!(headless.app_state_path(), None);
    }
}
