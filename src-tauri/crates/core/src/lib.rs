//! LLM Wiki backend, independent of any UI shell.
//!
//! The server (`llm-wiki-server`) exposes these functions over HTTP;
//! nothing in this crate may depend on a UI shell.

pub mod agent;
pub mod api_server;
pub mod clip_server;
pub mod commands;
pub mod context;
pub mod cors;
pub mod panic_guard;
pub mod proxy;
pub mod runtime;
pub mod server_bind;
pub mod types;

pub use context::{CoreContext, EventEmitter, EventSink};
