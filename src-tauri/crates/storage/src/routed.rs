use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;

use crate::route::{route, Backend};
use crate::store::{
    BlobStreams, Body, ByteStream, EntryKind, Meta, Precondition, ProjectFiles, ProjectId, WriteCtx,
};
use crate::{RelPath, StoreError};

const STATE_DIR: &str = ".llm-wiki";

/// The whole Project file tree, assembled from its stores: pages, blobs and
/// state. A path is served by whichever store [`route`] names for it; a
/// directory is the union of what the stores hold under it. Callers see one
/// tree and never learn where a file lives.
///
/// Calls that touch two stores (a directory holding both pages and blobs, or
/// a rename that changes a file's store) are not atomic across them. Renames
/// undo what they already moved when a later step fails.
pub struct RoutedFiles {
    pages: Arc<dyn ProjectFiles>,
    blobs: Arc<dyn ProjectFiles>,
    state: Option<Arc<dyn ProjectFiles>>,
    streams: Option<Arc<dyn BlobStreams>>,
}

/// One file that a rename relocates.
struct Move {
    from_backend: Backend,
    to_backend: Backend,
    from: RelPath,
    to: RelPath,
}

impl Move {
    fn reversed(&self) -> Move {
        Move {
            from_backend: self.to_backend,
            to_backend: self.from_backend,
            from: self.to.clone(),
            to: self.from.clone(),
        }
    }
}

impl RoutedFiles {
    pub fn new(pages: Arc<dyn ProjectFiles>, blobs: Arc<dyn ProjectFiles>) -> Self {
        Self {
            pages,
            blobs,
            state: None,
            streams: None,
        }
    }

    /// The store behind `.llm-wiki/**`. Without one those paths are absent
    /// when read and refused when written.
    pub fn with_state(mut self, state: Arc<dyn ProjectFiles>) -> Self {
        self.state = Some(state);
        self
    }

    /// Streaming access to the blob store, for [`BlobStreams`].
    pub fn with_blob_streams(mut self, streams: Arc<dyn BlobStreams>) -> Self {
        self.streams = Some(streams);
        self
    }

    fn store(&self, backend: Backend) -> Result<&dyn ProjectFiles, StoreError> {
        match backend {
            Backend::Pages => Ok(self.pages.as_ref()),
            Backend::Blobs => Ok(self.blobs.as_ref()),
            Backend::State => self
                .state
                .as_deref()
                .ok_or_else(|| StoreError::Backend("state files are not configured".into())),
        }
    }

    /// The stores that can hold something at or under `path`, the one that
    /// would hold `path` itself first.
    fn scope(&self, path: &RelPath) -> Vec<Backend> {
        if is_state(path) {
            return if self.state.is_some() {
                vec![Backend::State]
            } else {
                Vec::new()
            };
        }
        let mut out = if route(path) == Backend::Blobs {
            vec![Backend::Blobs, Backend::Pages]
        } else {
            vec![Backend::Pages, Backend::Blobs]
        };
        if path.is_root() && self.state.is_some() {
            out.push(Backend::State);
        }
        out
    }

    /// Refuses a write that would put a file where the other content store has
    /// a directory, or beneath a file the other store holds. Each store checks
    /// this within itself; this covers the seam between them.
    async fn check_placement(
        &self,
        project: ProjectId,
        path: &RelPath,
        backend: Backend,
    ) -> Result<(), StoreError> {
        let other = match backend {
            Backend::Pages => Backend::Blobs,
            Backend::Blobs => Backend::Pages,
            Backend::State => return Ok(()),
        };
        let other_store = self.store(other)?;
        if other_store.stat(project, path).await?.is_some() {
            return Err(StoreError::already_exists(path));
        }
        for ancestor in path.ancestors() {
            if route(&ancestor) == other {
                let held = other_store.stat(project, &ancestor).await?;
                if held.is_some_and(|meta| meta.kind == EntryKind::File) {
                    return Err(StoreError::already_exists(&ancestor));
                }
            }
        }
        Ok(())
    }

    /// Whether `to` is free in every content store, including as a directory.
    async fn check_target(&self, project: ProjectId, to: &RelPath) -> Result<(), StoreError> {
        for backend in [Backend::Pages, Backend::Blobs] {
            if self.store(backend)?.stat(project, to).await?.is_some() {
                return Err(StoreError::already_exists(to));
            }
        }
        for ancestor in to.ancestors() {
            let held = self
                .store(route(&ancestor))?
                .stat(project, &ancestor)
                .await?;
            if held.is_some_and(|meta| meta.kind == EntryKind::File) {
                return Err(StoreError::already_exists(&ancestor));
            }
        }
        Ok(())
    }

    /// Every file at or under `path` in the content stores.
    async fn files_under(
        &self,
        project: ProjectId,
        path: &RelPath,
    ) -> Result<Vec<(Backend, RelPath)>, StoreError> {
        let mut out = Vec::new();
        for backend in [Backend::Pages, Backend::Blobs] {
            let store = self.store(backend)?;
            match store.stat(project, path).await? {
                None => {}
                Some(meta) if meta.kind == EntryKind::File => out.push((backend, meta.path)),
                Some(_) => {
                    for meta in store.list(project, path, true).await? {
                        if meta.kind == EntryKind::File {
                            out.push((backend, meta.path));
                        }
                    }
                }
            }
        }
        Ok(out)
    }

    async fn move_file(
        &self,
        project: ProjectId,
        step: &Move,
        ctx: &WriteCtx,
    ) -> Result<(), StoreError> {
        let from = self.store(step.from_backend)?;
        if step.from_backend == step.to_backend {
            return from.rename(project, &step.from, &step.to, ctx).await;
        }
        // The file changes store: copy it, then drop the original.
        let to = self.store(step.to_backend)?;
        let (body, _) = from.read(project, &step.from).await?;
        let body = match (step.to_backend, body) {
            (Backend::Pages, Body::Bytes(bytes)) => Body::Text(String::from_utf8(bytes).map_err(
                |_| StoreError::InvalidPath {
                    path: step.to.to_string(),
                    reason: "binary content cannot become a page",
                },
            )?),
            (_, body) => body,
        };
        to.write(project, &step.to, body, Precondition::CreateOnly, ctx)
            .await?;
        if let Err(error) = from.delete(project, &step.from, ctx).await {
            let _ = to.delete(project, &step.to, ctx).await;
            return Err(error);
        }
        Ok(())
    }

    fn streams(&self, path: &RelPath) -> Result<&dyn BlobStreams, StoreError> {
        if route(path) != Backend::Blobs {
            return Err(StoreError::InvalidPath {
                path: path.to_string(),
                reason: "streaming is only for blob paths",
            });
        }
        self.streams
            .as_deref()
            .ok_or_else(|| StoreError::Backend("blob streaming is not configured".into()))
    }
}

fn is_state(path: &RelPath) -> bool {
    path.top_level() == STATE_DIR
}

/// The same directory seen by two stores: the newer modification time wins.
fn merge(held: Meta, other: Meta) -> Meta {
    match (held.kind, other.kind) {
        (EntryKind::Dir, EntryKind::Dir) => Meta {
            modified: held.modified.max(other.modified),
            ..held
        },
        (EntryKind::File, _) => held,
        _ => other,
    }
}

#[async_trait]
impl ProjectFiles for RoutedFiles {
    async fn read(&self, project: ProjectId, path: &RelPath) -> Result<(Body, Meta), StoreError> {
        self.store(route(path))?.read(project, path).await
    }

    async fn write(
        &self,
        project: ProjectId,
        path: &RelPath,
        body: Body,
        precondition: Precondition,
        ctx: &WriteCtx,
    ) -> Result<Meta, StoreError> {
        let backend = route(path);
        let store = self.store(backend)?;
        self.check_placement(project, path, backend).await?;
        store.write(project, path, body, precondition, ctx).await
    }

    async fn delete(
        &self,
        project: ProjectId,
        path: &RelPath,
        ctx: &WriteCtx,
    ) -> Result<usize, StoreError> {
        let mut removed = 0;
        for backend in self.scope(path) {
            match self.store(backend)?.delete(project, path, ctx).await {
                Ok(count) => removed += count,
                Err(StoreError::NotFound(_)) => {}
                Err(error) => return Err(error),
            }
        }
        if removed == 0 {
            return Err(StoreError::not_found(path));
        }
        Ok(removed)
    }

    async fn rename(
        &self,
        project: ProjectId,
        from: &RelPath,
        to: &RelPath,
        ctx: &WriteCtx,
    ) -> Result<(), StoreError> {
        if from.is_root() || to.is_root() {
            return Err(StoreError::InvalidPath {
                path: String::new(),
                reason: "cannot move the root",
            });
        }
        if is_state(from) != is_state(to) {
            return Err(StoreError::InvalidPath {
                path: to.to_string(),
                reason: "files cannot move between state and content",
            });
        }
        if is_state(from) {
            return self
                .store(Backend::State)?
                .rename(project, from, to, ctx)
                .await;
        }

        let files = self.files_under(project, from).await?;
        if files.is_empty() {
            return Err(StoreError::not_found(from));
        }
        if to.is_within(from) {
            return Err(StoreError::InvalidPath {
                path: to.to_string(),
                reason: "destination is inside the source",
            });
        }
        self.check_target(project, to).await?;

        let moves = files
            .into_iter()
            .map(|(from_backend, path)| {
                let suffix = path.as_str().strip_prefix(from.as_str()).unwrap_or("");
                let target = RelPath::parse(&format!("{to}{suffix}"))?;
                Ok(Move {
                    from_backend,
                    to_backend: route(&target),
                    from: path,
                    to: target,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;

        if moves.iter().all(|m| m.from_backend == m.to_backend) {
            // No file changes store, so each store moves its own share in one step.
            let mut moved: Vec<Backend> = Vec::new();
            for backend in [Backend::Pages, Backend::Blobs] {
                if !moves.iter().any(|m| m.from_backend == backend) {
                    continue;
                }
                if let Err(error) = self.store(backend)?.rename(project, from, to, ctx).await {
                    for done in moved {
                        let _ = self.store(done)?.rename(project, to, from, ctx).await;
                    }
                    return Err(error);
                }
                moved.push(backend);
            }
            return Ok(());
        }

        let mut done: Vec<&Move> = Vec::new();
        for step in &moves {
            if let Err(error) = self.move_file(project, step, ctx).await {
                for finished in done.into_iter().rev() {
                    let _ = self.move_file(project, &finished.reversed(), ctx).await;
                }
                return Err(error);
            }
            done.push(step);
        }
        Ok(())
    }

    async fn stat(&self, project: ProjectId, path: &RelPath) -> Result<Option<Meta>, StoreError> {
        let mut found: Option<Meta> = None;
        for backend in self.scope(path) {
            let Some(meta) = self.store(backend)?.stat(project, path).await? else {
                continue;
            };
            if meta.kind == EntryKind::File {
                return Ok(Some(meta));
            }
            found = Some(match found {
                Some(held) => merge(held, meta),
                None => meta,
            });
        }
        Ok(found)
    }

    async fn read_tree(
        &self,
        project: ProjectId,
        dir: &RelPath,
        limit: usize,
    ) -> Result<Vec<(Body, Meta)>, StoreError> {
        let mut all = Vec::new();
        for backend in self.scope(dir) {
            all.extend(self.store(backend)?.read_tree(project, dir, limit).await?);
        }
        all.sort_by(|(_, a), (_, b)| a.path.cmp(&b.path));
        all.truncate(limit);
        Ok(all)
    }

    async fn list(
        &self,
        project: ProjectId,
        dir: &RelPath,
        recursive: bool,
    ) -> Result<Vec<Meta>, StoreError> {
        let mut merged: BTreeMap<RelPath, Meta> = BTreeMap::new();
        for backend in self.scope(dir) {
            for meta in self.store(backend)?.list(project, dir, recursive).await? {
                match merged.remove(&meta.path) {
                    Some(held) => merged.insert(meta.path.clone(), merge(held, meta)),
                    None => merged.insert(meta.path.clone(), meta),
                };
            }
        }
        Ok(merged.into_values().collect())
    }
}

#[async_trait]
impl BlobStreams for RoutedFiles {
    async fn put_stream(
        &self,
        project: ProjectId,
        path: &RelPath,
        body: ByteStream,
        precondition: Precondition,
        ctx: &WriteCtx,
    ) -> Result<Meta, StoreError> {
        let streams = self.streams(path)?;
        self.check_placement(project, path, Backend::Blobs).await?;
        streams
            .put_stream(project, path, body, precondition, ctx)
            .await
    }

    async fn get_stream(
        &self,
        project: ProjectId,
        path: &RelPath,
    ) -> Result<(ByteStream, Meta), StoreError> {
        self.streams(path)?.get_stream(project, path).await
    }
}

#[cfg(test)]
mod tests {
    use object_store::memory::InMemory;
    use sqlx::postgres::PgPoolOptions;
    use sqlx::PgPool;

    use super::*;
    use crate::conformance::{self, Harness};
    use crate::{MemoryStore, PgBlobs, PgFiles, PgState};

    fn rel(path: &str) -> RelPath {
        if path.is_empty() {
            return RelPath::root();
        }
        RelPath::parse(path).unwrap()
    }

    fn ctx() -> WriteCtx {
        WriteCtx::new("test")
    }

    /// Three memory stores behind one `RoutedFiles`, kept so tests can look
    /// at which store a file really landed in.
    struct Fixture {
        routed: RoutedFiles,
        pages: Arc<MemoryStore>,
        blobs: Arc<MemoryStore>,
        state: Arc<MemoryStore>,
        project: ProjectId,
    }

    fn fixture() -> Fixture {
        let pages = Arc::new(MemoryStore::new());
        let blobs = Arc::new(MemoryStore::new());
        let state = Arc::new(MemoryStore::new());
        Fixture {
            routed: RoutedFiles::new(pages.clone(), blobs.clone()).with_state(state.clone()),
            pages,
            blobs,
            state,
            project: ProjectId(uuid::Uuid::new_v4()),
        }
    }

    impl Fixture {
        async fn put(&self, path: &str, body: Body) {
            self.routed
                .write(self.project, &rel(path), body, Precondition::Any, &ctx())
                .await
                .unwrap();
        }

        async fn has(&self, store: &MemoryStore, path: &str) -> bool {
            store
                .stat(self.project, &rel(path))
                .await
                .unwrap()
                .is_some()
        }

        async fn names(&self, dir: &str, recursive: bool) -> Vec<String> {
            self.routed
                .list(self.project, &rel(dir), recursive)
                .await
                .unwrap()
                .into_iter()
                .map(|meta| meta.path.to_string())
                .collect()
        }
    }

    fn text(content: &str) -> Body {
        Body::Text(content.to_string())
    }

    #[tokio::test]
    async fn each_path_is_stored_by_the_store_route_names() {
        let f = fixture();
        f.put("wiki/concepts/a.md", text("page")).await;
        f.put("purpose.md", text("why")).await;
        f.put("raw/sources/report.pdf", Body::Bytes(vec![1, 2]))
            .await;
        f.put("wiki/media/pic.png", Body::Bytes(vec![3])).await;
        f.put(".llm-wiki/review.json", text("[]")).await;

        assert!(f.has(&f.pages, "wiki/concepts/a.md").await);
        assert!(f.has(&f.pages, "purpose.md").await);
        assert!(f.has(&f.blobs, "raw/sources/report.pdf").await);
        assert!(f.has(&f.blobs, "wiki/media/pic.png").await);
        assert!(f.has(&f.state, ".llm-wiki/review.json").await);
        assert!(!f.has(&f.blobs, "wiki/concepts/a.md").await);
        assert!(!f.has(&f.pages, "raw/sources/report.pdf").await);

        let (body, _) = f
            .routed
            .read(f.project, &rel("raw/sources/report.pdf"))
            .await
            .unwrap();
        assert_eq!(body, Body::Bytes(vec![1, 2]));
    }

    #[tokio::test]
    async fn a_directory_lists_what_every_store_holds_under_it() {
        let f = fixture();
        f.put("wiki/a.md", text("page")).await;
        f.put("wiki/media/pic.png", Body::Bytes(vec![1])).await;
        f.put("wiki/logo.png", Body::Bytes(vec![2])).await;
        f.put("purpose.md", text("why")).await;
        f.put("raw/sources/s.pdf", Body::Bytes(vec![3])).await;
        f.put(".llm-wiki/ingest.json", text("{}")).await;

        assert_eq!(
            f.names("", false).await,
            [".llm-wiki", "purpose.md", "raw", "wiki"]
        );
        assert_eq!(
            f.names("wiki", false).await,
            ["wiki/a.md", "wiki/logo.png", "wiki/media"]
        );
        assert_eq!(
            f.names("wiki", true).await,
            ["wiki/a.md", "wiki/logo.png", "wiki/media/pic.png"]
        );
        assert_eq!(f.names(".llm-wiki", true).await, [".llm-wiki/ingest.json"]);
    }

    #[tokio::test]
    async fn stat_of_a_directory_spans_stores() {
        let f = fixture();
        f.put("wiki/a.md", text("page")).await;
        f.put("wiki/media/pic.png", Body::Bytes(vec![1])).await;
        let wiki = f
            .routed
            .stat(f.project, &rel("wiki"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(wiki.kind, EntryKind::Dir);
        let media = f
            .routed
            .stat(f.project, &rel("wiki/media"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(media.kind, EntryKind::Dir);
        assert!(f
            .routed
            .stat(f.project, &rel("nothing"))
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn deleting_a_directory_removes_it_from_every_store() {
        let f = fixture();
        f.put("wiki/a.md", text("page")).await;
        f.put("wiki/media/pic.png", Body::Bytes(vec![1])).await;
        f.put("raw/sources/s.pdf", Body::Bytes(vec![2])).await;
        let removed = f
            .routed
            .delete(f.project, &rel("wiki"), &ctx())
            .await
            .unwrap();
        assert_eq!(removed, 2);
        assert!(f.has(&f.blobs, "raw/sources/s.pdf").await);
        assert_eq!(
            f.routed.delete(f.project, &rel("wiki"), &ctx()).await,
            Err(StoreError::NotFound("wiki".into()))
        );
    }

    #[tokio::test]
    async fn renaming_a_directory_moves_pages_and_blobs_alike() {
        let f = fixture();
        f.put("wiki/old/a.md", text("page")).await;
        f.put("wiki/old/pic.png", Body::Bytes(vec![1])).await;
        f.routed
            .rename(f.project, &rel("wiki/old"), &rel("wiki/new"), &ctx())
            .await
            .unwrap();
        assert_eq!(
            f.names("wiki", true).await,
            ["wiki/new/a.md", "wiki/new/pic.png"]
        );
        assert!(f.has(&f.pages, "wiki/new/a.md").await);
        assert!(f.has(&f.blobs, "wiki/new/pic.png").await);
        assert!(!f.has(&f.pages, "wiki/old/a.md").await);
    }

    #[tokio::test]
    async fn renaming_across_a_store_boundary_copies_and_drops_the_original() {
        let f = fixture();
        f.put("wiki/notes.md", text("keep me")).await;
        f.routed
            .rename(
                f.project,
                &rel("wiki/notes.md"),
                &rel("raw/sources/notes.md"),
                &ctx(),
            )
            .await
            .unwrap();
        assert!(!f.has(&f.pages, "wiki/notes.md").await);
        let (body, _) = f
            .routed
            .read(f.project, &rel("raw/sources/notes.md"))
            .await
            .unwrap();
        assert_eq!(body.as_text(), Some("keep me"));

        // And back: bytes that are valid UTF-8 become a page again.
        f.routed
            .rename(
                f.project,
                &rel("raw/sources/notes.md"),
                &rel("wiki/back.md"),
                &ctx(),
            )
            .await
            .unwrap();
        assert!(f.has(&f.pages, "wiki/back.md").await);
        assert!(!f.has(&f.blobs, "raw/sources/notes.md").await);
    }

    #[tokio::test]
    async fn a_directory_move_that_changes_some_files_store_keeps_the_rest_together() {
        let f = fixture();
        f.put("wiki/old/a.md", text("page")).await;
        f.put("wiki/old/pic.png", Body::Bytes(vec![1])).await;
        // Under `wiki/media` the page becomes a blob; the picture stays one.
        f.routed
            .rename(f.project, &rel("wiki/old"), &rel("wiki/media/old"), &ctx())
            .await
            .unwrap();
        assert!(f.has(&f.blobs, "wiki/media/old/a.md").await);
        assert!(f.has(&f.blobs, "wiki/media/old/pic.png").await);
        assert!(!f.has(&f.pages, "wiki/old/a.md").await);
        assert!(!f.has(&f.blobs, "wiki/old/pic.png").await);
    }

    #[tokio::test]
    async fn a_failed_rename_puts_back_what_it_already_moved() {
        let f = fixture();
        f.put("raw/a.md", text("fine as a page")).await;
        f.put("raw/b.md", Body::Bytes(vec![0xff, 0xfe])).await;
        // `a.md` moves first and succeeds; `b.md` is not UTF-8 so it cannot
        // become a page, and the whole move is undone.
        let err = f
            .routed
            .rename(f.project, &rel("raw"), &rel("wiki/raw"), &ctx())
            .await
            .unwrap_err();
        assert!(matches!(err, StoreError::InvalidPath { .. }), "{err:?}");
        assert!(f.has(&f.blobs, "raw/a.md").await);
        assert!(f.has(&f.blobs, "raw/b.md").await);
        assert!(!f.has(&f.pages, "wiki/raw/a.md").await);
        assert!(!f.has(&f.blobs, "wiki/raw/b.md").await);
    }

    #[tokio::test]
    async fn rename_refuses_an_occupied_target_in_the_other_store() {
        let f = fixture();
        f.put("wiki/a.md", text("page")).await;
        f.put("wiki/media/x/pic.png", Body::Bytes(vec![1])).await;
        assert_eq!(
            f.routed
                .rename(f.project, &rel("wiki/a.md"), &rel("wiki/media/x"), &ctx())
                .await,
            Err(StoreError::AlreadyExists("wiki/media/x".into()))
        );
        assert!(f.has(&f.pages, "wiki/a.md").await);
    }

    #[tokio::test]
    async fn files_do_not_move_between_state_and_content() {
        let f = fixture();
        f.put("wiki/a.md", text("page")).await;
        let err = f
            .routed
            .rename(f.project, &rel("wiki/a.md"), &rel(".llm-wiki/a.md"), &ctx())
            .await
            .unwrap_err();
        assert!(matches!(err, StoreError::InvalidPath { .. }), "{err:?}");
    }

    #[tokio::test]
    async fn a_page_and_a_blob_cannot_shadow_each_other() {
        let f = fixture();
        // A blob file named `wiki/doc` blocks pages beneath it.
        f.put("wiki/doc", Body::Bytes(vec![1])).await;
        assert!(matches!(
            f.routed
                .write(
                    f.project,
                    &rel("wiki/doc/a.md"),
                    text("x"),
                    Precondition::Any,
                    &ctx()
                )
                .await,
            Err(StoreError::AlreadyExists(_))
        ));
        // A page named `wiki/b.md` blocks blobs beneath it, and the other way round.
        f.put("wiki/b.md", text("page")).await;
        assert!(matches!(
            f.routed
                .write(
                    f.project,
                    &rel("wiki/b.md/pic.png"),
                    Body::Bytes(vec![1]),
                    Precondition::Any,
                    &ctx()
                )
                .await,
            Err(StoreError::AlreadyExists(_))
        ));
        f.put("wiki/c.md/pic.png", Body::Bytes(vec![1])).await;
        assert!(matches!(
            f.routed
                .write(
                    f.project,
                    &rel("wiki/c.md"),
                    text("x"),
                    Precondition::Any,
                    &ctx()
                )
                .await,
            Err(StoreError::AlreadyExists(_))
        ));
    }

    #[tokio::test]
    async fn state_paths_need_a_state_store() {
        let routed = RoutedFiles::new(Arc::new(MemoryStore::new()), Arc::new(MemoryStore::new()));
        let project = ProjectId(uuid::Uuid::new_v4());
        let path = rel(".llm-wiki/x.json");
        assert!(matches!(
            routed
                .write(project, &path, text("{}"), Precondition::Any, &ctx())
                .await,
            Err(StoreError::Backend(_))
        ));
        assert!(routed.stat(project, &path).await.unwrap().is_none());
        assert!(routed
            .list(project, &rel(".llm-wiki"), true)
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn streaming_is_only_for_blob_paths() {
        let f = fixture();
        let empty: ByteStream = Box::pin(futures::stream::empty());
        let err = f
            .routed
            .put_stream(
                f.project,
                &rel("wiki/a.md"),
                empty,
                Precondition::Any,
                &ctx(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, StoreError::InvalidPath { .. }), "{err:?}");
        let err = f
            .routed
            .get_stream(f.project, &rel("raw/sources/s.pdf"))
            .await
            .map(|_| ())
            .unwrap_err();
        assert!(matches!(err, StoreError::Backend(_)), "{err:?}");
    }

    /// The whole tree behind one store, run through the shared suite twice:
    /// once for each folder a store serves.
    /// One test at a time talks to the object store: the tests reach it
    /// through a single `kubectl port-forward`, which times out when a few
    /// dozen of them upload at once.
    static OBJECT_STORE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    struct RoutedHarness {
        routed: RoutedFiles,
        root: &'static str,
        versioned: bool,
        pool: Option<PgPool>,
        _serial: Option<tokio::sync::MutexGuard<'static, ()>>,
    }

    #[async_trait]
    impl Harness for RoutedHarness {
        fn store(&self) -> &dyn ProjectFiles {
            &self.routed
        }

        async fn new_project(&self) -> ProjectId {
            let Some(pool) = &self.pool else {
                return ProjectId(uuid::Uuid::new_v4());
            };
            let id: uuid::Uuid = sqlx::query_scalar(
                "INSERT INTO projects (name) VALUES ('conformance') RETURNING id",
            )
            .fetch_one(pool)
            .await
            .unwrap();
            ProjectId(id)
        }

        fn root(&self) -> &'static str {
            self.root
        }

        fn versioned(&self) -> bool {
            self.versioned
        }
    }

    fn memory_harness(root: &'static str) -> Option<RoutedHarness> {
        Some(RoutedHarness {
            routed: RoutedFiles::new(Arc::new(MemoryStore::new()), Arc::new(MemoryStore::new()))
                .with_state(Arc::new(MemoryStore::new())),
            root,
            versioned: true,
            pool: None,
            _serial: None,
        })
    }

    async fn pg_harness(root: &'static str) -> Option<RoutedHarness> {
        pg_harness_over(root, Arc::new(InMemory::new())).await
    }

    /// Blob contents in the MinIO bucket `make test-pg` provides.
    async fn s3_harness(root: &'static str) -> Option<RoutedHarness> {
        let config = crate::S3Config::from_env("LLM_WIKI_TEST_S3").unwrap()?;
        let serial = OBJECT_STORE.lock().await;
        let mut harness = pg_harness_over(root, config.build().unwrap()).await?;
        harness._serial = Some(serial);
        Some(harness)
    }

    async fn pg_harness_over(
        root: &'static str,
        objects: Arc<dyn object_store::ObjectStore>,
    ) -> Option<RoutedHarness> {
        let url = std::env::var("LLM_WIKI_TEST_DATABASE_URL").ok()?;
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .connect(&url)
            .await
            .unwrap();
        crate::migrate(&pool).await.unwrap();
        let blobs = Arc::new(PgBlobs::new(pool.clone(), objects));
        Some(RoutedHarness {
            routed: RoutedFiles::new(Arc::new(PgFiles::new(pool.clone())), blobs.clone())
                .with_state(Arc::new(PgState::new(pool.clone())))
                .with_blob_streams(blobs),
            root,
            // Pages and state files carry versions; blobs do not.
            versioned: !matches!(root, "raw/sources" | "wiki/media"),
            pool: Some(pool),
            _serial: None,
        })
    }

    mod memory_wiki {
        use super::*;
        conformance::conformance_tests!(memory_harness("wiki"));
    }

    mod memory_raw {
        use super::*;
        conformance::conformance_tests!(memory_harness("raw/sources"));
    }

    mod postgres_wiki {
        use super::*;
        conformance::conformance_tests!(pg_harness("wiki").await);
    }

    mod memory_state {
        use super::*;
        conformance::conformance_tests!(memory_harness(".llm-wiki/data"));
    }

    mod postgres_state {
        use super::*;
        conformance::conformance_tests!(pg_harness(".llm-wiki/data").await);
    }

    mod postgres_raw {
        use super::*;
        conformance::conformance_tests!(pg_harness("raw/sources").await);
    }

    mod minio_raw {
        use super::*;
        conformance::conformance_tests!(s3_harness("raw/sources").await);
    }

    mod minio_wiki_media {
        use super::*;
        conformance::conformance_tests!(s3_harness("wiki/media").await);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_large_upload_streams_through_the_routed_tree() {
        let Some(h) = pg_harness("raw/sources").await else {
            return;
        };
        let p = h.new_project().await;
        let path = rel("raw/sources/big.bin");
        let parts: Vec<Result<bytes::Bytes, StoreError>> = (0..3u8)
            .map(|n| Ok(bytes::Bytes::from(vec![n; 5])))
            .collect();
        let meta = h
            .routed
            .put_stream(
                p,
                &path,
                Box::pin(futures::stream::iter(parts)),
                Precondition::Any,
                &ctx(),
            )
            .await
            .unwrap();
        assert_eq!(meta.size, 15);
        let (_, got) = h.routed.get_stream(p, &path).await.unwrap();
        assert_eq!(got.size, 15);
        assert_eq!(
            h.routed.list(p, &rel("raw"), true).await.unwrap()[0].path,
            path
        );
    }

    /// Three parts of 8 MiB go to a real S3 server as a multipart upload.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_multipart_upload_to_minio_round_trips_with_its_hash() {
        use futures::TryStreamExt;
        use sha2::{Digest, Sha256};

        let Some(h) = s3_harness("raw/sources").await else {
            return;
        };
        let p = h.new_project().await;
        let path = rel("raw/sources/big.bin");
        let chunk = |n: u8| bytes::Bytes::from(vec![n; 5 * 1024 * 1024]);
        let chunks = [chunk(1), chunk(2), chunk(3), chunk(4)];
        let mut hasher = Sha256::new();
        for part in &chunks {
            hasher.update(part);
        }
        let parts: Vec<Result<bytes::Bytes, StoreError>> = chunks.into_iter().map(Ok).collect();
        let meta = h
            .routed
            .put_stream(
                p,
                &path,
                Box::pin(futures::stream::iter(parts)),
                Precondition::Any,
                &ctx(),
            )
            .await
            .unwrap();
        assert_eq!(meta.size, 20 * 1024 * 1024);

        let (stream, got) = h.routed.get_stream(p, &path).await.unwrap();
        assert_eq!(got.size, meta.size);
        let mut read_hash = Sha256::new();
        let mut total = 0;
        let mut stream = stream;
        while let Some(part) = stream.try_next().await.unwrap() {
            total += part.len();
            read_hash.update(&part);
        }
        assert_eq!(total as u64, meta.size);
        assert_eq!(read_hash.finalize(), hasher.finalize());
    }
}
