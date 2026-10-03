//! Tokio runtime access for code that runs outside an async context
//! (the `tiny_http` API/clip server threads).
//!
//! The host installs its own runtime handle at startup so the desktop app
//! keeps a single runtime (Tauri's). Without one — the headless server or
//! tests — a multi-threaded runtime is created on first use.

use std::future::Future;
use std::sync::OnceLock;

use tokio::runtime::{Builder, Handle, Runtime};
use tokio::task::JoinHandle;

static INSTALLED: OnceLock<Handle> = OnceLock::new();
static FALLBACK: OnceLock<Runtime> = OnceLock::new();

/// Use `handle` for all core background work. Only the first call wins.
pub fn install(handle: Handle) {
    let _ = INSTALLED.set(handle);
}

pub fn handle() -> Handle {
    if let Some(handle) = INSTALLED.get() {
        return handle.clone();
    }
    FALLBACK
        .get_or_init(|| {
            Builder::new_multi_thread()
                .enable_all()
                .thread_name("llm-wiki-core")
                .build()
                .expect("failed to build llm-wiki-core tokio runtime")
        })
        .handle()
        .clone()
}

/// Runs `future` to completion. Must not be called from inside an async task.
pub fn block_on<F: Future>(future: F) -> F::Output {
    handle().block_on(future)
}

pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    handle().spawn(future)
}

#[cfg(test)]
mod tests {
    #[test]
    fn falls_back_to_an_owned_runtime_without_a_host() {
        let joined = super::block_on(super::spawn(async { 40 + 2 }));
        assert_eq!(joined.unwrap(), 42);
    }
}
