//! Who is asking, on a platform server.
//!
//! A browser is a signed-in user. A script holding the server token is the
//! operator, who runs the platform and is trusted with all of it. The ingest
//! worker is the platform acting on a Project's behalf.

use llm_wiki_storage::{ProjectId, Projects, Role, StoreError, User};

#[derive(Debug, Clone)]
pub enum Principal {
    User(User),
    /// The holder of the server token.
    Operator,
    /// The ingest worker this server spawned.
    Worker,
}

impl Principal {
    pub fn user(&self) -> Option<&User> {
        match self {
            Principal::User(user) => Some(user),
            _ => None,
        }
    }

    pub fn is_platform_admin(&self) -> bool {
        match self {
            Principal::User(user) => user.is_platform_admin,
            Principal::Operator => true,
            Principal::Worker => false,
        }
    }

    /// The role this principal has in a Project. The operator and the worker
    /// work on every Project; a user only on the ones they belong to, and a
    /// Platform Admin is no exception until they add themselves as a member.
    pub async fn role_in(
        &self,
        projects: &Projects,
        project: ProjectId,
    ) -> Result<Option<Role>, StoreError> {
        match self {
            Principal::Operator | Principal::Worker => Ok(Some(Role::Owner)),
            Principal::User(user) => projects.role_of(project, user.id).await,
        }
    }
}
