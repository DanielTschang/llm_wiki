//! The commands that manage accounts, Projects and who belongs to them. They
//! exist only on a platform server and are answered here rather than by the
//! shared backend.

use axum::http::StatusCode;
use llm_wiki_storage::{
    Member, NewUser, ProjectId, ProjectInfo, ProjectLocator, Role, StoreError, User,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::authz::{project_ref, Denied};
use crate::principal::{PlatformState, Principal};

type Reply = Result<Value, Denied>;

fn status_of(error: &StoreError) -> StatusCode {
    match error {
        StoreError::NotFound(_) => StatusCode::NOT_FOUND,
        StoreError::Forbidden(_) => StatusCode::FORBIDDEN,
        StoreError::AlreadyExists(_) | StoreError::VersionConflict { .. } => StatusCode::CONFLICT,
        StoreError::Invalid(_) | StoreError::InvalidPath { .. } => StatusCode::BAD_REQUEST,
        StoreError::Backend(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn denied(error: StoreError) -> Denied {
    if matches!(error, StoreError::Backend(_)) {
        eprintln!("[accounts] {error}");
    }
    Denied::new(status_of(&error), error.to_string())
}

fn bad_request(message: impl Into<String>) -> Denied {
    Denied::new(StatusCode::BAD_REQUEST, message)
}

fn text<'a>(args: &'a Value, key: &str) -> Result<&'a str, Denied> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| bad_request(format!("`{key}` is required")))
}

fn optional_text<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(Value::as_str)
}

fn flag(args: &Value, key: &str) -> Result<bool, Denied> {
    args.get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| bad_request(format!("`{key}` must be true or false")))
}

fn uuid_arg(args: &Value, key: &str) -> Result<Uuid, Denied> {
    Uuid::parse_str(text(args, key)?).map_err(|_| bad_request(format!("`{key}` is not an id")))
}

/// The Project a request names, from `projectId` (an id or a project path).
fn project_arg(args: &Value) -> Result<ProjectId, Denied> {
    let (project, _) =
        project_ref(&ProjectLocator::default(), text(args, "projectId")?).map_err(bad_request)?;
    Ok(project)
}

pub fn project_json(info: &ProjectInfo) -> Value {
    json!({
        "id": info.id.0.to_string(),
        "name": info.name,
        "path": ProjectLocator::default().project_path(info.id),
        "role": info.role.map(Role::as_str),
        "memberCount": info.member_count,
        "createdAt": info.created_at.to_rfc3339(),
    })
}

fn user_json(user: &User) -> Value {
    json!({
        "id": user.id.to_string(),
        "username": user.username,
        "displayName": user.display_name,
        "email": user.email,
        "isPlatformAdmin": user.is_platform_admin,
        "disabled": user.disabled,
        "isService": user.is_service,
    })
}

fn member_json(member: &Member) -> Value {
    json!({
        "userId": member.user_id.to_string(),
        "username": member.username,
        "displayName": member.display_name,
        "role": member.role.as_str(),
        "addedAt": member.added_at.to_rfc3339(),
    })
}

fn require_admin(principal: &Principal) -> Result<(), Denied> {
    if principal.is_platform_admin() {
        Ok(())
    } else {
        Err(Denied::forbidden("only a Platform Admin can do that"))
    }
}

fn actor(principal: &Principal) -> Result<&User, Denied> {
    principal
        .actor()
        .ok_or_else(|| Denied::forbidden("this needs an account"))
}

/// A user may manage a Project's members if they own it; a Platform Admin
/// always may, so a Project whose owners are gone can still be recovered. The
/// change is recorded under the admin's name either way.
async fn require_manager(
    platform: &PlatformState,
    principal: &Principal,
    project: ProjectId,
) -> Result<(), Denied> {
    if principal.is_platform_admin() {
        return Ok(());
    }
    match principal
        .role_in(&platform.projects, project)
        .await
        .map_err(denied)?
    {
        Some(Role::Owner) => Ok(()),
        Some(_) => Err(Denied::forbidden("only an owner can do that")),
        None => Err(Denied::forbidden("you do not have access to this project")),
    }
}

async fn require_member(
    platform: &PlatformState,
    principal: &Principal,
    project: ProjectId,
    at_least: Role,
) -> Result<Role, Denied> {
    match principal
        .role_in(&platform.projects, project)
        .await
        .map_err(denied)?
    {
        Some(role) if role >= at_least => Ok(role),
        _ => Err(Denied::forbidden("you do not have access to this project")),
    }
}

/// Answers an account or Project management command. `None` if `command` is
/// not one of them, so the caller can try the shared backend.
pub async fn handle(
    platform: &PlatformState,
    principal: &Principal,
    command: &str,
    args: Value,
) -> Option<Reply> {
    Some(match command {
        "whoami" => whoami(principal),
        "list_projects" => list_projects(platform, principal).await,
        "list_all_projects" => list_all_projects(platform, principal).await,
        "create_project" => create_project(platform, principal, &args).await,
        "open_project" => open_project(platform, principal, &args).await,
        "rename_project" => rename_project(platform, principal, &args).await,
        "delete_project" => delete_project(platform, principal, &args).await,
        "list_project_members" => list_members(platform, principal, &args).await,
        "add_project_member" => add_member(platform, principal, &args).await,
        "set_project_member_role" => set_member_role(platform, principal, &args).await,
        "remove_project_member" => remove_member(platform, principal, &args).await,
        "lookup_user" => lookup_user(platform, principal, &args).await,
        "list_users" => list_users(platform, principal).await,
        "create_user" => create_user(platform, principal, &args).await,
        "set_user_password" => set_user_password(platform, principal, &args).await,
        "change_my_password" => change_my_password(platform, principal, &args).await,
        "set_user_disabled" => set_user_disabled(platform, principal, &args).await,
        "set_user_admin" => set_user_admin(platform, principal, &args).await,
        _ => return None,
    })
}

fn whoami(principal: &Principal) -> Reply {
    Ok(match principal {
        Principal::User(user) => json!({ "kind": "user", "user": user_json(user) }),
        Principal::Operator(_) => json!({ "kind": "operator", "isPlatformAdmin": true }),
        Principal::Worker => json!({ "kind": "worker", "isPlatformAdmin": false }),
    })
}

async fn list_projects(platform: &PlatformState, principal: &Principal) -> Reply {
    let projects = match principal {
        Principal::User(user) => platform.projects.list_for(user.id).await,
        // The operator and the worker work on every Project.
        _ => {
            let all = platform.projects.list_all(platform.operator.id).await;
            all.map(|mut all| {
                for project in &mut all {
                    project.role = Some(Role::Owner);
                }
                all
            })
        }
    }
    .map_err(denied)?;
    Ok(Value::Array(projects.iter().map(project_json).collect()))
}

async fn list_all_projects(platform: &PlatformState, principal: &Principal) -> Reply {
    require_admin(principal)?;
    let viewer = actor(principal)?;
    let all = platform
        .projects
        .list_all(viewer.id)
        .await
        .map_err(denied)?;
    Ok(Value::Array(all.iter().map(project_json).collect()))
}

async fn create_project(platform: &PlatformState, principal: &Principal, args: &Value) -> Reply {
    require_admin(principal)?;
    let name = text(args, "name")?;
    // A Project needs an owner who is a person. A Platform Admin owns what
    // they create; the operator names who.
    let owner = match principal {
        Principal::User(user) => user.clone(),
        _ => {
            let username = optional_text(args, "ownerUsername")
                .ok_or_else(|| bad_request("`ownerUsername` is required"))?;
            let found = platform
                .accounts
                .find_by_username(username)
                .await
                .map_err(denied)?;
            found.ok_or_else(|| Denied::new(StatusCode::NOT_FOUND, "no such user"))?
        }
    };
    let project = platform
        .projects
        .create(name, &owner)
        .await
        .map_err(denied)?;
    Ok(project_json(&project))
}

async fn open_project(platform: &PlatformState, principal: &Principal, args: &Value) -> Reply {
    let value = optional_text(args, "path")
        .or_else(|| optional_text(args, "projectId"))
        .ok_or_else(|| bad_request("`path` is required"))?;
    let (project, _) = project_ref(&ProjectLocator::default(), value).map_err(bad_request)?;
    let role = require_member(platform, principal, project, Role::Viewer).await?;
    let mut info = platform
        .projects
        .get(project, None)
        .await
        .map_err(denied)?
        .ok_or_else(|| Denied::new(StatusCode::NOT_FOUND, "no such project"))?;
    info.role = Some(role);
    Ok(project_json(&info))
}

async fn rename_project(platform: &PlatformState, principal: &Principal, args: &Value) -> Reply {
    let project = project_arg(args)?;
    require_manager(platform, principal, project).await?;
    platform
        .projects
        .rename(project, text(args, "name")?, actor(principal)?)
        .await
        .map_err(denied)?;
    Ok(Value::Null)
}

/// Deleting a Project deletes its files too, so it takes an owner (or the
/// operator): a Platform Admin is not enough on their own.
async fn delete_project(platform: &PlatformState, principal: &Principal, args: &Value) -> Reply {
    let project = project_arg(args)?;
    if !matches!(principal, Principal::Operator(_)) {
        require_member(platform, principal, project, Role::Owner).await?;
    }
    // The stored files first: once the rows are gone nothing says which
    // objects were this Project's.
    platform
        .blobs
        .delete_project_objects(project)
        .await
        .map_err(denied)?;
    platform
        .projects
        .delete(project, actor(principal)?)
        .await
        .map_err(denied)?;
    Ok(Value::Null)
}

async fn list_members(platform: &PlatformState, principal: &Principal, args: &Value) -> Reply {
    let project = project_arg(args)?;
    if !principal.is_platform_admin() {
        require_member(platform, principal, project, Role::Viewer).await?;
    }
    let members = platform.projects.members(project).await.map_err(denied)?;
    Ok(Value::Array(members.iter().map(member_json).collect()))
}

async fn member_target(platform: &PlatformState, args: &Value) -> Result<Uuid, Denied> {
    if let Some(username) = optional_text(args, "username") {
        let user = platform
            .accounts
            .find_by_username(username)
            .await
            .map_err(denied)?;
        return user
            .map(|u| u.id)
            .ok_or_else(|| Denied::new(StatusCode::NOT_FOUND, "no such user"));
    }
    uuid_arg(args, "userId")
}

async fn add_member(platform: &PlatformState, principal: &Principal, args: &Value) -> Reply {
    let project = project_arg(args)?;
    require_manager(platform, principal, project).await?;
    let user = member_target(platform, args).await?;
    let role = Role::parse(text(args, "role")?).map_err(denied)?;
    platform
        .projects
        .add_member(project, user, role, actor(principal)?)
        .await
        .map_err(denied)?;
    Ok(Value::Null)
}

async fn set_member_role(platform: &PlatformState, principal: &Principal, args: &Value) -> Reply {
    let project = project_arg(args)?;
    require_manager(platform, principal, project).await?;
    let user = member_target(platform, args).await?;
    let role = Role::parse(text(args, "role")?).map_err(denied)?;
    platform
        .projects
        .set_member_role(project, user, role, actor(principal)?)
        .await
        .map_err(denied)?;
    Ok(Value::Null)
}

/// A member may always leave a Project; removing someone else takes a manager.
async fn remove_member(platform: &PlatformState, principal: &Principal, args: &Value) -> Reply {
    let project = project_arg(args)?;
    let user = member_target(platform, args).await?;
    let leaving = principal.user().is_some_and(|me| me.id == user);
    if !leaving {
        require_manager(platform, principal, project).await?;
    }
    platform
        .projects
        .remove_member(project, user, actor(principal)?)
        .await
        .map_err(denied)?;
    Ok(Value::Null)
}

/// Finds an account by username so an owner can add it. Reveals only what
/// the owner needs to pick the right person.
async fn lookup_user(platform: &PlatformState, principal: &Principal, args: &Value) -> Reply {
    actor(principal)?;
    let found = platform
        .accounts
        .find_by_username(text(args, "username")?)
        .await
        .map_err(denied)?
        .filter(|user| !user.disabled && !user.is_service);
    Ok(match found {
        Some(user) => json!({
            "id": user.id.to_string(),
            "username": user.username,
            "displayName": user.display_name,
        }),
        None => Value::Null,
    })
}

async fn list_users(platform: &PlatformState, principal: &Principal) -> Reply {
    require_admin(principal)?;
    let users = platform.accounts.list_users().await.map_err(denied)?;
    Ok(Value::Array(
        users
            .iter()
            .filter(|user| !user.is_service)
            .map(user_json)
            .collect(),
    ))
}

async fn create_user(platform: &PlatformState, principal: &Principal, args: &Value) -> Reply {
    require_admin(principal)?;
    let user = platform
        .accounts
        .create_user(NewUser {
            username: text(args, "username")?.to_string(),
            display_name: optional_text(args, "displayName")
                .unwrap_or_else(|| optional_text(args, "username").unwrap_or_default())
                .to_string(),
            email: optional_text(args, "email").map(str::to_string),
            password: text(args, "password")?.to_string(),
            is_platform_admin: args
                .get("isPlatformAdmin")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        })
        .await
        .map_err(denied)?;
    Ok(user_json(&user))
}

async fn set_user_password(platform: &PlatformState, principal: &Principal, args: &Value) -> Reply {
    require_admin(principal)?;
    platform
        .accounts
        .set_password(uuid_arg(args, "userId")?, text(args, "password")?)
        .await
        .map_err(denied)?;
    Ok(Value::Null)
}

async fn change_my_password(
    platform: &PlatformState,
    principal: &Principal,
    args: &Value,
) -> Reply {
    let Principal::User(me) = principal else {
        return Err(Denied::forbidden("only a signed-in user has a password"));
    };
    let username = me
        .username
        .as_deref()
        .ok_or_else(|| Denied::forbidden("this account has no password"))?;
    // Their current password again, so a stolen session cannot lock them out.
    platform
        .accounts
        .authenticate(username, text(args, "currentPassword")?)
        .await
        .map_err(|_| Denied::new(StatusCode::UNAUTHORIZED, "the current password is wrong"))?;
    platform
        .accounts
        .set_password(me.id, text(args, "newPassword")?)
        .await
        .map_err(denied)?;
    Ok(Value::Null)
}

async fn set_user_disabled(platform: &PlatformState, principal: &Principal, args: &Value) -> Reply {
    require_admin(principal)?;
    platform
        .accounts
        .set_disabled(uuid_arg(args, "userId")?, flag(args, "disabled")?)
        .await
        .map_err(denied)?;
    Ok(Value::Null)
}

async fn set_user_admin(platform: &PlatformState, principal: &Principal, args: &Value) -> Reply {
    require_admin(principal)?;
    platform
        .accounts
        .set_platform_admin(uuid_arg(args, "userId")?, flag(args, "admin")?)
        .await
        .map_err(denied)?;
    Ok(Value::Null)
}
