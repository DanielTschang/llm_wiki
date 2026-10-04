//! Server-side replacement for Tauri's plugin-store: flat JSON objects in
//! the data dir, in the same file format, so core code that reads
//! `app-state.json` directly sees the frontend's settings.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde_json::{Map, Value};

pub struct AppStore {
    dir: PathBuf,
    // Serializes read-modify-write cycles across concurrent requests.
    lock: Mutex<()>,
}

impl AppStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            lock: Mutex::new(()),
        }
    }

    pub fn get(&self, store: &str, key: &str) -> Result<Value, String> {
        let _guard = self.lock.lock().map_err(|_| "store lock poisoned")?;
        Ok(read(&self.path(store)?)?.remove(key).unwrap_or(Value::Null))
    }

    pub fn set(&self, store: &str, key: &str, value: Value) -> Result<(), String> {
        let _guard = self.lock.lock().map_err(|_| "store lock poisoned")?;
        let path = self.path(store)?;
        let mut map = read(&path)?;
        map.insert(key.to_string(), value);
        write(&path, &map)
    }

    pub fn delete(&self, store: &str, key: &str) -> Result<bool, String> {
        let _guard = self.lock.lock().map_err(|_| "store lock poisoned")?;
        let path = self.path(store)?;
        let mut map = read(&path)?;
        let existed = map.remove(key).is_some();
        if existed {
            write(&path, &map)?;
        }
        Ok(existed)
    }

    fn path(&self, store: &str) -> Result<PathBuf, String> {
        let valid = store.ends_with(".json")
            && !store.starts_with('.')
            && store
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'));
        if valid {
            Ok(self.dir.join(store))
        } else {
            Err(format!("Invalid store name: {store}"))
        }
    }
}

fn read(path: &Path) -> Result<Map<String, Value>, String> {
    match fs::read_to_string(path) {
        Ok(raw) if raw.trim().is_empty() => Ok(Map::new()),
        Ok(raw) => {
            serde_json::from_str(&raw).map_err(|e| format!("{} is corrupt: {e}", path.display()))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Map::new()),
        Err(e) => Err(format!("Failed to read {}: {e}", path.display())),
    }
}

fn write(path: &Path, map: &Map<String, Value>) -> Result<(), String> {
    write_private_json(path, map)
}

/// Atomically replaces `path` with `map` as pretty JSON, readable only by
/// the owner: these files hold API keys.
pub fn write_private_json(path: &Path, map: &Map<String, Value>) -> Result<(), String> {
    let raw = serde_json::to_string_pretty(map).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&tmp)
        .map_err(|e| format!("Failed to write {}: {e}", tmp.display()))?;
    std::io::Write::write_all(&mut file, raw.as_bytes())
        .map_err(|e| format!("Failed to write {}: {e}", tmp.display()))?;
    drop(file);
    fs::rename(&tmp, path).map_err(|e| format!("Failed to replace {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn round_trips_keys_in_plugin_store_format() {
        let dir = tempfile::tempdir().unwrap();
        let store = AppStore::new(dir.path());

        assert_eq!(
            store.get("app-state.json", "lastProject").unwrap(),
            Value::Null
        );
        store
            .set("app-state.json", "lastProject", json!({ "path": "/p" }))
            .unwrap();
        store.set("app-state.json", "theme", json!("dark")).unwrap();
        assert_eq!(store.get("app-state.json", "theme").unwrap(), json!("dark"));

        let on_disk: Value =
            serde_json::from_str(&fs::read_to_string(dir.path().join("app-state.json")).unwrap())
                .unwrap();
        assert_eq!(
            on_disk,
            json!({ "lastProject": { "path": "/p" }, "theme": "dark" })
        );

        assert!(store.delete("app-state.json", "theme").unwrap());
        assert!(!store.delete("app-state.json", "theme").unwrap());
    }

    #[test]
    fn rejects_store_names_that_leave_the_data_dir() {
        let store = AppStore::new("/data");
        for name in ["../x.json", "a/b.json", ".hidden.json", "server-token", ""] {
            assert!(store.get(name, "k").is_err(), "{name}");
        }
    }
}
