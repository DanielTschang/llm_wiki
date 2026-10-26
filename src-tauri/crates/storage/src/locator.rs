use uuid::Uuid;

use crate::{ProjectId, RelPath, StoreError};

/// Turns the absolute "project path" strings the RPC layer and the TS app
/// already pass around into `(ProjectId, RelPath)`.
///
/// There is no folder on disk any more, so a Project's path is a virtual
/// root, `<prefix>/<project uuid>` (default `/projects/<uuid>`). Callers that
/// still hold a `project_path: &str` keep working; the string just no longer
/// names a directory.
#[derive(Debug, Clone)]
pub struct ProjectLocator {
    prefix: String,
}

impl Default for ProjectLocator {
    fn default() -> Self {
        Self::new("/projects")
    }
}

impl ProjectLocator {
    pub fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.trim_end_matches('/').to_string(),
        }
    }

    pub fn project_path(&self, project: ProjectId) -> String {
        format!("{}/{}", self.prefix, project.0)
    }

    /// Resolves an absolute path to a Project and a Project-relative path
    /// (the root for the bare project path). Rejects anything that is not
    /// under the prefix, so a path can never name a host file.
    pub fn resolve(&self, absolute: &str) -> Result<(ProjectId, RelPath), StoreError> {
        let invalid = |reason| StoreError::InvalidPath {
            path: absolute.to_string(),
            reason,
        };
        let rest = absolute
            .strip_prefix(self.prefix.as_str())
            .and_then(|rest| rest.strip_prefix('/'))
            .ok_or_else(|| invalid("not under the projects root"))?;
        let (id, rel) = match rest.split_once('/') {
            Some((id, rel)) => (id, rel),
            None => (rest, ""),
        };
        let id = Uuid::parse_str(id).map_err(|_| invalid("not a project id"))?;
        let rel = if rel.is_empty() {
            RelPath::root()
        } else {
            RelPath::parse(rel)?
        };
        Ok((ProjectId(id), rel))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_a_project_path() {
        let locator = ProjectLocator::default();
        let id = ProjectId(Uuid::new_v4());
        let root = locator.project_path(id);
        assert_eq!(locator.resolve(&root).unwrap(), (id, RelPath::root()));
        let page = format!("{root}/wiki/a.md");
        assert_eq!(
            locator.resolve(&page).unwrap(),
            (id, RelPath::parse("wiki/a.md").unwrap())
        );
    }

    #[test]
    fn rejects_host_paths_and_escapes() {
        let locator = ProjectLocator::default();
        let id = Uuid::new_v4();
        for bad in [
            "/etc/passwd".to_string(),
            "/projects".to_string(),
            "/projects/not-a-uuid/wiki".to_string(),
            format!("/projects/{id}/../other"),
            format!("/projectsx/{id}/a"),
            format!("projects/{id}/a"),
        ] {
            assert!(locator.resolve(&bad).is_err(), "{bad} should be rejected");
        }
    }
}
