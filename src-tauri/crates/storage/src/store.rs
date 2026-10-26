use async_trait::async_trait;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures::stream::BoxStream;
use uuid::Uuid;

use crate::{RelPath, StoreError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProjectId(pub Uuid);

/// File content. Pages are text; Sources, images and agent artifacts are bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Body {
    Text(String),
    Bytes(Vec<u8>),
}

impl Body {
    /// The content as text: always for `Text`, and for `Bytes` when valid UTF-8.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Body::Text(text) => Some(text),
            Body::Bytes(bytes) => std::str::from_utf8(bytes).ok(),
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            Body::Text(text) => text.into_bytes(),
            Body::Bytes(bytes) => bytes,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Body::Text(text) => text.len(),
            Body::Bytes(bytes) => bytes.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    File,
    /// Directories are not stored; they exist while something lies under them.
    Dir,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Meta {
    pub path: RelPath,
    pub kind: EntryKind,
    pub size: u64,
    pub modified: DateTime<Utc>,
    /// Page version for optimistic locking. `None` for blobs and directories.
    pub version: Option<u32>,
}

/// What a write requires of the file's current state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Precondition {
    /// Create or overwrite.
    #[default]
    Any,
    /// Fail with `AlreadyExists` if the path is taken. The agent's page writes
    /// use this so retrieved text cannot make it overwrite a page.
    CreateOnly,
    /// Overwrite only if the page is still at this version.
    Version(u32),
}

/// Who is writing and why; recorded in the page's version log.
#[derive(Debug, Clone, Default)]
pub struct WriteCtx {
    pub actor: Option<Uuid>,
    /// `edit`, `ingest`, `dedup`, `agent`, `restore`, ...
    pub tool: String,
    pub ingest_job_id: Option<Uuid>,
}

impl WriteCtx {
    pub fn new(tool: impl Into<String>) -> Self {
        Self {
            tool: tool.into(),
            ..Self::default()
        }
    }
}

/// The Project file tree, as seen by the ingest worker, the agent, search and
/// the API. Every path is relative to the Project root.
#[async_trait]
pub trait ProjectFiles: Send + Sync {
    async fn read(&self, project: ProjectId, path: &RelPath) -> Result<(Body, Meta), StoreError>;

    /// Writes `body`, creating parent directories implicitly. Returns the
    /// stored file's new metadata.
    async fn write(
        &self,
        project: ProjectId,
        path: &RelPath,
        body: Body,
        precondition: Precondition,
        ctx: &WriteCtx,
    ) -> Result<Meta, StoreError>;

    /// Deletes a file, or everything under a directory. Returns how many
    /// files were removed; `NotFound` if there were none.
    async fn delete(
        &self,
        project: ProjectId,
        path: &RelPath,
        ctx: &WriteCtx,
    ) -> Result<usize, StoreError>;

    /// Moves a file or a directory. `AlreadyExists` if `to` is taken.
    async fn rename(
        &self,
        project: ProjectId,
        from: &RelPath,
        to: &RelPath,
        ctx: &WriteCtx,
    ) -> Result<(), StoreError>;

    async fn stat(&self, project: ProjectId, path: &RelPath) -> Result<Option<Meta>, StoreError>;

    /// Lists the entries under `dir`: its direct children, or with `recursive`
    /// every file beneath it. Sorted by path. A directory with nothing under
    /// it lists as empty.
    async fn list(
        &self,
        project: ProjectId,
        dir: &RelPath,
        recursive: bool,
    ) -> Result<Vec<Meta>, StoreError>;

    /// Reads every file beneath `dir`, in path order, up to `limit` files.
    /// For callers that scan a whole tree (search, the graph, embedding): a
    /// store that can fetch them in one go should override this instead of
    /// answering one `read` per file.
    async fn read_tree(
        &self,
        project: ProjectId,
        dir: &RelPath,
        limit: usize,
    ) -> Result<Vec<(Body, Meta)>, StoreError> {
        let mut out = Vec::new();
        for meta in self.list(project, dir, true).await? {
            if out.len() >= limit {
                break;
            }
            if meta.kind != EntryKind::File {
                continue;
            }
            match self.read(project, &meta.path).await {
                Ok(item) => out.push(item),
                // Deleted since the listing: not part of the tree any more.
                Err(StoreError::NotFound(_)) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(out)
    }
}

/// A body delivered in pieces, so large files never have to fit in memory.
pub type ByteStream = BoxStream<'static, Result<Bytes, StoreError>>;

/// Streaming access for stores of large files. Uploads and downloads pass
/// through the API this way; there are no presigned URLs.
#[async_trait]
pub trait BlobStreams: Send + Sync {
    async fn put_stream(
        &self,
        project: ProjectId,
        path: &RelPath,
        body: ByteStream,
        precondition: Precondition,
        ctx: &WriteCtx,
    ) -> Result<Meta, StoreError>;

    async fn get_stream(
        &self,
        project: ProjectId,
        path: &RelPath,
    ) -> Result<(ByteStream, Meta), StoreError>;
}
