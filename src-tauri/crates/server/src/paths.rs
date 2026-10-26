//! Confines every path a request names to a Project.
//!
//! Commands take paths as arguments, and a request must never reach the host's
//! disk, including through script injected into rendered wiki content. The
//! dispatcher checks every path argument here before any command runs.

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
/// Every path an RPC call names must be a Project path, `/projects/<uuid>/…`. Relative
/// values are taken as relative to `projectPath`. Nothing resolves to the
/// host disk, so there is no root list to escape from.
pub fn check_platform_args(
    locator: &llm_wiki_storage::ProjectLocator,
    args: &Value,
) -> Result<(), String> {
    let Some(object) = args.as_object() else {
        return Ok(());
    };
    let base = object.get("projectPath").and_then(Value::as_str);
    let check_one = |key: &str, value: &str| -> Result<(), String> {
        let joined;
        let path = if value.starts_with('/') {
            value
        } else {
            match base {
                Some(base) if key != "projectPath" => {
                    joined = format!("{}/{value}", base.trim_end_matches('/'));
                    joined.as_str()
                }
                _ => return Err(format!("`{key}` must be a project path: {value}")),
            }
        };
        locator.resolve(path).map(drop).map_err(|e| e.to_string())
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn platform_mode_accepts_only_project_paths() {
        let locator = llm_wiki_storage::ProjectLocator::default();
        let id = uuid::Uuid::new_v4();
        let ok = |args: serde_json::Value| check_platform_args(&locator, &args);

        assert!(ok(json!({ "path": format!("/projects/{id}/wiki/a.md") })).is_ok());
        assert!(
            ok(json!({ "projectPath": format!("/projects/{id}"), "paths": ["wiki/a.md"] })).is_ok()
        );
        assert!(ok(json!({ "source": format!("/projects/{id}/raw/a.pdf"), "destination": format!("/projects/{id}/raw/sources/a.pdf") })).is_ok());
        assert!(ok(json!({ "query": "no paths here" })).is_ok());

        for bad in [
            json!({ "path": "/etc/passwd" }),
            json!({ "path": "/projects/not-a-uuid/x" }),
            json!({ "path": format!("/projects/{id}/../other") }),
            json!({ "path": "relative.md" }),
            json!({ "projectPath": "/home/me/wiki" }),
            json!({ "projectPath": format!("/projects/{id}"), "paths": ["../escape.md"] }),
            json!({ "path": format!("/projects/{id}/wiki/a.md"), "destination": "/tmp/x" }),
        ] {
            assert!(ok(bad.clone()).is_err(), "{bad}");
        }
    }
}
