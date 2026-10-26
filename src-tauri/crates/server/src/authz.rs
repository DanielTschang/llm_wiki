//! What a signed-in user may do in a Project.
//!
//! Every platform request that names a Project is checked here before it runs:
//! the Project is found from the request's path or id arguments, and the
//! caller's role in it must be at least what the command needs. Roles apply to
//! the whole Project, never to individual pages.

use std::collections::HashMap;

use axum::http::StatusCode;
use llm_wiki_storage::{ProjectId, ProjectLocator, RelPath, Role};
use serde_json::Value;

use crate::principal::{PlatformState, Principal};

#[derive(Debug, PartialEq, Eq)]
pub struct Denied {
    pub status: StatusCode,
    pub message: String,
}

impl Denied {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, message)
    }
}

/// What a command does with the Project an argument names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Access {
    Read,
    Write,
    /// Removing pages needs an Owner; removing anything else, an Editor.
    Delete,
}

const AGENT_TURNS: &[&str] = &["agent_start_turn", "agent_start_turn_stream"];

/// The arguments of `command` that name a Project's files, and how each is
/// used. `None` for a command that touches no Project.
fn arguments(command: &str) -> Option<&'static [(&'static str, Access)]> {
    use Access::*;
    Some(match command {
        "read_file"
        | "list_directory"
        | "file_exists"
        | "get_file_modified_time"
        | "get_file_size"
        | "get_file_md5"
        | "read_file_as_base64" => &[("path", Read)],
        "preprocess_file" | "write_file" | "write_file_base64" | "write_file_atomic"
        | "create_directory" => &[("path", Write)],
        "delete_file" => &[("path", Delete)],
        "copy_file" | "copy_directory" => &[("source", Read), ("destination", Write)],
        "search_project" | "agent_list_skills" => &[("projectPath", Read)],
        "get_page_links" => &[("projectPath", Read), ("filePath", Read)],
        "agent_start_turn"
        | "agent_start_turn_stream"
        | "agent_cancel_turn"
        | "agent_get_session"
        | "agent_list_sessions" => &[("projectId", Read)],
        _ => return None,
    })
}

/// Commands with a global effect, which only a Platform Admin may run.
fn admin_only(command: &str) -> bool {
    matches!(command, "set_proxy_env" | "api_server_reload_config")
}

/// A Project named by a `/projects/<id>/…` path or by a bare id.
pub fn project_ref(locator: &ProjectLocator, value: &str) -> Result<(ProjectId, RelPath), String> {
    if value.starts_with('/') {
        return locator.resolve(value).map_err(|e| e.to_string());
    }
    let id = uuid::Uuid::parse_str(value.trim())
        .map_err(|_| format!("not a project id or path: {value}"))?;
    Ok((ProjectId(id), RelPath::root()))
}

fn needs(access: Access, rel: &RelPath) -> Role {
    match access {
        Access::Read => Role::Viewer,
        Access::Write => Role::Editor,
        Access::Delete if rel.is_root() || rel.top_level() == "wiki" => Role::Owner,
        Access::Delete => Role::Editor,
    }
}

/// Checks an RPC call against the caller's roles. For an agent turn by
/// someone who may not edit, sets `readOnly` so the agent cannot write.
pub async fn authorize_rpc(
    platform: &PlatformState,
    principal: &Principal,
    command: &str,
    args: &mut Value,
) -> Result<(), Denied> {
    if admin_only(command) && !principal.is_platform_admin() {
        return Err(Denied::forbidden("only a Platform Admin can do that"));
    }
    let Some(rules) = arguments(command) else {
        return Ok(());
    };
    let mut required: HashMap<ProjectId, Role> = HashMap::new();
    for (key, access) in rules {
        let Some(value) = args.get(*key).and_then(Value::as_str) else {
            continue;
        };
        let (project, rel) = project_ref(&llm_wiki_storage::ProjectLocator::default(), value)
            .map_err(|message| Denied::new(StatusCode::BAD_REQUEST, message))?;
        let role = needs(*access, &rel);
        let entry = required.entry(project).or_insert(role);
        *entry = (*entry).max(role);
    }
    let mut weakest: Option<Role> = None;
    for (project, need) in required {
        let role = principal
            .role_in(&platform.projects, project)
            .await
            .map_err(|e| Denied::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        match role {
            // The same answer whether the Project exists or not, so a user
            // cannot probe for Projects they do not belong to.
            None => return Err(Denied::forbidden("you do not have access to this project")),
            Some(role) if role < need => {
                return Err(Denied::forbidden(format!(
                    "this needs the {} role in the project; you are a {}",
                    need.as_str(),
                    role.as_str()
                )))
            }
            Some(role) => weakest = Some(weakest.map_or(role, |w| w.min(role))),
        }
    }
    if AGENT_TURNS.contains(&command) && weakest.is_some_and(|role| role < Role::Editor) {
        if let Some(object) = args.as_object_mut() {
            object.insert("readOnly".into(), Value::Bool(true));
        }
    }
    Ok(())
}

/// The role needed to read a file over `GET /files`, and to stage an upload.
pub async fn require_role(
    platform: &PlatformState,
    principal: &Principal,
    project: ProjectId,
    need: Role,
) -> Result<(), Denied> {
    let role = principal
        .role_in(&platform.projects, project)
        .await
        .map_err(|e| Denied::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    match role {
        Some(role) if role >= need => Ok(()),
        _ => Err(Denied::forbidden("you do not have access to this project")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rel(path: &str) -> RelPath {
        RelPath::parse(path).unwrap()
    }

    #[test]
    fn commands_map_to_the_access_they_need() {
        assert_eq!(arguments("read_file"), Some(&[("path", Access::Read)][..]));
        assert_eq!(
            arguments("write_file"),
            Some(&[("path", Access::Write)][..])
        );
        assert_eq!(
            arguments("delete_file"),
            Some(&[("path", Access::Delete)][..])
        );
        assert_eq!(
            arguments("copy_file"),
            Some(&[("source", Access::Read), ("destination", Access::Write)][..])
        );
        // Not a Project command at all.
        assert_eq!(arguments("web_search"), None);
        assert_eq!(arguments("an_unknown_command"), None);
        assert!(admin_only("set_proxy_env") && !admin_only("read_file"));
    }

    #[test]
    fn every_command_that_changes_files_needs_an_editor() {
        for command in [
            "write_file",
            "write_file_base64",
            "write_file_atomic",
            "create_directory",
            "preprocess_file",
        ] {
            let rules = arguments(command).unwrap();
            assert!(rules.iter().all(|(_, a)| *a == Access::Write), "{command}");
        }
        for command in [
            "read_file",
            "list_directory",
            "file_exists",
            "search_project",
            "get_page_links",
            "agent_start_turn",
        ] {
            let rules = arguments(command).unwrap();
            assert!(rules.iter().all(|(_, a)| *a == Access::Read), "{command}");
        }
    }

    #[test]
    fn deleting_pages_needs_an_owner_but_deleting_sources_an_editor() {
        assert_eq!(needs(Access::Delete, &rel("wiki/a.md")), Role::Owner);
        assert_eq!(needs(Access::Delete, &rel("wiki")), Role::Owner);
        assert_eq!(needs(Access::Delete, &RelPath::root()), Role::Owner);
        assert_eq!(
            needs(Access::Delete, &rel("raw/sources/a.pdf")),
            Role::Editor
        );
        assert_eq!(needs(Access::Read, &rel("wiki/a.md")), Role::Viewer);
        assert_eq!(needs(Access::Write, &rel("wiki/a.md")), Role::Editor);
    }

    #[test]
    fn a_project_is_named_by_path_or_by_id() {
        let locator = ProjectLocator::default();
        let id = uuid::Uuid::new_v4();
        let by_path = project_ref(&locator, &format!("/projects/{id}/wiki/a.md")).unwrap();
        assert_eq!(by_path, (ProjectId(id), rel("wiki/a.md")));
        let by_id = project_ref(&locator, &id.to_string()).unwrap();
        assert_eq!(by_id, (ProjectId(id), RelPath::root()));
        for bad in ["", "/etc/passwd", "not-a-uuid", "/projects/x/y"] {
            assert!(project_ref(&locator, bad).is_err(), "{bad:?}");
        }
    }
}
