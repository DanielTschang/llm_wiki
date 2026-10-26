//! Who asked for a model call, and where to report it.
//!
//! The server tags the configuration of every call it makes for a person (an
//! agent turn, a search) with a [`UsageTag`]. When the call ends, the client
//! hands a [`LlmCall`] to the installed [`UsageSink`], which the server points
//! at its usage table. A configuration without a tag, such as one a script
//! builds, is not reported.

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Whom a call is charged to. Set by the server; anything a client sends in
/// its place is discarded.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UsageTag {
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    /// The model's id in the server's catalogue.
    pub model_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallKind {
    Chat,
    Embedding,
}

/// One finished call.
#[derive(Debug, Clone)]
pub struct LlmCall {
    pub tag: UsageTag,
    pub kind: CallKind,
    pub ok: bool,
    pub duration: Duration,
}

pub trait UsageSink: Send + Sync {
    fn record(&self, call: LlmCall);
}

static SINK: OnceLock<Arc<dyn UsageSink>> = OnceLock::new();

/// Installs the process-wide sink. Call once, at startup.
pub fn install_sink(sink: Arc<dyn UsageSink>) -> Result<(), String> {
    SINK.set(sink)
        .map_err(|_| "the usage sink is already installed".to_string())
}

pub(crate) fn report(tag: Option<&UsageTag>, kind: CallKind, ok: bool, duration: Duration) {
    if let (Some(tag), Some(sink)) = (tag, SINK.get()) {
        sink.record(LlmCall {
            tag: tag.clone(),
            kind,
            ok,
            duration,
        });
    }
}
