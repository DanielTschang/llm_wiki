//! Everything that talks to a model provider: chat completions in each
//! provider's format with streaming, embeddings, and the HTTP client and
//! proxy settings both use. Knows nothing about Projects, pages or the agent.

pub mod embedding;
pub mod provider;
pub mod proxy;
pub mod usage;

pub use embedding::SearchEmbeddingConfig;
pub use usage::{install_sink, CallKind, LlmCall, UsageSink, UsageTag};

/// An image sent along with a chat message.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentImage {
    pub media_type: String,
    pub data_base64: String,
}
