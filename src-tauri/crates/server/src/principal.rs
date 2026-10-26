//! Who is asking, on a platform server.
//!
//! A browser is a signed-in user. A script holding the server token is the
//! operator, who runs the platform and is trusted with all of it. The ingest
//! worker is the platform acting on a Project's behalf.

use std::ops::Deref;

use llm_wiki_storage::{ProjectId, Projects, Role, Services, StoreError, User};

/// What a platform server needs beyond the stores: the account the operator's
/// token acts as, so what the operator does is attributed in the activity log.
#[derive(Clone)]
pub struct PlatformState {
    services: Services,
    pub operator: User,
}

impl PlatformState {
    pub fn new(services: Services, operator: User) -> Self {
        Self { services, operator }
    }
}

impl Deref for PlatformState {
    type Target = Services;

    fn deref(&self) -> &Services {
        &self.services
    }
}

#[derive(Debug, Clone)]
pub enum Principal {
    User(User),
    /// The holder of the server token, acting as the operator account.
    Operator(User),
    /// The ingest worker this server spawned.
    Worker,
}

impl Principal {
    /// The signed-in person, if this is one.
    pub fn user(&self) -> Option<&User> {
        match self {
            Principal::User(user) => Some(user),
            _ => None,
        }
    }

    /// The account whose name goes in the activity log.
    pub fn actor(&self) -> Option<&User> {
        match self {
            Principal::User(user) | Principal::Operator(user) => Some(user),
            Principal::Worker => None,
        }
    }

    pub fn is_platform_admin(&self) -> bool {
        match self {
            Principal::User(user) => user.is_platform_admin,
            Principal::Operator(_) => true,
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
            Principal::Operator(_) | Principal::Worker => Ok(Some(Role::Owner)),
            Principal::User(user) => projects.role_of(project, user.id).await,
        }
    }
}
