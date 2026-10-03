//! App-level status and settings commands.

use crate::panic_guard::run_guarded;
use crate::{api_server, clip_server, proxy};

pub fn clip_server_status() -> String {
    run_guarded("clip_server_status", || {
        Ok(clip_server::get_daemon_status().to_string())
    })
    .unwrap_or_else(|e| format!("error: {e}"))
}

pub fn api_server_status() -> String {
    run_guarded("api_server_status", || {
        Ok(api_server::get_api_status().to_string())
    })
    .unwrap_or_else(|e| format!("error: {e}"))
}

pub fn api_server_reload_config() -> String {
    run_guarded("api_server_reload_config", || {
        api_server::invalidate_config_cache();
        Ok("ok".to_string())
    })
    .unwrap_or_else(|e| format!("error: {e}"))
}

/// Apply a proxy configuration to the process env immediately, so the
/// next outbound HTTP request picks it up without needing the user to
/// restart the app. tauri-plugin-http builds a fresh
/// `reqwest::ClientBuilder` per fetch and reqwest's `auto_sys_proxy`
/// re-reads HTTP_PROXY / HTTPS_PROXY / NO_PROXY each time, so updating
/// these env vars is sufficient to flip the proxy on/off live.
///
/// Returns the same human-readable summary `apply_proxy_env` produces
/// for logging.
pub fn set_proxy_env(config: proxy::ProxyConfig) -> String {
    let summary = proxy::apply_proxy_env(&config);
    eprintln!("[proxy] live update: {summary}");
    summary
}
