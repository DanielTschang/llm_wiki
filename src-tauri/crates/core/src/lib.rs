//! LLM Wiki backend, independent of any UI shell.
//!
//! The desktop app (`src-tauri`) wraps these functions as Tauri commands;
//! the self-hosted server (plans/web-server-mode.md, Phase 3) exposes the
//! same functions over HTTP. Nothing in this crate may depend on `tauri`.

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
