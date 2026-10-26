use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use object_store::path::Path as ObjectPath;
use object_store::{ObjectStore, PutPayload, WriteMultipart};
use sha2::{Digest, Sha256};
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::listing::{dir_meta, list_entries, FileInfo};
use crate::route::blob_kind;
use crate::sql::{any_live, backend, below, collides, is_unique_violation, WriteFailure};
use crate::store::{
    BlobStreams, Body, ByteStream, EntryKind, Meta, Precondition, ProjectFiles, ProjectId, WriteCtx,
};
use crate::{RelPath, StoreError};

/// Multipart upload part size. S3 requires at least 5 MiB for all but the last part.
const PART_BYTES: usize = 8 * 1024 * 1024;

/// Binary files: object bytes in an S3-compatible store (MinIO), described by
/// rows of the `blobs` table.
///
/// Objects are immutable. The company MinIO allows at most 10 versions per
/// key, so a changed file is a new object under a new key (`<project>/<id>`)
/// and the old row is soft-deleted; a key is never written twice. Renaming
/// only updates the row. Old objects are removed later by [`purge_deleted`].
///
/// [`purge_deleted`]: PgBlobs::purge_deleted
#[derive(Clone)]
pub struct PgBlobs {
    pool: PgPool,
    objects: Arc<dyn ObjectStore>,
}

fn object_key(project: ProjectId, id: Uuid) -> ObjectPath {
    ObjectPath::from(format!("{}/{id}", project.0))
}

fn object_error(error: object_store::Error) -> StoreError {
    StoreError::Backend(format!("object store: {error}"))
}

fn file_meta(path: RelPath, row: &PgRow) -> Result<Meta, StoreError> {
    Ok(Meta {
        path,
        kind: EntryKind::File,
        size: row.try_get::<i64, _>("size_bytes").map_err(backend)? as u64,
        modified: row
            .try_get::<DateTime<Utc>, _>("created_at")
            .map_err(backend)?,
        version: None,
    })
}

struct Upload {
    id: Uuid,
    size: u64,
    sha256: Vec<u8>,
}

impl PgBlobs {
    pub fn new(pool: PgPool, objects: Arc<dyn ObjectStore>) -> Self {
        Self { pool, objects }
    }

    /// Deletes the objects of blobs soft-deleted more than `grace` ago, then
    /// their rows. Returns how many were removed. Run periodically.
    pub async fn purge_deleted(&self, grace: Duration) -> Result<usize, StoreError> {
        self.purge(None, grace).await
    }

    /// [`purge_deleted`](Self::purge_deleted) limited to one project.
    pub async fn purge_project(
        &self,
        project: ProjectId,
        grace: Duration,
    ) -> Result<usize, StoreError> {
        self.purge(Some(project), grace).await
    }

    async fn purge(
        &self,
        project: Option<ProjectId>,
        grace: Duration,
    ) -> Result<usize, StoreError> {
        let rows = sqlx::query(
            "SELECT id, project_id FROM blobs
             WHERE deleted_at IS NOT NULL AND deleted_at < now() - make_interval(secs => $1)
               AND ($2::uuid IS NULL OR project_id = $2)
             ORDER BY deleted_at LIMIT 500",
        )
        .bind(grace.as_secs_f64())
        .bind(project.map(|p| p.0))
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        for row in &rows {
            let id: Uuid = row.try_get("id").map_err(backend)?;
            let project = ProjectId(row.try_get("project_id").map_err(backend)?);
            match self.objects.delete(&object_key(project, id)).await {
                Ok(()) | Err(object_store::Error::NotFound { .. }) => {}
                Err(error) => return Err(object_error(error)),
            }
            sqlx::query("DELETE FROM blobs WHERE id = $1")
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(backend)?;
        }
        Ok(rows.len())
    }

    /// Fails early, before any bytes are uploaded, when the write cannot succeed.
    async fn precheck(
        &self,
        project: ProjectId,
        path: &RelPath,
        precondition: Precondition,
    ) -> Result<(), StoreError> {
        if path.is_root() {
            return Err(StoreError::InvalidPath {
                path: String::new(),
                reason: "cannot write the root",
            });
        }
        match precondition {
            Precondition::Version(_) => Err(StoreError::Backend(format!(
                "{path} is a blob; blobs have no version to check"
            ))),
            Precondition::CreateOnly if self.stat(project, path).await?.is_some() => {
                Err(StoreError::already_exists(path))
            }
            _ => Ok(()),
        }
    }

    /// Points `path` at the freshly uploaded object, retiring the row it replaces.
    async fn commit(
        &self,
        project: ProjectId,
        path: &RelPath,
        upload: &Upload,
        precondition: Precondition,
        ctx: &WriteCtx,
    ) -> Result<Meta, WriteFailure> {
        let mut tx = self.pool.begin().await?;
        if collides(&mut tx, "blobs", project, path).await? {
            return Err(StoreError::already_exists(path).into());
        }
        let current: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM blobs
             WHERE project_id = $1 AND path = $2 AND deleted_at IS NULL FOR UPDATE",
        )
        .bind(project.0)
        .bind(path.as_str())
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(old) = current {
            if precondition == Precondition::CreateOnly {
                return Err(StoreError::already_exists(path).into());
            }
            sqlx::query("UPDATE blobs SET deleted_at = now() WHERE id = $1")
                .bind(old)
                .execute(&mut *tx)
                .await?;
        }
        let created_at: DateTime<Utc> = sqlx::query_scalar(
            "INSERT INTO blobs (id, project_id, path, kind, size_bytes, sha256, uploaded_by)
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING created_at",
        )
        .bind(upload.id)
        .bind(project.0)
        .bind(path.as_str())
        .bind(blob_kind(path))
        .bind(upload.size as i64)
        .bind(&upload.sha256)
        .bind(ctx.actor)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Meta {
            path: path.clone(),
            kind: EntryKind::File,
            size: upload.size,
            modified: created_at,
            version: None,
        })
    }

    /// Commits the row for an uploaded object; if that fails the object is
    /// removed again so nothing is left behind.
    async fn finish(
        &self,
        project: ProjectId,
        path: &RelPath,
        upload: Upload,
        precondition: Precondition,
        ctx: &WriteCtx,
    ) -> Result<Meta, StoreError> {
        let mut outcome = Err(StoreError::already_exists(path));
        // Two writers creating the same path race to the unique index; the
        // loser re-runs and then sees the winner's row.
        for attempt in 0..3 {
            match self.commit(project, path, &upload, precondition, ctx).await {
                Err(WriteFailure::Db(error)) if is_unique_violation(&error) => {
                    if precondition == Precondition::CreateOnly || attempt == 2 {
                        break;
                    }
                }
                Err(WriteFailure::Db(error)) => {
                    outcome = Err(backend(error));
                    break;
                }
                Err(WriteFailure::Store(error)) => {
                    outcome = Err(error);
                    break;
                }
                Ok(meta) => {
                    outcome = Ok(meta);
                    break;
                }
            }
        }
        if outcome.is_err() {
            let _ = self.objects.delete(&object_key(project, upload.id)).await;
        }
        outcome
    }

    async fn live_row(
        &self,
        project: ProjectId,
        path: &RelPath,
    ) -> Result<Option<(Uuid, PgRow)>, StoreError> {
        let row = sqlx::query(
            "SELECT id, size_bytes, created_at FROM blobs
             WHERE project_id = $1 AND path = $2 AND deleted_at IS NULL",
        )
        .bind(project.0)
        .bind(path.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(backend)?;
        row.map(|row| Ok((row.try_get("id").map_err(backend)?, row)))
            .transpose()
    }
}

#[async_trait]
impl ProjectFiles for PgBlobs {
    async fn read(&self, project: ProjectId, path: &RelPath) -> Result<(Body, Meta), StoreError> {
        let (id, row) = self
            .live_row(project, path)
            .await?
            .ok_or_else(|| StoreError::not_found(path))?;
        let bytes = self
            .objects
            .get(&object_key(project, id))
            .await
            .map_err(object_error)?
            .bytes()
            .await
            .map_err(object_error)?;
        Ok((Body::Bytes(bytes.to_vec()), file_meta(path.clone(), &row)?))
    }

    async fn write(
        &self,
        project: ProjectId,
        path: &RelPath,
        body: Body,
        precondition: Precondition,
        ctx: &WriteCtx,
    ) -> Result<Meta, StoreError> {
        self.precheck(project, path, precondition).await?;
        let bytes = body.into_bytes();
        let upload = Upload {
            id: Uuid::new_v4(),
            size: bytes.len() as u64,
            sha256: Sha256::digest(&bytes).to_vec(),
        };
        self.objects
            .put(&object_key(project, upload.id), PutPayload::from(bytes))
            .await
            .map_err(object_error)?;
        self.finish(project, path, upload, precondition, ctx).await
    }

    async fn delete(
        &self,
        project: ProjectId,
        path: &RelPath,
        _ctx: &WriteCtx,
    ) -> Result<usize, StoreError> {
        let result = sqlx::query(
            "UPDATE blobs SET deleted_at = now()
             WHERE project_id = $1 AND deleted_at IS NULL
               AND (path = $2 OR path LIKE $3 ESCAPE '\\')",
        )
        .bind(project.0)
        .bind(path.as_str())
        .bind(below(path))
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        match result.rows_affected() {
            0 => Err(StoreError::not_found(path)),
            removed => Ok(removed as usize),
        }
    }

    async fn rename(
        &self,
        project: ProjectId,
        from: &RelPath,
        to: &RelPath,
        _ctx: &WriteCtx,
    ) -> Result<(), StoreError> {
        if to.is_within(from) {
            return Err(StoreError::InvalidPath {
                path: to.to_string(),
                reason: "destination is inside the source",
            });
        }
        let mut tx = self.pool.begin().await.map_err(backend)?;
        let moving: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM blobs
             WHERE project_id = $1 AND deleted_at IS NULL
               AND (path = $2 OR path LIKE $3 ESCAPE '\\')
             FOR UPDATE",
        )
        .bind(project.0)
        .bind(from.as_str())
        .bind(below(from))
        .fetch_all(&mut *tx)
        .await
        .map_err(backend)?;
        if moving.is_empty() {
            return Err(StoreError::not_found(from));
        }
        let mut occupied: Vec<String> = to.ancestors().iter().map(ToString::to_string).collect();
        occupied.push(to.to_string());
        if any_live(&mut tx, "blobs", project, &occupied, Some(&below(to))).await? {
            return Err(StoreError::already_exists(to));
        }
        // Only rows change: the objects are addressed by id, not by path.
        sqlx::query("UPDATE blobs SET path = $2 || substr(path, $3) WHERE id = ANY($1)")
            .bind(&moving)
            .bind(to.as_str())
            .bind(from.as_str().chars().count() as i32 + 1)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        tx.commit().await.map_err(backend)?;
        Ok(())
    }

    async fn stat(&self, project: ProjectId, path: &RelPath) -> Result<Option<Meta>, StoreError> {
        if let Some((_, row)) = self.live_row(project, path).await? {
            return Ok(Some(file_meta(path.clone(), &row)?));
        }
        let newest: Option<DateTime<Utc>> = sqlx::query_scalar(
            "SELECT max(created_at) FROM blobs
             WHERE project_id = $1 AND deleted_at IS NULL AND path LIKE $2 ESCAPE '\\'",
        )
        .bind(project.0)
        .bind(below(path))
        .fetch_one(&self.pool)
        .await
        .map_err(backend)?;
        Ok(newest.map(|modified| dir_meta(path.clone(), modified)))
    }

    async fn list(
        &self,
        project: ProjectId,
        dir: &RelPath,
        recursive: bool,
    ) -> Result<Vec<Meta>, StoreError> {
        let rows = sqlx::query(
            "SELECT path, size_bytes, created_at FROM blobs
             WHERE project_id = $1 AND deleted_at IS NULL AND path LIKE $2 ESCAPE '\\'",
        )
        .bind(project.0)
        .bind(below(dir))
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        let files = rows
            .iter()
            .map(|row| {
                let path = RelPath::parse(&row.try_get::<String, _>("path").map_err(backend)?)?;
                let meta = file_meta(path, row)?;
                Ok((
                    meta.path,
                    FileInfo {
                        size: meta.size,
                        modified: meta.modified,
                        version: None,
                    },
                ))
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        list_entries(dir, recursive, files)
    }
}

#[async_trait]
impl BlobStreams for PgBlobs {
    async fn put_stream(
        &self,
        project: ProjectId,
        path: &RelPath,
        mut body: ByteStream,
        precondition: Precondition,
        ctx: &WriteCtx,
    ) -> Result<Meta, StoreError> {
        self.precheck(project, path, precondition).await?;
        let id = Uuid::new_v4();
        let key = object_key(project, id);
        let multipart = self
            .objects
            .put_multipart(&key)
            .await
            .map_err(object_error)?;
        let mut writer = WriteMultipart::new_with_chunk_size(multipart, PART_BYTES);
        let mut hasher = Sha256::new();
        let mut size = 0u64;
        while let Some(chunk) = body.next().await {
            match chunk {
                Ok(chunk) => {
                    hasher.update(&chunk);
                    size += chunk.len() as u64;
                    writer.write(&chunk);
                }
                Err(error) => {
                    let _ = writer.abort().await;
                    return Err(error);
                }
            }
        }
        writer.finish().await.map_err(object_error)?;
        let upload = Upload {
            id,
            size,
            sha256: hasher.finalize().to_vec(),
        };
        self.finish(project, path, upload, precondition, ctx).await
    }

    async fn get_stream(
        &self,
        project: ProjectId,
        path: &RelPath,
    ) -> Result<(ByteStream, Meta), StoreError> {
        let (id, row) = self
            .live_row(project, path)
            .await?
            .ok_or_else(|| StoreError::not_found(path))?;
        let meta = file_meta(path.clone(), &row)?;
        let stream = self
            .objects
            .get(&object_key(project, id))
            .await
            .map_err(object_error)?
            .into_stream()
            .map(|chunk| chunk.map_err(object_error))
            .boxed();
        Ok((stream, meta))
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use futures::{stream, TryStreamExt};
    use object_store::memory::InMemory;
    use sqlx::postgres::PgPoolOptions;

    use super::*;
    use crate::conformance::{self, Harness};

    struct BlobHarness {
        blobs: PgBlobs,
        objects: Arc<InMemory>,
    }

    #[async_trait]
    impl Harness for BlobHarness {
        fn store(&self) -> &dyn ProjectFiles {
            &self.blobs
        }

        async fn new_project(&self) -> ProjectId {
            let id: Uuid = sqlx::query_scalar(
                "INSERT INTO projects (name) VALUES ('conformance') RETURNING id",
            )
            .fetch_one(&self.blobs.pool)
            .await
            .unwrap();
            ProjectId(id)
        }

        fn root(&self) -> &'static str {
            "raw/sources"
        }

        fn versioned(&self) -> bool {
            false
        }
    }

    async fn harness() -> Option<BlobHarness> {
        let url = std::env::var("LLM_WIKI_TEST_DATABASE_URL").ok()?;
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .connect(&url)
            .await
            .unwrap();
        crate::migrate(&pool).await.unwrap();
        let objects = Arc::new(InMemory::new());
        Some(BlobHarness {
            blobs: PgBlobs::new(pool, objects.clone()),
            objects,
        })
    }

    conformance::conformance_tests!(harness().await);

    async fn object_count(h: &BlobHarness) -> usize {
        h.objects
            .list(None)
            .try_collect::<Vec<_>>()
            .await
            .unwrap()
            .len()
    }

    fn rel(path: &str) -> RelPath {
        RelPath::parse(path).unwrap()
    }

    fn chunks(parts: &[&[u8]]) -> ByteStream {
        let owned: Vec<Result<Bytes, StoreError>> = parts
            .iter()
            .map(|p| Ok(Bytes::copy_from_slice(p)))
            .collect();
        stream::iter(owned).boxed()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn overwriting_a_path_uses_a_new_key_and_never_rewrites_one() {
        let Some(h) = harness().await else { return };
        let p = h.new_project().await;
        let path = rel("raw/sources/report.pdf");
        for n in 1..=3 {
            h.blobs
                .write(
                    p,
                    &path,
                    Body::Bytes(vec![n; 4]),
                    Precondition::Any,
                    &WriteCtx::new("test"),
                )
                .await
                .unwrap();
        }
        // Three writes, three distinct objects: no key ever has a second version.
        assert_eq!(object_count(&h).await, 3);
        assert_eq!(
            h.blobs.read(p, &path).await.unwrap().0,
            Body::Bytes(vec![3; 4])
        );

        // Purging retires the two replaced objects and keeps the live one.
        assert_eq!(h.blobs.purge_project(p, Duration::ZERO).await.unwrap(), 2);
        assert_eq!(object_count(&h).await, 1);
        assert_eq!(
            h.blobs.read(p, &path).await.unwrap().0,
            Body::Bytes(vec![3; 4])
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn purge_respects_the_grace_period() {
        let Some(h) = harness().await else { return };
        let p = h.new_project().await;
        let path = rel("raw/sources/a.bin");
        let ctx = WriteCtx::new("test");
        h.blobs
            .write(p, &path, Body::Bytes(vec![1]), Precondition::Any, &ctx)
            .await
            .unwrap();
        h.blobs.delete(p, &path, &ctx).await.unwrap();
        assert_eq!(
            h.blobs
                .purge_project(p, Duration::from_secs(3600))
                .await
                .unwrap(),
            0
        );
        assert_eq!(object_count(&h).await, 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn rename_changes_the_row_not_the_object() {
        let Some(h) = harness().await else { return };
        let p = h.new_project().await;
        let ctx = WriteCtx::new("test");
        h.blobs
            .write(
                p,
                &rel("raw/sources/a.bin"),
                Body::Bytes(vec![7]),
                Precondition::Any,
                &ctx,
            )
            .await
            .unwrap();
        h.blobs
            .rename(
                p,
                &rel("raw/sources/a.bin"),
                &rel("raw/sources/b.bin"),
                &ctx,
            )
            .await
            .unwrap();
        assert_eq!(object_count(&h).await, 1);
        assert_eq!(
            h.blobs.read(p, &rel("raw/sources/b.bin")).await.unwrap().0,
            Body::Bytes(vec![7])
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn streams_round_trip_across_chunks_and_record_size_and_hash() {
        let Some(h) = harness().await else { return };
        let p = h.new_project().await;
        let path = rel("raw/sources/big.bin");
        let meta = h
            .blobs
            .put_stream(
                p,
                &path,
                chunks(&[b"hello ", b"streaming ", b"world"]),
                Precondition::Any,
                &WriteCtx::new("upload"),
            )
            .await
            .unwrap();
        assert_eq!(meta.size, 21);
        let sha: Vec<u8> = sqlx::query_scalar("SELECT sha256 FROM blobs WHERE project_id = $1")
            .bind(p.0)
            .fetch_one(&h.blobs.pool)
            .await
            .unwrap();
        assert_eq!(sha, Sha256::digest(b"hello streaming world").to_vec());

        let (stream, got) = h.blobs.get_stream(p, &path).await.unwrap();
        let joined: Vec<u8> = stream
            .try_fold(Vec::new(), |mut acc, part| async move {
                acc.extend_from_slice(&part);
                Ok(acc)
            })
            .await
            .unwrap();
        assert_eq!(joined, b"hello streaming world");
        assert_eq!(got.size, 21);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_refused_create_only_stream_uploads_nothing() {
        let Some(h) = harness().await else { return };
        let p = h.new_project().await;
        let path = rel("raw/sources/a.bin");
        let ctx = WriteCtx::new("test");
        h.blobs
            .write(p, &path, Body::Bytes(vec![1]), Precondition::Any, &ctx)
            .await
            .unwrap();
        let err = h
            .blobs
            .put_stream(p, &path, chunks(&[b"x"]), Precondition::CreateOnly, &ctx)
            .await
            .unwrap_err();
        assert!(matches!(err, StoreError::AlreadyExists(_)));
        assert_eq!(object_count(&h).await, 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_failed_stream_leaves_no_object_or_row() {
        let Some(h) = harness().await else { return };
        let p = h.new_project().await;
        let body: ByteStream = stream::iter(vec![
            Ok(Bytes::from_static(b"partial")),
            Err(StoreError::Backend("client went away".into())),
        ])
        .boxed();
        let err = h
            .blobs
            .put_stream(
                p,
                &rel("raw/sources/a.bin"),
                body,
                Precondition::Any,
                &WriteCtx::new("t"),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, StoreError::Backend(_)));
        assert_eq!(object_count(&h).await, 0);
        assert_eq!(
            h.blobs.stat(p, &rel("raw/sources/a.bin")).await.unwrap(),
            None
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn blobs_reject_version_preconditions() {
        let Some(h) = harness().await else { return };
        let p = h.new_project().await;
        let err = h
            .blobs
            .write(
                p,
                &rel("raw/sources/a.bin"),
                Body::Bytes(vec![1]),
                Precondition::Version(1),
                &WriteCtx::new("t"),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, StoreError::Backend(_)));
        assert_eq!(object_count(&h).await, 0);
    }
}
