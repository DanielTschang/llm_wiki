//! Platform mode: Project files live in the storage layer, not on the host.
//!
//! The multi-user server installs a [`Platform`] at startup. From then on the
//! file commands treat a path as `/projects/<uuid>/<relative path>` and serve
//! it from Postgres and MinIO; a path that names anything else is refused, so
//! a request can never reach the server host's own files. Without a platform
//! (tests, a plain folder checkout) the commands keep reading the disk.

use std::sync::{Arc, OnceLock};

use llm_wiki_storage::{BlobStreams, ProjectFiles, ProjectLocator};

pub struct Platform {
    pub files: Arc<dyn ProjectFiles>,
    /// Streaming access to blob contents, for uploads and downloads of files
    /// too large to hold in memory.
    pub streams: Option<Arc<dyn BlobStreams>>,
    pub locator: ProjectLocator,
}

static PLATFORM: OnceLock<Platform> = OnceLock::new();

/// Switches the process into platform mode. Call once, at startup.
pub fn install(
    files: Arc<dyn ProjectFiles>,
    streams: Option<Arc<dyn BlobStreams>>,
    locator: ProjectLocator,
) -> Result<(), String> {
    PLATFORM
        .set(Platform {
            files,
            streams,
            locator,
        })
        .map_err(|_| "the project file store is already installed".to_string())
}

pub fn get() -> Option<&'static Platform> {
    PLATFORM.get()
}

/// The error for a command that has not been moved onto the storage layer
/// yet. In platform mode it must fail rather than fall back to the host disk.
pub fn unsupported(command: &str) -> String {
    format!("{command} is not available on the platform server yet")
}
