use std::collections::BTreeMap;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::listing::{dir_meta, list_entries, FileInfo};
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
            let mut stored = files.remove(&key).expect("key came from the map");
            // A move is a change: it gets its own version, like any other write.
            stored.version += 1;
            stored.modified = Utc::now();
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
        Ok(newest.map(|modified| dir_meta(path.clone(), modified)))
    }

    async fn list(
        &self,
        project: ProjectId,
        dir: &RelPath,
        recursive: bool,
    ) -> Result<Vec<Meta>, StoreError> {
        let files = self.lock();
        list_entries(
            dir,
            recursive,
            files
                .iter()
                .filter(|((p, _), _)| *p == project.0)
                .map(|((_, path), stored)| {
                    (
                        path.clone(),
                        FileInfo {
                            size: stored.body.len() as u64,
                            modified: stored.modified,
                            version: Some(stored.version),
                        },
                    )
                }),
        )
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;

    use super::*;
    use crate::conformance::{self, Harness};

    struct MemoryHarness(MemoryStore);

    #[async_trait]
    impl Harness for MemoryHarness {
        fn store(&self) -> &dyn ProjectFiles {
            &self.0
        }

        async fn new_project(&self) -> ProjectId {
            ProjectId(uuid::Uuid::new_v4())
        }

        fn root(&self) -> &'static str {
            "wiki"
        }

        fn versioned(&self) -> bool {
            true
        }
    }

    conformance::conformance_tests!(Some(MemoryHarness(MemoryStore::new())));
}
