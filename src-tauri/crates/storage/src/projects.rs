//! Projects and who belongs to them.
//!
//! Permissions exist only at Project level: a member has one role and it
//! applies to everything in the Project. Who may *ask* for a change (only a
//! Platform Admin creates a Project, only an Owner manages members) is decided
//! by the caller; what lives here are the rules that must hold whoever asks:
//! a Project always has an Owner, and members are real, active accounts.

use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::accounts::User;
use crate::sql::backend;
use crate::store::ProjectId;
use crate::StoreError;

const MAX_NAME_CHARS: usize = 120;

/// What a member may do in a Project. Ordered: each role includes the ones
/// before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Role {
    /// Browse, search, chat and use the agent.
    Viewer,
    /// Also upload Sources, ingest, edit pages and handle Review Items.
    Editor,
    /// Also manage members and settings, and delete pages.
    Owner,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Viewer => "viewer",
            Role::Editor => "editor",
            Role::Owner => "owner",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, StoreError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "viewer" => Ok(Role::Viewer),
            "editor" => Ok(Role::Editor),
            "owner" => Ok(Role::Owner),
            other => Err(StoreError::Invalid(format!(
                "{other:?} is not a role; use owner, editor or viewer"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectInfo {
    pub id: ProjectId,
    pub name: String,
    pub created_at: DateTime<Utc>,
    /// The asking user's role; `None` when listing for an administrator who is
    /// not a member.
    pub role: Option<Role>,
    pub member_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    pub user_id: Uuid,
    pub username: Option<String>,
    pub display_name: String,
    pub role: Role,
    pub added_at: DateTime<Utc>,
}

fn clean_name(raw: &str) -> Result<String, StoreError> {
    let name = raw.trim();
    if name.is_empty() {
        return Err(StoreError::Invalid("a project needs a name".into()));
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(StoreError::Invalid(format!(
            "a project name is at most {MAX_NAME_CHARS} characters"
        )));
    }
    Ok(name.to_string())
}

fn info_from_row(row: &PgRow) -> Result<ProjectInfo, StoreError> {
    let role: Option<String> = row.try_get("role").map_err(backend)?;
    Ok(ProjectInfo {
        id: ProjectId(row.try_get("id").map_err(backend)?),
        name: row.try_get("name").map_err(backend)?,
        created_at: row.try_get("created_at").map_err(backend)?,
        role: role.as_deref().map(Role::parse).transpose()?,
        member_count: row.try_get("member_count").map_err(backend)?,
    })
}

const PROJECT_SELECT: &str = "SELECT p.id, p.name, p.created_at, m.role,
    (SELECT count(*) FROM project_members x WHERE x.project_id = p.id) AS member_count
    FROM projects p LEFT JOIN project_members m ON m.project_id = p.id AND m.user_id = $1";

#[derive(Clone)]
pub struct Projects {
    pool: PgPool,
}

impl Projects {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn log(
        &self,
        project: Option<ProjectId>,
        actor: &User,
        action: &str,
        target: Option<Uuid>,
        detail: serde_json::Value,
    ) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO activity_log (project_id, actor_id, action, target_type, target_id, detail)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(project.map(|p| p.0))
        .bind(actor.id)
        .bind(action)
        .bind(target.map(|_| "user"))
        .bind(target.map(|t| t.to_string()))
        .bind(detail)
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    /// Creates a Project with `creator` as its first Owner. Only a Platform
    /// Admin may.
    pub async fn create(&self, name: &str, creator: &User) -> Result<ProjectInfo, StoreError> {
        if !creator.is_platform_admin {
            return Err(StoreError::Forbidden(
                "only a Platform Admin can create a project".into(),
            ));
        }
        let name = clean_name(name)?;
        let mut tx = self.pool.begin().await.map_err(backend)?;
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO projects (name, created_by) VALUES ($1, $2) RETURNING id",
        )
        .bind(&name)
        .bind(creator.id)
        .fetch_one(&mut *tx)
        .await
        .map_err(backend)?;
        sqlx::query(
            "INSERT INTO project_members (project_id, user_id, role, added_by) VALUES ($1, $2, 'owner', $2)",
        )
        .bind(id)
        .bind(creator.id)
        .execute(&mut *tx)
        .await
        .map_err(backend)?;
        tx.commit().await.map_err(backend)?;
        let project = ProjectId(id);
        self.log(
            Some(project),
            creator,
            "project.create",
            None,
            json!({ "name": name }),
        )
        .await?;
        self.get(project, Some(creator.id))
            .await?
            .ok_or_else(|| StoreError::Backend("the new project vanished".into()))
    }

    /// One Project, with `as_user`'s role in it if they have one.
    pub async fn get(
        &self,
        project: ProjectId,
        as_user: Option<Uuid>,
    ) -> Result<Option<ProjectInfo>, StoreError> {
        let row = sqlx::query(&format!("{PROJECT_SELECT} WHERE p.id = $2"))
            .bind(as_user)
            .bind(project.0)
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;
        row.as_ref().map(info_from_row).transpose()
    }

    /// The Projects `user_id` belongs to, newest first.
    pub async fn list_for(&self, user_id: Uuid) -> Result<Vec<ProjectInfo>, StoreError> {
        let rows = sqlx::query(&format!(
            "{PROJECT_SELECT} WHERE m.user_id IS NOT NULL ORDER BY p.created_at DESC, p.name"
        ))
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        rows.iter().map(info_from_row).collect()
    }

    /// Every Project, for administration. `role` is `as_user`'s, if any.
    pub async fn list_all(&self, as_user: Uuid) -> Result<Vec<ProjectInfo>, StoreError> {
        let rows = sqlx::query(&format!(
            "{PROJECT_SELECT} ORDER BY p.created_at DESC, p.name"
        ))
        .bind(as_user)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        rows.iter().map(info_from_row).collect()
    }

    pub async fn role_of(
        &self,
        project: ProjectId,
        user_id: Uuid,
    ) -> Result<Option<Role>, StoreError> {
        let role: Option<String> = sqlx::query_scalar(
            "SELECT m.role FROM project_members m
             JOIN users u ON u.id = m.user_id
             WHERE m.project_id = $1 AND m.user_id = $2 AND u.disabled_at IS NULL",
        )
        .bind(project.0)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(backend)?;
        role.as_deref().map(Role::parse).transpose()
    }

    pub async fn rename(
        &self,
        project: ProjectId,
        name: &str,
        actor: &User,
    ) -> Result<(), StoreError> {
        let name = clean_name(name)?;
        let changed = sqlx::query("UPDATE projects SET name = $2 WHERE id = $1")
            .bind(project.0)
            .bind(&name)
            .execute(&self.pool)
            .await
            .map_err(backend)?
            .rows_affected();
        if changed == 0 {
            return Err(StoreError::NotFound(format!("project {}", project.0)));
        }
        self.log(
            Some(project),
            actor,
            "project.rename",
            None,
            json!({ "name": name }),
        )
        .await
    }

    /// Removes the Project row, and with it every row that belongs to it. The
    /// objects in the object store are not touched here: delete those first,
    /// or they are orphaned.
    pub async fn delete(&self, project: ProjectId, actor: &User) -> Result<(), StoreError> {
        let name: Option<String> =
            sqlx::query_scalar("DELETE FROM projects WHERE id = $1 RETURNING name")
                .bind(project.0)
                .fetch_optional(&self.pool)
                .await
                .map_err(backend)?;
        let Some(name) = name else {
            return Err(StoreError::NotFound(format!("project {}", project.0)));
        };
        // The Project's own log went with it, so record this at platform level.
        self.log(
            None,
            actor,
            "project.delete",
            None,
            json!({ "project_id": project.0.to_string(), "name": name }),
        )
        .await
    }

    pub async fn members(&self, project: ProjectId) -> Result<Vec<Member>, StoreError> {
        let rows = sqlx::query(
            "SELECT m.user_id, c.username, u.display_name, m.role, m.added_at
             FROM project_members m
             JOIN users u ON u.id = m.user_id
             LEFT JOIN user_credentials c ON c.user_id = u.id
             WHERE m.project_id = $1
             ORDER BY CASE m.role WHEN 'owner' THEN 0 WHEN 'editor' THEN 1 ELSE 2 END,
                      lower(u.display_name)",
        )
        .bind(project.0)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        rows.iter()
            .map(|row| {
                let role: String = row.try_get("role").map_err(backend)?;
                Ok(Member {
                    user_id: row.try_get("user_id").map_err(backend)?,
                    username: row.try_get("username").map_err(backend)?,
                    display_name: row.try_get("display_name").map_err(backend)?,
                    role: Role::parse(&role)?,
                    added_at: row.try_get("added_at").map_err(backend)?,
                })
            })
            .collect()
    }

    pub async fn add_member(
        &self,
        project: ProjectId,
        user_id: Uuid,
        role: Role,
        actor: &User,
    ) -> Result<(), StoreError> {
        let user_ok: Option<bool> =
            sqlx::query_scalar("SELECT disabled_at IS NULL FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(backend)?;
        match user_ok {
            None => return Err(StoreError::NotFound(format!("user {user_id}"))),
            Some(false) => return Err(StoreError::Invalid("that account is disabled".into())),
            Some(true) => {}
        }
        let inserted = sqlx::query(
            "INSERT INTO project_members (project_id, user_id, role, added_by)
             SELECT $1, $2, $3, $4 WHERE EXISTS (SELECT 1 FROM projects WHERE id = $1)
             ON CONFLICT DO NOTHING",
        )
        .bind(project.0)
        .bind(user_id)
        .bind(role.as_str())
        .bind(actor.id)
        .execute(&self.pool)
        .await
        .map_err(backend)?
        .rows_affected();
        if inserted == 0 {
            return Err(if self.get(project, None).await?.is_none() {
                StoreError::NotFound(format!("project {}", project.0))
            } else {
                StoreError::AlreadyExists("that user is already a member".into())
            });
        }
        self.log(
            Some(project),
            actor,
            "member.add",
            Some(user_id),
            json!({ "role": role.as_str() }),
        )
        .await
    }

    pub async fn set_member_role(
        &self,
        project: ProjectId,
        user_id: Uuid,
        role: Role,
        actor: &User,
    ) -> Result<(), StoreError> {
        let mut tx = self.pool.begin().await.map_err(backend)?;
        let current = lock_member(&mut tx, project, user_id).await?;
        if current == Role::Owner
            && role != Role::Owner
            && owner_count(&mut tx, project).await? <= 1
        {
            return Err(StoreError::Forbidden(
                "a project must keep at least one owner".into(),
            ));
        }
        sqlx::query("UPDATE project_members SET role = $3 WHERE project_id = $1 AND user_id = $2")
            .bind(project.0)
            .bind(user_id)
            .bind(role.as_str())
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        tx.commit().await.map_err(backend)?;
        self.log(
            Some(project),
            actor,
            "member.role",
            Some(user_id),
            json!({ "from": current.as_str(), "to": role.as_str() }),
        )
        .await
    }

    pub async fn remove_member(
        &self,
        project: ProjectId,
        user_id: Uuid,
        actor: &User,
    ) -> Result<(), StoreError> {
        let mut tx = self.pool.begin().await.map_err(backend)?;
        let current = lock_member(&mut tx, project, user_id).await?;
        if current == Role::Owner && owner_count(&mut tx, project).await? <= 1 {
            return Err(StoreError::Forbidden(
                "a project must keep at least one owner".into(),
            ));
        }
        sqlx::query("DELETE FROM project_members WHERE project_id = $1 AND user_id = $2")
            .bind(project.0)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        tx.commit().await.map_err(backend)?;
        self.log(
            Some(project),
            actor,
            "member.remove",
            Some(user_id),
            json!({ "role": current.as_str() }),
        )
        .await
    }
}

/// The member's current role, with the Project's membership rows locked so
/// two simultaneous demotions cannot both see "another owner exists".
async fn lock_member(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    project: ProjectId,
    user_id: Uuid,
) -> Result<Role, StoreError> {
    let rows: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT user_id, role FROM project_members WHERE project_id = $1 FOR UPDATE",
    )
    .bind(project.0)
    .fetch_all(&mut **tx)
    .await
    .map_err(backend)?;
    let role = rows
        .iter()
        .find(|(id, _)| *id == user_id)
        .map(|(_, role)| Role::parse(role))
        .transpose()?;
    role.ok_or_else(|| StoreError::NotFound(format!("member {user_id}")))
}

async fn owner_count(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    project: ProjectId,
) -> Result<i64, StoreError> {
    sqlx::query_scalar(
        "SELECT count(*) FROM project_members m JOIN users u ON u.id = m.user_id
         WHERE m.project_id = $1 AND m.role = 'owner' AND u.disabled_at IS NULL",
    )
    .bind(project.0)
    .fetch_one(&mut **tx)
    .await
    .map_err(backend)
}

#[cfg(test)]
mod tests {
    use sqlx::postgres::PgPoolOptions;

    use super::*;
    use crate::accounts::{Accounts, NewUser};

    #[test]
    fn roles_are_ordered_and_parsed() {
        assert!(Role::Owner > Role::Editor && Role::Editor > Role::Viewer);
        assert_eq!(Role::parse(" Editor ").unwrap(), Role::Editor);
        assert!(Role::parse("admin").is_err());
        assert_eq!(Role::Owner.as_str(), "owner");
    }

    #[test]
    fn project_names_are_trimmed_and_bounded() {
        assert_eq!(clean_name("  Lithography  ").unwrap(), "Lithography");
        assert!(clean_name("   ").is_err());
        assert!(clean_name(&"x".repeat(MAX_NAME_CHARS + 1)).is_err());
    }

    struct World {
        accounts: Accounts,
        projects: Projects,
        pool: PgPool,
    }

    async fn world() -> Option<World> {
        let url = std::env::var("LLM_WIKI_TEST_DATABASE_URL").ok()?;
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .connect(&url)
            .await
            .unwrap();
        crate::migrate(&pool).await.unwrap();
        Some(World {
            accounts: Accounts::new(pool.clone()),
            projects: Projects::new(pool.clone()),
            pool,
        })
    }

    impl World {
        async fn user(&self, admin: bool) -> User {
            let name = format!("u-{}", &Uuid::new_v4().simple().to_string()[..12]);
            self.accounts
                .create_user(NewUser {
                    username: name.clone(),
                    display_name: name,
                    email: None,
                    password: "a fine password".into(),
                    is_platform_admin: admin,
                })
                .await
                .unwrap()
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn only_a_platform_admin_creates_a_project_and_becomes_its_owner() {
        let Some(w) = world().await else { return };
        let (admin, member) = (w.user(true).await, w.user(false).await);
        let refused = w.projects.create("Nope", &member).await;
        assert!(
            matches!(refused, Err(StoreError::Forbidden(_))),
            "{refused:?}"
        );

        let project = w.projects.create("  Lithography  ", &admin).await.unwrap();
        assert_eq!(project.name, "Lithography");
        assert_eq!(project.role, Some(Role::Owner));
        assert_eq!(project.member_count, 1);
        assert_eq!(
            w.projects.role_of(project.id, admin.id).await.unwrap(),
            Some(Role::Owner)
        );
        assert_eq!(
            w.projects.role_of(project.id, member.id).await.unwrap(),
            None
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_user_sees_only_the_projects_they_belong_to() {
        let Some(w) = world().await else { return };
        let (admin, alice, bob) = (w.user(true).await, w.user(false).await, w.user(false).await);
        let shared = w.projects.create("Shared", &admin).await.unwrap();
        let private = w.projects.create("Private", &admin).await.unwrap();
        w.projects
            .add_member(shared.id, alice.id, Role::Viewer, &admin)
            .await
            .unwrap();
        w.projects
            .add_member(private.id, bob.id, Role::Editor, &admin)
            .await
            .unwrap();

        let of = |user: &User| {
            let projects = w.projects.clone();
            let id = user.id;
            async move { projects.list_for(id).await.unwrap() }
        };
        let alice_sees: Vec<_> = of(&alice)
            .await
            .into_iter()
            .map(|p| (p.id, p.role))
            .collect();
        assert_eq!(alice_sees, [(shared.id, Some(Role::Viewer))]);
        let bob_sees: Vec<_> = of(&bob).await.into_iter().map(|p| p.id).collect();
        assert_eq!(bob_sees, [private.id]);

        // An administrator who is not a member can list but has no role.
        let outsider_admin = w.user(true).await;
        let all = w.projects.list_all(outsider_admin.id).await.unwrap();
        let seen = all.iter().find(|p| p.id == shared.id).unwrap();
        assert_eq!(seen.role, None);
        assert!(w
            .projects
            .list_for(outsider_admin.id)
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_project_always_keeps_an_owner() {
        let Some(w) = world().await else { return };
        let (admin, other) = (w.user(true).await, w.user(false).await);
        let project = w.projects.create("Owned", &admin).await.unwrap();

        // The only owner cannot leave or step down.
        for result in [
            w.projects.remove_member(project.id, admin.id, &admin).await,
            w.projects
                .set_member_role(project.id, admin.id, Role::Editor, &admin)
                .await,
        ] {
            assert!(
                matches!(result, Err(StoreError::Forbidden(_))),
                "{result:?}"
            );
        }
        // With a second owner either may.
        w.projects
            .add_member(project.id, other.id, Role::Owner, &admin)
            .await
            .unwrap();
        w.projects
            .set_member_role(project.id, admin.id, Role::Editor, &admin)
            .await
            .unwrap();
        assert_eq!(
            w.projects.role_of(project.id, admin.id).await.unwrap(),
            Some(Role::Editor)
        );
        assert!(matches!(
            w.projects.remove_member(project.id, other.id, &admin).await,
            Err(StoreError::Forbidden(_))
        ));
        w.projects
            .remove_member(project.id, admin.id, &other)
            .await
            .unwrap();
        assert_eq!(w.projects.members(project.id).await.unwrap().len(), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn membership_changes_are_checked_and_logged() {
        let Some(w) = world().await else { return };
        let (admin, alice) = (w.user(true).await, w.user(false).await);
        let project = w.projects.create("Team", &admin).await.unwrap();

        w.projects
            .add_member(project.id, alice.id, Role::Viewer, &admin)
            .await
            .unwrap();
        assert!(matches!(
            w.projects
                .add_member(project.id, alice.id, Role::Editor, &admin)
                .await,
            Err(StoreError::AlreadyExists(_))
        ));
        assert!(matches!(
            w.projects
                .add_member(project.id, Uuid::new_v4(), Role::Viewer, &admin)
                .await,
            Err(StoreError::NotFound(_))
        ));
        assert!(matches!(
            w.projects
                .add_member(ProjectId(Uuid::new_v4()), alice.id, Role::Viewer, &admin)
                .await,
            Err(StoreError::NotFound(_))
        ));
        w.projects
            .set_member_role(project.id, alice.id, Role::Editor, &admin)
            .await
            .unwrap();

        // A disabled account cannot be added, and a disabled member has no role.
        let gone = w.user(false).await;
        w.accounts.set_disabled(gone.id, true).await.unwrap();
        assert!(matches!(
            w.projects
                .add_member(project.id, gone.id, Role::Viewer, &admin)
                .await,
            Err(StoreError::Invalid(_))
        ));
        w.accounts.set_disabled(alice.id, true).await.unwrap();
        assert_eq!(
            w.projects.role_of(project.id, alice.id).await.unwrap(),
            None
        );

        let members = w.projects.members(project.id).await.unwrap();
        assert_eq!(members[0].role, Role::Owner);
        let actions: Vec<String> =
            sqlx::query_scalar("SELECT action FROM activity_log WHERE project_id = $1 ORDER BY id")
                .bind(project.id.0)
                .fetch_all(&w.pool)
                .await
                .unwrap();
        assert_eq!(actions, ["project.create", "member.add", "member.role"]);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn renaming_and_deleting_a_project_take_its_rows_with_it() {
        let Some(w) = world().await else { return };
        let admin = w.user(true).await;
        let project = w.projects.create("Before", &admin).await.unwrap();
        w.projects
            .rename(project.id, "After", &admin)
            .await
            .unwrap();
        assert_eq!(
            w.projects
                .get(project.id, None)
                .await
                .unwrap()
                .unwrap()
                .name,
            "After"
        );

        sqlx::query("INSERT INTO state_files (project_id, path, content) VALUES ($1, '.llm-wiki/x.json', '{}')")
            .bind(project.id.0)
            .execute(&w.pool)
            .await
            .unwrap();
        w.projects.delete(project.id, &admin).await.unwrap();
        assert!(w.projects.get(project.id, None).await.unwrap().is_none());
        let left: i64 =
            sqlx::query_scalar("SELECT count(*) FROM state_files WHERE project_id = $1")
                .bind(project.id.0)
                .fetch_one(&w.pool)
                .await
                .unwrap();
        assert_eq!(left, 0);
        assert!(matches!(
            w.projects.delete(project.id, &admin).await,
            Err(StoreError::NotFound(_))
        ));
        // The deletion is on record even though the Project's own log is gone.
        let logged: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM activity_log WHERE action = 'project.delete' AND detail->>'project_id' = $1",
        )
        .bind(project.id.0.to_string())
        .fetch_one(&w.pool)
        .await
        .unwrap();
        assert_eq!(logged, 1);
    }
}
