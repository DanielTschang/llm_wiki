//! Core events → Server-Sent Events. Every connected browser tab receives
//! every event as `{ "event": name, "payload": … }`, which is what the
//! web platform's `listen()` expects.

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::Stream;
use llm_wiki_core::EventSink;
use serde_json::{json, Value};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

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

pub async fn sse(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // A tab that falls more than CHANNEL_CAPACITY events behind skips the
    // gap rather than stalling everyone else.
    let stream = BroadcastStream::new(state.events.subscribe())
        .filter_map(|message| message.ok())
        .map(|message| Ok(Event::default().data(message.as_ref())));
    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}
