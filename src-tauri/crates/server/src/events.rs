//! Core events → Server-Sent Events. Every connected browser tab receives
//! every event as `{ "event": name, "payload": … }`, which is what the
//! web platform's `listen()` expects.

use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Extension, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::Stream;
use llm_wiki_core::EventSink;
use serde_json::{json, Value};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::principal::Principal;
use crate::AppState;

const CHANNEL_CAPACITY: usize = 1024;

#[derive(Clone)]
pub struct EventHub {
    sender: broadcast::Sender<Arc<str>>,
}

impl EventHub {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(CHANNEL_CAPACITY);
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Arc<str>> {
        self.sender.subscribe()
    }
}

impl EventSink for EventHub {
    fn emit_value(&self, event: &str, payload: Value) -> Result<(), String> {
        let message: Arc<str> = json!({ "event": event, "payload": payload })
            .to_string()
            .into();
        // No open tab is not an error: callers such as the CLI stream
        // readers stop their work when emit fails.
        let _ = self.sender.send(message);
        Ok(())
    }
}

/// The Project an event is about, if its payload names one.
fn event_project(payload: &Value) -> Option<llm_wiki_storage::ProjectId> {
    let locator = llm_wiki_storage::ProjectLocator::default();
    ["projectId", "projectPath"]
        .iter()
        .filter_map(|key| payload.get(*key).and_then(Value::as_str))
        .find_map(|value| crate::authz::project_ref(&locator, value).ok())
        .map(|(project, _)| project)
}

/// Whether `principal` should be sent this event. Commands for the ingest
/// worker go only to the worker. An event about a Project goes only to its
/// members. Anything else is not about a Project and goes to everyone.
async fn may_receive(
    platform: &crate::principal::PlatformState,
    principal: &Principal,
    message: &str,
    known: &mut HashMap<llm_wiki_storage::ProjectId, bool>,
) -> bool {
    let Ok(parsed) = serde_json::from_str::<Value>(message) else {
        return false;
    };
    let name = parsed
        .get("event")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if name.starts_with("ingest-worker://") {
        return matches!(principal, Principal::Worker);
    }
    let Some(project) = parsed.get("payload").and_then(event_project) else {
        return true;
    };
    if let Some(allowed) = known.get(&project) {
        return *allowed;
    }
    let allowed = matches!(
        principal.role_in(&platform.projects, project).await,
        Ok(Some(_))
    );
    known.insert(project, allowed);
    allowed
}

pub async fn sse(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let platform = state.platform.clone();
    let principal = principal.map(|Extension(principal)| principal);
    // Who may see a Project can change while the tab stays open, so the
    // answers are kept for a short while only.
    let mut known: HashMap<llm_wiki_storage::ProjectId, bool> = HashMap::new();
    let mut checked_at = std::time::Instant::now();
    // A tab that falls more than CHANNEL_CAPACITY events behind skips the
    // gap rather than stalling everyone else.
    let stream = BroadcastStream::new(state.events.subscribe())
        .filter_map(|message| message.ok())
        .then(move |message| {
            let platform = platform.clone();
            let principal = principal.clone();
            if checked_at.elapsed() > Duration::from_secs(30) {
                known.clear();
                checked_at = std::time::Instant::now();
            }
            let mut snapshot = known.clone();
            async move {
                // Every stream belongs to a signed-in principal; one without
                // is sent nothing.
                let deliver = match &principal {
                    Some(principal) => {
                        may_receive(&platform, principal, message.as_ref(), &mut snapshot).await
                    }
                    None => false,
                };
                (deliver, message, snapshot)
            }
        })
        .map(|(deliver, message, _)| deliver.then_some(message))
        .filter_map(|message| message)
        .map(|message| Ok(Event::default().data(message.as_ref())));
    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}
