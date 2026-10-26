//! The agent: routing, retrieval, tool execution, context assembly, sessions.
//!
//! Keep routing, retrieval, tool execution, context assembly, sessions, and
//! cancellation in this Rust module. The React/TypeScript side may render UI
//! state and bridge provider-specific transports, but it should not reimplement
//! the Agent core; otherwise API/MCP/UI behavior will drift.

pub mod cancel;
pub mod commands;
pub mod context;
pub mod events;
pub mod permissions;
pub mod router;
pub mod runtime;
pub mod session;
pub mod skills;
pub mod tools;
pub mod types;
pub mod workspace;

pub use runtime::AgentRuntime;
pub use types::AgentChatRequest;
