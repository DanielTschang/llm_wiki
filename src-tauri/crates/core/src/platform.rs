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

impl Platform {
    /// The Markdown pages under a Project's `wiki/`, as (project-relative
    /// path, content), in path order, at most `limit` files. One store call,
    /// not one per page.
    pub async fn wiki_markdown(
        &self,
        project_path: &str,
        limit: usize,
    ) -> Result<Vec<(String, String)>, String> {
        use llm_wiki_storage::RelPath;

        let (project, root) = self
            .locator
            .resolve(project_path)
            .map_err(|error| error.to_string())?;
        if !root.is_root() {
            return Err(format!("not a project path: {project_path}"));
        }
        let wiki = RelPath::parse("wiki").map_err(|error| error.to_string())?;
        let tree = self
            .files
            .read_tree(project, &wiki, limit)
            .await
            .map_err(|error| error.to_string())?;
        Ok(tree
            .into_iter()
            .filter(|(_, meta)| {
                meta.path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
            })
            .filter_map(|(body, meta)| {
                body.as_text()
                    .map(|text| (meta.path.to_string(), text.to_string()))
            })
            .collect())
    }

    /// One file of a Project as text; `None` if it is missing or not text.
    pub async fn read_text(&self, project_path: &str, rel: &str) -> Option<String> {
        use llm_wiki_storage::RelPath;

        let (project, root) = self.locator.resolve(project_path).ok()?;
        if !root.is_root() {
            return None;
        }
        let (body, _) = self
            .files
            .read(project, &RelPath::parse(rel).ok()?)
            .await
            .ok()?;
        body.as_text().map(str::to_string)
    }
}
