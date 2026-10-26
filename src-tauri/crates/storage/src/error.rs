use crate::RelPath;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum StoreError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("already exists: {0}")]
    AlreadyExists(String),
    /// A writer's `Precondition::Version` no longer matches. Carries the
    /// version that is current so the caller can re-read and merge.
    #[error("version conflict on {path}: current version is {current}")]
    VersionConflict { path: String, current: u32 },
    #[error("invalid path {path:?}: {reason}")]
    InvalidPath { path: String, reason: &'static str },
    #[error("{0}")]
    Backend(String),
}

impl StoreError {
    pub(crate) fn not_found(path: &RelPath) -> Self {
        Self::NotFound(path.to_string())
    }

    pub(crate) fn already_exists(path: &RelPath) -> Self {
        Self::AlreadyExists(path.to_string())
    }
}
