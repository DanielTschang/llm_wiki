mod commands;
mod tray;

use std::sync::{Arc, Mutex};

use llm_wiki_core::panic_guard::run_guarded;
use llm_wiki_core::{api_server, clip_server, proxy, CoreContext, EventEmitter, EventSink};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_opener::OpenerExt;

/// Forwards core events to the webview as Tauri events.
struct TauriEventSink(AppHandle);

impl EventSink for TauriEventSink {
    fn emit_value(&self, event: &str, payload: serde_json::Value) -> Result<(), String> {
        self.0.emit(event, payload).map_err(|e| e.to_string())
    }
}

struct CloseBehaviorState(Mutex<String>);
struct TrayAvailabilityState(Mutex<bool>);

/// Opens `path` with the OS default handler, falling back to revealing
/// it in the file manager.
fn open_with_os(app: &AppHandle, path: String, what: &str) -> Result<(), String> {
    match app.opener().open_path(path.clone(), None::<&str>) {
        Ok(()) => Ok(()),
        Err(open_err) => app.opener().reveal_item_in_dir(path).map_err(|reveal_err| {
            format!(
                "Failed to open {what}: {}; reveal fallback also failed: {}",
                open_err, reveal_err
            )
        }),
    }
}

#[tauri::command]
fn open_project_folder(app: AppHandle, path: String) -> Result<(), String> {
    run_guarded("open_project_folder", || {
        let canonical = llm_wiki_core::commands::project::resolve_project_folder(&path)?;
        open_with_os(&app, canonical, "project folder")
    })
}

#[tauri::command]
fn open_path_in_project(
    app: AppHandle,
    project_path: String,
    target_path: String,
) -> Result<(), String> {
    run_guarded("open_path_in_project", || {
        let target =
            llm_wiki_core::commands::project::resolve_path_in_project(&project_path, &target_path)?;
        open_with_os(&app, target, "project path")
    })
}

#[tauri::command]
fn mcp_server_entry_path(app: tauri::AppHandle) -> Result<String, String> {
    run_guarded("mcp_server_entry_path", || {
        let relative = std::path::Path::new("mcp-server")
            .join("dist")
            .join("src")
            .join("index.js");
        let mut candidates = Vec::new();

        let mut push_repo_candidates = |base: std::path::PathBuf| {
            candidates.push(base.join(&relative));
            candidates.push(base.join("..").join(&relative));
            candidates.push(base.join("..").join("..").join(&relative));
        };

        push_repo_candidates(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        if let Ok(cwd) = std::env::current_dir() {
            push_repo_candidates(cwd);
        }
        if let Ok(resource_dir) = app.path().resource_dir() {
            candidates.push(resource_dir.join(&relative));
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(exe_dir) = exe.parent() {
                candidates.push(exe_dir.join(&relative));
                candidates.push(exe_dir.join("..").join("Resources").join(&relative));
            }
        }

        for candidate in &candidates {
            if candidate.is_file() {
                return Ok(candidate
                    .canonicalize()
                    .unwrap_or_else(|_| candidate.clone())
                    .to_string_lossy()
                    .into_owned());
            }
        }

        Err("MCP server entry was not found. Run `npm run mcp:build` from the LLM Wiki repository, then reopen Settings.".to_string())
    })
}

#[tauri::command]
fn set_close_behavior(
    value: String,
    state: tauri::State<'_, CloseBehaviorState>,
) -> Result<String, String> {
    let normalized = match value.as_str() {
        "ask" | "minimize" | "exit" => value,
        other => return Err(format!("Invalid close behavior: {other}")),
    };
    let mut guard = state
        .0
        .lock()
        .map_err(|_| "Close behavior state is unavailable".to_string())?;
    *guard = normalized.clone();
    Ok(normalized)
}

fn close_behavior<R: tauri::Runtime>(window: &tauri::Window<R>) -> String {
    window
        .state::<CloseBehaviorState>()
        .0
        .lock()
        .map(|value| value.clone())
        .unwrap_or_else(|_| "minimize".to_string())
}

fn tray_available<R: tauri::Runtime>(window: &tauri::Window<R>) -> bool {
    window
        .state::<TrayAvailabilityState>()
        .0
        .lock()
        .map(|value| *value)
        .unwrap_or(false)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    apply_linux_webkit_compat_env();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None::<Vec<&str>>,
        ))
        // Rust-backed fetch so third-party LLM APIs that reject
        // browser-origin headers via CORS preflight (MiniMax, Volcengine
        // Ark's api/coding/v3, etc.) still work. Requests leave the app
        // from Rust, never the webview.
        .plugin(tauri_plugin_http::init())
        .setup(|app| {
            // Let the PDF extractor find the bundled pdfium dynamic
            // library via Tauri's platform-correct resource path.
            if let Ok(dir) = app.path().resource_dir() {
                llm_wiki_core::commands::fs::set_resource_dir_hint(dir);
            }
            // Apply user-configured global HTTP proxy by setting
            // HTTP_PROXY / HTTPS_PROXY / NO_PROXY env vars BEFORE
            // any HTTP request is made. tauri-plugin-http's reqwest
            // client reads these on first construction. Lives next
            // to the resource-dir hint so the proxy applies to
            // everything: LLM, embedding, update check, deep
            // research, captioning. See src-tauri/crates/core/src/proxy.rs.
            if let Ok(dir) = app.path().app_data_dir() {
                let store_path = dir.join("app-state.json");
                eprintln!("[proxy] reading from {}", store_path.display());
                if let Some(cfg) = proxy::read_proxy_config_from_store(&store_path) {
                    let summary = proxy::apply_proxy_env(&cfg);
                    eprintln!("[proxy] {summary}");
                } else {
                    eprintln!("[proxy] no proxyConfig in store, requests go direct");
                }
            } else {
                eprintln!("[proxy] could not resolve app_data_dir");
            }
            // Keep core background work (API server threads) on Tauri's
            // runtime instead of a second one.
            llm_wiki_core::runtime::install(tauri::async_runtime::handle().inner().clone());
            let core = Arc::new(CoreContext::new(
                app.path().app_data_dir().ok(),
                env!("CARGO_PKG_VERSION"),
                EventEmitter::new(TauriEventSink(app.handle().clone())),
            ));
            app.manage(Arc::clone(&core));
            app.manage(CloseBehaviorState(Mutex::new("minimize".to_string())));
            app.manage(TrayAvailabilityState(Mutex::new(false)));
            // Start the API before optional desktop integrations so the
            // backend is reachable if tray setup or another integration fails.
            clip_server::start_clip_server(Arc::clone(&core));
            api_server::start_api_server(core);
            let tray_available = match tray::create_tray(app.handle()) {
                Ok(()) => true,
                Err(err) => {
                    eprintln!("[tray] system tray unavailable, continuing without it: {err}");
                    false
                }
            };
            match app.state::<TrayAvailabilityState>().0.lock() {
                Ok(mut state) => {
                    *state = tray_available;
                }
                Err(err) => {
                    eprintln!("[tray] failed to update tray availability state: {err}");
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::fs::read_file,
            commands::fs::write_file,
            commands::fs::write_file_base64,
            commands::fs::write_file_atomic,
            commands::fs::apply_text_selection_edit,
            commands::fs::create_missing_wiki_page,
            commands::fs::list_directory,
            commands::fs::copy_file,
            commands::fs::copy_directory,
            commands::fs::preprocess_file,
            commands::fs::delete_file,
            commands::fs::find_related_wiki_pages,
            commands::fs::create_directory,
            commands::fs::file_exists,
            commands::fs::get_file_modified_time,
            commands::fs::get_file_size,
            commands::fs::get_file_md5,
            commands::fs::read_file_as_base64,
            commands::file_history::list_file_history,
            commands::file_history::restore_file_history,
            commands::file_history::get_file_history_stats,
            commands::file_history::get_file_history_settings,
            commands::file_history::set_file_history_settings,
            commands::file_history::clear_file_history,
            commands::project::create_project,
            commands::project::open_project,
            commands::project_maintenance::export_project_archive,
            commands::project_maintenance::import_project_archive,
            commands::project_maintenance::rebuild_wiki_index,
            commands::search::search_project,
            commands::search::embedding_fetch,
            commands::search::embedding_fetch_batch,
            commands::search::get_page_links,
            commands::external_search::web_search,
            commands::external_search::anytxt_search,
            commands::app::clip_server_status,
            commands::app::api_server_status,
            commands::app::api_server_reload_config,
            commands::app::set_proxy_env,
            commands::agent_commands::agent_start_turn,
            commands::agent_commands::agent_start_turn_stream,
            commands::agent_commands::agent_cancel_turn,
            commands::agent_commands::agent_get_session,
            commands::agent_commands::agent_list_sessions,
            commands::agent_skills::agent_list_skills,
            commands::vectorstore::vector_upsert,
            commands::vectorstore::vector_search,
            commands::vectorstore::vector_delete,
            commands::vectorstore::vector_count,
            commands::vectorstore::vector_upsert_chunks,
            commands::vectorstore::vector_search_chunks,
            commands::vectorstore::vector_delete_page,
            commands::vectorstore::vector_count_chunks,
            commands::vectorstore::vector_clear_chunks,
            commands::vectorstore::vector_optimize_chunks,
            commands::vectorstore::vector_legacy_row_count,
            commands::vectorstore::vector_drop_legacy,
            commands::claude_cli::claude_cli_detect,
            commands::claude_cli::claude_cli_spawn,
            commands::claude_cli::claude_cli_kill,
            commands::codex_cli::codex_cli_detect,
            commands::codex_cli::codex_cli_spawn,
            commands::codex_cli::codex_cli_kill,
            commands::extract_images::extract_pdf_images_cmd,
            commands::extract_images::extract_office_images_cmd,
            commands::extract_images::extract_and_save_pdf_images_cmd,
            commands::extract_images::extract_and_save_office_images_cmd,
            commands::file_sync::start_project_file_watcher,
            commands::file_sync::stop_project_file_watcher,
            commands::file_sync::rescan_project_files,
            commands::file_sync::invalidate_project_file_snapshot_paths,
            commands::file_sync::get_file_change_queue,
            commands::file_sync::retry_file_change_task,
            commands::file_sync::ignore_file_change_task,
            open_project_folder,
            open_path_in_project,
            mcp_server_entry_path,
            set_close_behavior,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let behavior = close_behavior(window);
                let win = window.clone();
                let app = window.app_handle().clone();
                match behavior.as_str() {
                    "exit" => {
                        tauri::async_runtime::spawn(async move {
                            let _ = win.destroy();
                            app.exit(0);
                        });
                    }
                    "minimize" => {
                        if tray_available(window) {
                            let _ = window.hide();
                        } else {
                            let _ = window.minimize();
                        }
                    }
                    _ => {
                        tauri::async_runtime::spawn(async move {
                            use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
                            let confirmed = app
                                .dialog()
                                .message(
                                    "Quit LLM Wiki? Choose Quit to exit. Choose Hide Window to keep background features running.",
                                )
                                .title("LLM Wiki")
                                .buttons(MessageDialogButtons::OkCancelCustom(
                                    "Quit".to_string(),
                                    "Hide Window".to_string(),
                                ))
                                .kind(tauri_plugin_dialog::MessageDialogKind::Warning)
                                .blocking_show();

                            if confirmed {
                                let _ = win.destroy();
                                app.exit(0);
                            } else {
                                let _ = win.hide();
                            }
                        });
                    }
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen {
                has_visible_windows,
                ..
            } = event
            {
                if !has_visible_windows {
                    use tauri::Manager;
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
            let _ = (app, event); // suppress unused warnings on non-macOS
        });
}

#[cfg(target_os = "linux")]
fn apply_linux_webkit_compat_env() {
    // WebKitGTK can crash or withdraw its window on some Wayland/XWayland
    // stacks unless accelerated render paths are disabled before the WebView
    // is created. Keep these as Linux-only defaults and do not override an
    // explicit user setting so advanced users and packagers can opt back into
    // the platform default if their stack supports it.
    if std::env::var_os("WEBKIT_DISABLE_COMPOSITING_MODE").is_none() {
        std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
    }
    // Some WebKitGTK/Mesa combinations still attempt the DMA-BUF renderer
    // even with accelerated compositing disabled. In an AppImage running
    // through XWayland that can withdraw the native window while leaving the
    // web and network processes alive. Respect explicit packager overrides.
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
}

#[cfg(not(target_os = "linux"))]
fn apply_linux_webkit_compat_env() {}
