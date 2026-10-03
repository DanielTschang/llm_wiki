//! Confines every filesystem path a request names to the allowed roots.
//!
//! Desktop commands accept arbitrary absolute paths because the user is at
//! the keyboard. On a server the same commands would expose the whole
//! machine to anything that can call `/rpc` (including script injected
//! into rendered wiki content), so the dispatcher checks path arguments
//! here before any command runs.

use std::path::{Path, PathBuf};

use serde_json::Value;

/// camelCase argument names that carry a filesystem path.
const PATH_KEYS: &[&str] = &[
    "path",
    "projectPath",
    "filePath",
    "source",
    "destination",
    "sourcePath",
    "destDir",
    "relTo",
    "archivePath",
    "workingDirectory",
];
/// Argument holding a list of paths relative to `projectPath`.
const PATH_LIST_KEY: &str = "paths";
/// `app-state.json` keys that register project locations.
pub const PROJECT_SETTING_KEYS: &[&str] = &["projectRegistry", "recentProjects", "lastProject"];

pub struct AllowedRoots {
    roots: Vec<PathBuf>,
}

impl AllowedRoots {
    /// Every root must exist; they are stored canonicalized.
    pub fn new(roots: impl IntoIterator<Item = PathBuf>) -> Result<Self, String> {
        let roots = roots
            .into_iter()
            .map(|root| {
                root.canonicalize()
                    .map_err(|e| format!("Allowed root {} is not usable: {e}", root.display()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { roots })
    }

    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    /// Resolves `path` (following symlinks for the part that exists) and
    /// requires it to be inside an allowed root.
    pub fn check(&self, path: &Path) -> Result<PathBuf, String> {
        if !path.is_absolute() {
            return Err(format!("Path must be absolute: {}", path.display()));
        }
        let resolved = resolve(path)?;
        if self.roots.iter().any(|root| resolved.starts_with(root)) {
            Ok(resolved)
        } else {
            Err(format!(
                "Path is outside the server's allowed folders: {}",
                path.display()
            ))
        }
    }

    /// Checks the path-carrying arguments of an RPC call. Relative values
    /// are resolved against `projectPath`, as the commands themselves do.
    pub fn check_args(&self, args: &Value) -> Result<(), String> {
        let Some(object) = args.as_object() else {
            return Ok(());
        };
        let base = object.get("projectPath").and_then(Value::as_str);
        let check_one = |key: &str, value: &str| -> Result<(), String> {
            let path = Path::new(value);
            if path.is_absolute() {
                return self.check(path).map(drop);
            }
            match base {
                Some(base) if key != "projectPath" => {
                    self.check(&Path::new(base).join(path)).map(drop)
                }
                _ => Err(format!("`{key}` must be an absolute path: {value}")),
            }
        };
        for key in PATH_KEYS {
            if let Some(value) = object.get(*key).and_then(Value::as_str) {
                check_one(key, value)?;
            }
        }
        if let Some(list) = object.get(PATH_LIST_KEY).and_then(Value::as_array) {
            for value in list.iter().filter_map(Value::as_str) {
                check_one(PATH_LIST_KEY, value)?;
            }
        }
        Ok(())
    }

    /// Project registry settings decide which folders the agent and file
    /// watcher operate on, so they are held to the same roots.
    pub fn check_project_setting(&self, value: &Value) -> Result<(), String> {
        let mut paths = Vec::new();
        collect_path_fields(value, &mut paths);
        for path in paths {
            self.check(Path::new(path))?;
        }
        Ok(())
    }
}

fn collect_path_fields<'a>(value: &'a Value, out: &mut Vec<&'a str>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                match (key.as_str(), child) {
                    ("path", Value::String(path)) => out.push(path),
                    _ => collect_path_fields(child, out),
                }
            }
        }
        Value::Array(items) => items.iter().for_each(|item| collect_path_fields(item, out)),
        _ => {}
    }
}

/// Canonicalizes the longest existing prefix of `path` and re-appends the
/// not-yet-existing tail, so targets of create/write commands can be
/// checked too. A tail containing `..` is rejected rather than guessed at.
fn resolve(path: &Path) -> Result<PathBuf, String> {
    let mut existing = path.to_path_buf();
    let mut tail = Vec::new();
    loop {
        if let Ok(canonical) = existing.canonicalize() {
            let mut resolved = canonical;
            for name in tail.iter().rev() {
                resolved.push(name);
            }
            return Ok(resolved);
        }
        let name = existing
            .file_name()
            .ok_or_else(|| format!("Unsupported path: {}", path.display()))?
            .to_owned();
        tail.push(name);
        if !existing.pop() {
            return Err(format!("Unsupported path: {}", path.display()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn setup() -> (tempfile::TempDir, AllowedRoots, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("projects");
        std::fs::create_dir_all(root.join("wiki-a/raw")).unwrap();
        std::fs::create_dir_all(dir.path().join("secret")).unwrap();
        let allowed = AllowedRoots::new([root.clone()]).unwrap();
        (dir, allowed, root.canonicalize().unwrap())
    }

    #[test]
    fn allows_existing_and_new_paths_inside_roots() {
        let (_dir, allowed, root) = setup();
        assert!(allowed.check(&root.join("wiki-a/raw")).is_ok());
        assert!(allowed.check(&root.join("wiki-b/new/file.md")).is_ok());
    }

    #[test]
    fn rejects_escapes() {
        let (dir, allowed, root) = setup();
        assert!(allowed.check(&dir.path().join("secret")).is_err());
        assert!(allowed.check(&root.join("../secret")).is_err());
        assert!(allowed.check(&root.join("missing/../../secret/x")).is_err());
        assert!(allowed.check(Path::new("relative/path")).is_err());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(dir.path().join("secret"), root.join("link")).unwrap();
            assert!(allowed.check(&root.join("link/x")).is_err());
        }
    }

    #[test]
    fn checks_rpc_arguments_relative_to_project() {
        let (dir, allowed, root) = setup();
        let project = root.join("wiki-a");
        let project = project.to_str().unwrap();
        assert!(allowed
            .check_args(
                &json!({ "projectPath": project, "filePath": "wiki/a.md", "paths": ["raw/x.pdf"] })
            )
            .is_ok());
        assert!(allowed
            .check_args(&json!({ "projectPath": project, "filePath": "../../secret/a" }))
            .is_err());
        assert!(allowed
            .check_args(&json!({ "source": dir.path().join("secret").to_str().unwrap() }))
            .is_err());
        assert!(allowed.check_args(&json!({ "path": "wiki/a.md" })).is_err());
        assert!(allowed
            .check_args(&json!({ "query": "/etc/passwd" }))
            .is_ok());
    }

    #[test]
    fn checks_paths_in_project_settings() {
        let (dir, allowed, root) = setup();
        let inside = root.join("wiki-a");
        assert!(allowed
            .check_project_setting(&json!([{ "name": "a", "path": inside.to_str().unwrap() }]))
            .is_ok());
        assert!(allowed
            .check_project_setting(
                &json!({ "id1": { "path": dir.path().join("secret").to_str().unwrap() } })
            )
            .is_err());
    }
}
