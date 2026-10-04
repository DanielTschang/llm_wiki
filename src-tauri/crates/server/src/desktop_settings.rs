//! First-run import of model settings from the desktop app.
//!
//! The server keeps its own `app-state.json`, so on a machine that also
//! runs the desktop app the web edition would otherwise start with no model,
//! endpoint or API key. When the server's file has no model configured yet,
//! copy the model-related keys from the desktop app's file. Project
//! registrations and anything else server-specific are left alone, and a
//! server that already has a model is never touched.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::store::write_private_json;

/// Taken from the desktop file as a whole.
const MODEL_KEYS: &[&str] = &[
    "llmConfig",
    "activePresetId",
    "customLlmPresets",
    "taskModelRouting",
    "embeddingConfig",
    "multimodalConfig",
    "mineruConfig",
    "proxyConfig",
    "outputLanguage",
    "searchApiConfig",
];
/// Merged entry by entry, the desktop winning on conflicts.
const MERGED_MAP_KEYS: &[&str] = &["providerConfigs", "projectOutputLanguages"];

/// Where Tauri keeps the desktop app's settings (its `app_data_dir`).
pub fn desktop_app_state_path() -> Option<PathBuf> {
    const IDENTIFIER: &str = "com.llmwiki.app";
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let base = if cfg!(target_os = "macos") {
        home.map(|home| home.join("Library/Application Support"))
    } else if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| home.map(|home| home.join(".local/share")))
    };
    base.map(|base| base.join(IDENTIFIER).join("app-state.json"))
}

fn has_model_settings(state: &Map<String, Value>) -> bool {
    let preset = state
        .get("activePresetId")
        .and_then(Value::as_str)
        .is_some_and(|id| !id.is_empty());
    let model = state
        .get("llmConfig")
        .and_then(|config| config.get("model"))
        .and_then(Value::as_str)
        .is_some_and(|model| !model.is_empty());
    preset || model
}

fn read_map(path: &Path) -> Result<Map<String, Value>, String> {
    match std::fs::read_to_string(path) {
        Ok(raw) if raw.trim().is_empty() => Ok(Map::new()),
        Ok(raw) => {
            serde_json::from_str(&raw).map_err(|e| format!("{} is corrupt: {e}", path.display()))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Map::new()),
        Err(e) => Err(format!("Cannot read {}: {e}", path.display())),
    }
}

/// Seeds `server_state` from `desktop_state` when the server has no model
/// yet. Returns the imported keys, or `None` when nothing was done.
pub fn seed_from_desktop(
    server_state: &Path,
    desktop_state: &Path,
) -> Result<Option<Vec<String>>, String> {
    let mut server = read_map(server_state)?;
    if has_model_settings(&server) {
        return Ok(None);
    }
    let desktop = read_map(desktop_state)?;
    if !has_model_settings(&desktop) {
        return Ok(None);
    }
    let mut imported = Vec::new();
    for key in MODEL_KEYS {
        if let Some(value) = desktop.get(*key) {
            server.insert((*key).to_string(), value.clone());
            imported.push((*key).to_string());
        }
    }
    for key in MERGED_MAP_KEYS {
        let Some(Value::Object(from_desktop)) = desktop.get(*key) else {
            continue;
        };
        let mut merged = match server.remove(*key) {
            Some(Value::Object(existing)) => existing,
            _ => Map::new(),
        };
        merged.extend(from_desktop.clone());
        server.insert((*key).to_string(), Value::Object(merged));
        imported.push((*key).to_string());
    }
    write_private_json(server_state, &server)?;
    Ok(Some(imported))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn write(path: &Path, value: Value) {
        std::fs::write(path, serde_json::to_string(&value).unwrap()).unwrap();
    }

    fn desktop_state() -> Value {
        json!({
            "activePresetId": "deepseek",
            "llmConfig": { "provider": "custom", "model": "deepseek-v4-flash", "customEndpoint": "https://api.deepseek.com/v1" },
            "providerConfigs": { "deepseek": { "apiKey": "d" }, "custom": { "apiKey": "desktop" } },
            "embeddingConfig": { "enabled": true, "endpoint": "https://e" },
            "projectRegistry": { "desktop-project": { "path": "/elsewhere" } },
            "zoomLevel": 1.2
        })
    }

    #[test]
    fn imports_model_settings_into_an_unconfigured_server() {
        let dir = tempfile::tempdir().unwrap();
        let (server, desktop) = (
            dir.path().join("server.json"),
            dir.path().join("desktop.json"),
        );
        write(&desktop, desktop_state());
        write(
            &server,
            json!({
                "llmConfig": { "provider": "openai", "model": "" },
                "providerConfigs": { "anthropic": { "apiKey": "web-only" }, "custom": { "apiKey": "web" } },
                "projectRegistry": { "web-project": { "path": "/srv/wiki" } }
            }),
        );

        let imported = seed_from_desktop(&server, &desktop).unwrap().unwrap();

        let result: Value =
            serde_json::from_str(&std::fs::read_to_string(&server).unwrap()).unwrap();
        assert!(imported.contains(&"llmConfig".to_string()));
        assert_eq!(result["activePresetId"], "deepseek");
        assert_eq!(
            result["llmConfig"]["customEndpoint"],
            "https://api.deepseek.com/v1"
        );
        assert_eq!(result["embeddingConfig"]["endpoint"], "https://e");
        // Desktop wins per preset; web-only presets stay.
        assert_eq!(result["providerConfigs"]["custom"]["apiKey"], "desktop");
        assert_eq!(result["providerConfigs"]["anthropic"]["apiKey"], "web-only");
        // Server-specific and non-model keys are untouched.
        assert_eq!(
            result["projectRegistry"],
            json!({ "web-project": { "path": "/srv/wiki" } })
        );
        assert!(result.get("zoomLevel").is_none());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&server).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "settings hold API keys");
        }
    }

    #[test]
    fn creates_the_server_file_on_first_start() {
        let dir = tempfile::tempdir().unwrap();
        let (server, desktop) = (
            dir.path().join("server.json"),
            dir.path().join("desktop.json"),
        );
        write(&desktop, desktop_state());

        assert!(seed_from_desktop(&server, &desktop).unwrap().is_some());
        let result: Value =
            serde_json::from_str(&std::fs::read_to_string(&server).unwrap()).unwrap();
        assert_eq!(result["llmConfig"]["model"], "deepseek-v4-flash");
    }

    #[test]
    fn leaves_a_configured_server_alone() {
        let dir = tempfile::tempdir().unwrap();
        let (server, desktop) = (
            dir.path().join("server.json"),
            dir.path().join("desktop.json"),
        );
        write(&desktop, desktop_state());
        let configured = json!({ "llmConfig": { "provider": "anthropic", "model": "claude-x" } });
        write(&server, configured.clone());

        assert_eq!(seed_from_desktop(&server, &desktop).unwrap(), None);
        let result: Value =
            serde_json::from_str(&std::fs::read_to_string(&server).unwrap()).unwrap();
        assert_eq!(result, configured);
    }

    #[test]
    fn does_nothing_without_a_configured_desktop_app() {
        let dir = tempfile::tempdir().unwrap();
        let server = dir.path().join("server.json");
        assert_eq!(
            seed_from_desktop(&server, &dir.path().join("missing.json")).unwrap(),
            None
        );
        assert!(!server.exists());
    }
}
