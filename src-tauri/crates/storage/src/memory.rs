use std::collections::BTreeMap;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::store::{Body, EntryKind, Meta, Precondition, ProjectFiles, ProjectId, WriteCtx};
use crate::{RelPath, StoreError};

#[derive(Clone)]
struct Stored {
    body: Body,
    version: u32,
    modified: DateTime<Utc>,
}

/// In-process implementation for tests and for running the callers that
/// moved onto [`ProjectFiles`] without a database. It enforces the same
/// semantics the Postgres store must: implicit directories, version locking,
/// create-only writes.
#[derive(Default)]
pub struct MemoryStore {
    files: Mutex<BTreeMap<(uuid::Uuid, RelPath), Stored>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, BTreeMap<(uuid::Uuid, RelPath), Stored>> {
        self.files.lock().expect("memory store lock poisoned")
    }
}

fn file_meta(path: &RelPath, stored: &Stored) -> Meta {
    Meta {
        path: path.clone(),
        kind: EntryKind::File,
        size: stored.body.len() as u64,
        modified: stored.modified,
        version: Some(stored.version),
    }
}

#[async_trait]
impl ProjectFiles for MemoryStore {
    async fn read(&self, project: ProjectId, path: &RelPath) -> Result<(Body, Meta), StoreError> {
        let files = self.lock();
        let stored = files
            .get(&(project.0, path.clone()))
            .ok_or_else(|| StoreError::not_found(path))?;
        Ok((stored.body.clone(), file_meta(path, stored)))
    }

    async fn write(
        &self,
        project: ProjectId,
        path: &RelPath,
        body: Body,
        precondition: Precondition,
        _ctx: &WriteCtx,
    ) -> Result<Meta, StoreError> {
        if path.is_root() {
            return Err(StoreError::InvalidPath {
                path: String::new(),
                reason: "cannot write the root",
            });
        }
        let mut files = self.lock();
        let key = (project.0, path.clone());
        // A file cannot be written where a directory already is, or beneath a file.
        let shadows_dir = files
            .range((project.0, path.clone())..)
            .take_while(|((p, other), _)| *p == project.0 && other.is_within(path))
            .any(|((_, other), _)| other != path);
        let under_file = {
            let mut cursor = path.parent();
            let mut found = false;
            while let Some(dir) = cursor {
                if !dir.is_root() && files.contains_key(&(project.0, dir.clone())) {
                    found = true;
                    break;
                }
                cursor = dir.parent();
            }
            found
        };
        if shadows_dir || under_file {
            return Err(StoreError::already_exists(path));
        }
        let current = files.get(&key).map(|stored| stored.version);
        match (precondition, current) {
            (Precondition::CreateOnly, Some(_)) => return Err(StoreError::already_exists(path)),
            (Precondition::Version(expected), Some(actual)) if expected != actual => {
                return Err(StoreError::VersionConflict {
                    path: path.to_string(),
                    current: actual,
                })
            }
            (Precondition::Version(_), None) => return Err(StoreError::not_found(path)),
            _ => {}
        }
        let stored = Stored {
            body,
            version: current.map_or(1, |version| version + 1),
            modified: Utc::now(),
        };
        let meta = file_meta(path, &stored);
        files.insert(key, stored);
        Ok(meta)
    }

    async fn delete(
        &self,
        project: ProjectId,
        path: &RelPath,
        _ctx: &WriteCtx,
    ) -> Result<usize, StoreError> {
        let mut files = self.lock();
        let doomed: Vec<_> = files
            .keys()
            .filter(|(p, other)| *p == project.0 && other.is_within(path))
            .cloned()
            .collect();
        if doomed.is_empty() {
            return Err(StoreError::not_found(path));
        }
        for key in &doomed {
            files.remove(key);
        }
        Ok(doomed.len())
    }

    async fn rename(
        &self,
        project: ProjectId,
        from: &RelPath,
        to: &RelPath,
        _ctx: &WriteCtx,
    ) -> Result<(), StoreError> {
        let mut files = self.lock();
        let moving: Vec<_> = files
            .keys()
            .filter(|(p, other)| *p == project.0 && other.is_within(from))
            .cloned()
            .collect();
        if moving.is_empty() {
            return Err(StoreError::not_found(from));
        }
        if to.is_within(from) {
            return Err(StoreError::InvalidPath {
                path: to.to_string(),
                reason: "destination is inside the source",
            });
        }
        let taken = files
            .keys()
            .any(|(p, other)| *p == project.0 && (other.is_within(to) || to.is_within(other)));
        if taken {
            return Err(StoreError::already_exists(to));
        }
        for key in moving {
            let stored = files.remove(&key).expect("key came from the map");
            let suffix = key.1.as_str().strip_prefix(from.as_str()).unwrap_or("");
            let target = RelPath::parse(&format!("{to}{suffix}"))?;
            files.insert((project.0, target), stored);
        }
        Ok(())
    }

    async fn stat(&self, project: ProjectId, path: &RelPath) -> Result<Option<Meta>, StoreError> {
        let files = self.lock();
        if let Some(stored) = files.get(&(project.0, path.clone())) {
            return Ok(Some(file_meta(path, stored)));
        }
        let newest = files
            .iter()
            .filter(|((p, other), _)| *p == project.0 && other.is_within(path))
            .map(|(_, stored)| stored.modified)
            .max();
        Ok(newest.map(|modified| Meta {
            path: path.clone(),
            kind: EntryKind::Dir,
            size: 0,
            modified,
            version: None,
        }))
    }

    async fn list(
        &self,
        project: ProjectId,
        dir: &RelPath,
        recursive: bool,
    ) -> Result<Vec<Meta>, StoreError> {
        let files = self.lock();
        let mut out: BTreeMap<RelPath, Meta> = BTreeMap::new();
        for ((p, path), stored) in files.iter() {
            if *p != project.0 || path == dir || !path.is_within(dir) {
                continue;
            }
            if recursive {
                out.insert(path.clone(), file_meta(path, stored));
                continue;
            }
            // Direct child: the file itself, or the directory that contains it.
            let rest = path.as_str()[dir.as_str().len()..].trim_start_matches('/');
            match rest.split_once('/') {
                None => {
                    out.insert(path.clone(), file_meta(path, stored));
                }
                Some((child, _)) => {
                    let child = dir.join(child)?;
                    out.entry(child.clone())
                        .and_modify(|meta| meta.modified = meta.modified.max(stored.modified))
                        .or_insert(Meta {
                            path: child,
                            kind: EntryKind::Dir,
                            size: 0,
                            modified: stored.modified,
                            version: None,
                        });
                }
            }
        }
        Ok(out.into_values().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> ProjectId {
        ProjectId(uuid::Uuid::new_v4())
    }

    fn rel(path: &str) -> RelPath {
        RelPath::parse(path).unwrap()
    }

    fn text(content: &str) -> Body {
        Body::Text(content.to_string())
    }

    async fn put(store: &MemoryStore, p: ProjectId, path: &str, content: &str) -> Meta {
        store
            .write(
                p,
                &rel(path),
                text(content),
                Precondition::Any,
                &WriteCtx::new("test"),
            )
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn write_then_read_round_trips_and_versions_count_up() {
        let store = MemoryStore::new();
        let p = project();
        assert_eq!(put(&store, p, "wiki/a.md", "one").await.version, Some(1));
        assert_eq!(put(&store, p, "wiki/a.md", "two").await.version, Some(2));
        let (body, meta) = store.read(p, &rel("wiki/a.md")).await.unwrap();
        assert_eq!(body, text("two"));
        assert_eq!(meta.version, Some(2));
        assert_eq!(meta.size, 3);
    }

    #[tokio::test]
    async fn projects_are_isolated() {
        let store = MemoryStore::new();
        let (a, b) = (project(), project());
        put(&store, a, "wiki/a.md", "secret").await;
        assert!(matches!(
            store.read(b, &rel("wiki/a.md")).await,
            Err(StoreError::NotFound(_))
        ));
        assert!(store
            .list(b, &RelPath::root(), true)
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn create_only_refuses_to_overwrite() {
        let store = MemoryStore::new();
        let p = project();
        let ctx = WriteCtx::new("agent");
        store
            .write(
                p,
                &rel("wiki/a.md"),
                text("kept"),
                Precondition::CreateOnly,
                &ctx,
            )
            .await
            .unwrap();
        let err = store
            .write(
                p,
                &rel("wiki/a.md"),
                text("clobber"),
                Precondition::CreateOnly,
                &ctx,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, StoreError::AlreadyExists(_)));
        assert_eq!(
            store.read(p, &rel("wiki/a.md")).await.unwrap().0,
            text("kept")
        );
    }

    #[tokio::test]
    async fn a_stale_writer_gets_a_version_conflict() {
        let store = MemoryStore::new();
        let p = project();
        let ctx = WriteCtx::new("edit");
        put(&store, p, "wiki/a.md", "v1").await;
        store
            .write(
                p,
                &rel("wiki/a.md"),
                text("mine"),
                Precondition::Version(1),
                &ctx,
            )
            .await
            .unwrap();
        let err = store
            .write(
                p,
                &rel("wiki/a.md"),
                text("stale"),
                Precondition::Version(1),
                &ctx,
            )
            .await
            .unwrap_err();
        assert_eq!(
            err,
            StoreError::VersionConflict {
                path: "wiki/a.md".into(),
                current: 2
            }
        );
    }

    #[tokio::test]
    async fn versioned_write_to_a_missing_file_is_not_found() {
        let store = MemoryStore::new();
        let err = store
            .write(
                project(),
                &rel("wiki/a.md"),
                text("x"),
                Precondition::Version(1),
                &WriteCtx::new("edit"),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, StoreError::NotFound(_)));
    }

    #[tokio::test]
    async fn directories_exist_only_while_they_hold_files() {
        let store = MemoryStore::new();
        let p = project();
        put(&store, p, "wiki/entities/a.md", "a").await;
        let dir = store.stat(p, &rel("wiki/entities")).await.unwrap().unwrap();
        assert_eq!(dir.kind, EntryKind::Dir);
        store
            .delete(p, &rel("wiki/entities/a.md"), &WriteCtx::new("edit"))
            .await
            .unwrap();
        assert_eq!(store.stat(p, &rel("wiki/entities")).await.unwrap(), None);
        assert_eq!(store.stat(p, &rel("wiki")).await.unwrap(), None);
    }

    #[tokio::test]
    async fn list_returns_direct_children_with_synthesised_directories() {
        let store = MemoryStore::new();
        let p = project();
        put(&store, p, "wiki/index.md", "i").await;
        put(&store, p, "wiki/entities/a.md", "a").await;
        put(&store, p, "wiki/entities/b.md", "b").await;
        put(&store, p, "wiki2/x.md", "x").await;
        let names: Vec<_> = store
            .list(p, &rel("wiki"), false)
            .await
            .unwrap()
            .iter()
            .map(|m| (m.path.to_string(), m.kind))
            .collect();
        assert_eq!(
            names,
            vec![
                ("wiki/entities".to_string(), EntryKind::Dir),
                ("wiki/index.md".to_string(), EntryKind::File),
            ]
        );
        let all = store.list(p, &rel("wiki"), true).await.unwrap();
        assert_eq!(all.len(), 3);
        assert!(all.iter().all(|m| m.kind == EntryKind::File));
    }

    #[tokio::test]
    async fn delete_removes_a_whole_subtree_and_counts_it() {
        let store = MemoryStore::new();
        let p = project();
        put(&store, p, "raw/sources/a.pdf", "a").await;
        put(&store, p, "raw/sources/sub/b.pdf", "b").await;
        put(&store, p, "raw/other.txt", "c").await;
        let removed = store
            .delete(p, &rel("raw/sources"), &WriteCtx::new("edit"))
            .await
            .unwrap();
        assert_eq!(removed, 2);
        assert!(store
            .stat(p, &rel("raw/other.txt"))
            .await
            .unwrap()
            .is_some());
        assert!(matches!(
            store
                .delete(p, &rel("raw/sources"), &WriteCtx::new("edit"))
                .await,
            Err(StoreError::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn rename_moves_a_directory_and_refuses_an_occupied_destination() {
        let store = MemoryStore::new();
        let p = project();
        let ctx = WriteCtx::new("edit");
        put(&store, p, "raw/old/a.md", "a").await;
        put(&store, p, "raw/old/deep/b.md", "b").await;
        put(&store, p, "raw/taken/c.md", "c").await;
        assert!(matches!(
            store
                .rename(p, &rel("raw/old"), &rel("raw/taken"), &ctx)
                .await,
            Err(StoreError::AlreadyExists(_))
        ));
        store
            .rename(p, &rel("raw/old"), &rel("raw/new"), &ctx)
            .await
            .unwrap();
        assert!(store
            .stat(p, &rel("raw/new/deep/b.md"))
            .await
            .unwrap()
            .is_some());
        assert_eq!(store.stat(p, &rel("raw/old")).await.unwrap(), None);
    }

    #[tokio::test]
    async fn a_file_cannot_shadow_a_directory_or_sit_beneath_a_file() {
        let store = MemoryStore::new();
        let p = project();
        let ctx = WriteCtx::new("edit");
        put(&store, p, "wiki/entities/a.md", "a").await;
        assert!(store
            .write(p, &rel("wiki/entities"), text("x"), Precondition::Any, &ctx)
            .await
            .is_err());
        assert!(store
            .write(
                p,
                &rel("wiki/entities/a.md/child"),
                text("x"),
                Precondition::Any,
                &ctx
            )
            .await
            .is_err());
    }
}
