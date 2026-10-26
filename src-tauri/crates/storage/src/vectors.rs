//! Chunk embeddings for semantic search (pgvector), one table per embedding
//! model (migration 0004). A Project's vectors are those of the model the
//! platform currently uses; switching models starts an empty table, which the
//! "re-index all pages" action fills.

use std::collections::BTreeMap;
use std::sync::Mutex;

use async_trait::async_trait;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::settings::{ServerSettings, LLM_MODELS_KEY};
use crate::sql::backend;
use crate::store::ProjectId;
use crate::{RelPath, StoreError};

/// One chunk of a page with its vector.
#[derive(Debug, Clone)]
pub struct ChunkInput {
    pub chunk_index: u32,
    pub chunk_text: String,
    pub heading_path: String,
    pub embedding: Vec<f32>,
}

/// A chunk found by similarity.
#[derive(Debug, Clone, PartialEq)]
pub struct ChunkHit {
    /// The page the chunk belongs to.
    pub path: String,
    pub chunk_index: u32,
    pub chunk_text: String,
    pub heading_path: String,
    /// Euclidean distance to the query; smaller is closer.
    pub distance: f32,
}

#[async_trait]
pub trait ChunkVectors: Send + Sync {
    /// Replaces the page's chunks. `fingerprint` records what they were
    /// computed from (see [`Self::revision_match`]); `None` forgets it.
    async fn upsert_chunks(
        &self,
        project: ProjectId,
        page: &RelPath,
        chunks: Vec<ChunkInput>,
        fingerprint: Option<&str>,
    ) -> Result<(), StoreError>;

    /// The `top_k` chunks closest to `query` among the Project's pages.
    async fn search_chunks(
        &self,
        project: ProjectId,
        query: &[f32],
        top_k: usize,
    ) -> Result<Vec<ChunkHit>, StoreError>;

    async fn delete_page(&self, project: ProjectId, page: &RelPath) -> Result<(), StoreError>;

    async fn count_chunks(&self, project: ProjectId) -> Result<usize, StoreError>;

    /// Drops every chunk of the Project for the model in use.
    async fn clear(&self, project: ProjectId) -> Result<(), StoreError>;

    /// How many chunks the page has, if they were computed from `fingerprint`.
    async fn revision_match(
        &self,
        project: ProjectId,
        page: &RelPath,
        fingerprint: &str,
    ) -> Result<Option<usize>, StoreError>;
}

/// Where the model in use comes from.
#[derive(Clone)]
pub enum ActiveModel {
    /// The first embedding model of the model catalogue (`llm.models`),
    /// read on every call so a change takes effect without a restart.
    Catalogue(ServerSettings),
    Fixed(String),
}

impl ActiveModel {
    async fn name(&self) -> Result<String, StoreError> {
        match self {
            Self::Fixed(name) => Ok(name.clone()),
            Self::Catalogue(settings) => settings
                .get(LLM_MODELS_KEY)
                .await?
                .and_then(|catalogue| first_embedding_model(&catalogue))
                .ok_or_else(|| {
                    StoreError::Backend("no embedding model is configured on this server".into())
                }),
        }
    }
}

/// `provider/model` of the first catalogue entry whose purpose is embedding.
fn first_embedding_model(catalogue: &serde_json::Value) -> Option<String> {
    catalogue.as_array()?.iter().find_map(|entry| {
        let text = |key: &str| entry.get(key)?.as_str().filter(|v| !v.is_empty());
        if text("purpose")? != "embedding" {
            return None;
        }
        Some(format!("{}/{}", text("provider")?, text("model")?))
    })
}

#[derive(Clone)]
pub struct PgVectors {
    pool: PgPool,
    model: ActiveModel,
}

/// pgvector text input: `[1,2,3]`. Rejects what the index cannot hold.
fn literal(embedding: &[f32]) -> Result<String, StoreError> {
    if embedding.is_empty() || embedding.iter().any(|v| !v.is_finite()) {
        return Err(StoreError::Backend(
            "an embedding must be non-empty and finite".into(),
        ));
    }
    let items: Vec<String> = embedding.iter().map(f32::to_string).collect();
    Ok(format!("[{}]", items.join(",")))
}

/// The pgvector type for a dimension, as `register_embedding_model` chose it.
fn column_type(dimensions: usize) -> &'static str {
    if dimensions <= 2000 {
        "vector"
    } else {
        "halfvec"
    }
}

impl PgVectors {
    pub fn new(pool: PgPool, model: ActiveModel) -> Self {
        Self { pool, model }
    }

    /// The table of the model in use, creating it on first use.
    async fn table(&self, dimensions: usize) -> Result<(String, &'static str), StoreError> {
        let model = self.model.name().await?;
        let id: i32 = sqlx::query_scalar("SELECT register_embedding_model($1, $2)")
            .bind(&model)
            .bind(dimensions as i32)
            .fetch_one(&self.pool)
            .await
            .map_err(backend)?;
        Ok((format!("chunks_m{id}"), column_type(dimensions)))
    }

    /// The table of the model in use if it has been created, with its
    /// dimension and model id.
    async fn existing(&self) -> Result<Option<(i32, String, usize)>, StoreError> {
        let model = self.model.name().await?;
        let row =
            sqlx::query("SELECT id, table_name, dimensions FROM embedding_models WHERE model = $1")
                .bind(model)
                .fetch_optional(&self.pool)
                .await
                .map_err(backend)?;
        row.map(|row| {
            Ok((
                row.try_get("id").map_err(backend)?,
                row.try_get("table_name").map_err(backend)?,
                row.try_get::<i32, _>("dimensions").map_err(backend)? as usize,
            ))
        })
        .transpose()
    }

    async fn page_id(
        &self,
        project: ProjectId,
        page: &RelPath,
    ) -> Result<Option<Uuid>, StoreError> {
        sqlx::query_scalar(
            "SELECT id FROM pages WHERE project_id = $1 AND path = $2 AND deleted_at IS NULL",
        )
        .bind(project.0)
        .bind(page.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(backend)
    }
}

#[async_trait]
impl ChunkVectors for PgVectors {
    async fn upsert_chunks(
        &self,
        project: ProjectId,
        page: &RelPath,
        chunks: Vec<ChunkInput>,
        fingerprint: Option<&str>,
    ) -> Result<(), StoreError> {
        // An empty list leaves the page's index alone, so a failed run does
        // not wipe embeddings that were fine.
        let Some(first) = chunks.first() else {
            return Ok(());
        };
        let dimensions = first.embedding.len();
        if chunks.iter().any(|c| c.embedding.len() != dimensions) {
            return Err(StoreError::Backend(
                "the chunks of a page must all have the same dimension".into(),
            ));
        }
        let vectors: Vec<String> = chunks
            .iter()
            .map(|c| literal(&c.embedding))
            .collect::<Result<_, _>>()?;
        let indexes: Vec<i32> = chunks.iter().map(|c| c.chunk_index as i32).collect();
        let texts: Vec<&str> = chunks.iter().map(|c| c.chunk_text.as_str()).collect();
        let headings: Vec<&str> = chunks.iter().map(|c| c.heading_path.as_str()).collect();

        let (table, kind) = self.table(dimensions).await?;
        let mut tx = self.pool.begin().await.map_err(backend)?;
        let page_id: Uuid = sqlx::query_scalar(
            "SELECT id FROM pages WHERE project_id = $1 AND path = $2 AND deleted_at IS NULL
             FOR SHARE",
        )
        .bind(project.0)
        .bind(page.as_str())
        .fetch_optional(&mut *tx)
        .await
        .map_err(backend)?
        .ok_or_else(|| StoreError::not_found(page))?;
        sqlx::query(&format!("DELETE FROM {table} WHERE page_id = $1"))
            .bind(page_id)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        sqlx::query(&format!(
            "INSERT INTO {table} (project_id, page_id, chunk_index, chunk_text, heading_path, embedding)
             SELECT $1, $2, i, t, h, v::{kind} FROM unnest($3::int[], $4::text[], $5::text[], $6::text[])
                 AS u(i, t, h, v)"
        ))
        .bind(project.0)
        .bind(page_id)
        .bind(&indexes)
        .bind(&texts)
        .bind(&headings)
        .bind(&vectors)
        .execute(&mut *tx)
        .await
        .map_err(backend)?;
        let model = self.model.name().await?;
        match fingerprint {
            Some(fingerprint) => {
                sqlx::query(
                    "INSERT INTO page_embedding_state (project_id, page_id, model_id, fingerprint)
                     SELECT $1, $2, id, $4 FROM embedding_models WHERE model = $3
                     ON CONFLICT (page_id, model_id)
                     DO UPDATE SET fingerprint = EXCLUDED.fingerprint, embedded_at = now()",
                )
                .bind(project.0)
                .bind(page_id)
                .bind(&model)
                .bind(fingerprint)
                .execute(&mut *tx)
                .await
                .map_err(backend)?;
            }
            None => {
                sqlx::query(
                    "DELETE FROM page_embedding_state
                     WHERE page_id = $1 AND model_id IN (SELECT id FROM embedding_models WHERE model = $2)",
                )
                .bind(page_id)
                .bind(&model)
                .execute(&mut *tx)
                .await
                .map_err(backend)?;
            }
        }
        tx.commit().await.map_err(backend)
    }

    async fn search_chunks(
        &self,
        project: ProjectId,
        query: &[f32],
        top_k: usize,
    ) -> Result<Vec<ChunkHit>, StoreError> {
        let vector = literal(query)?;
        let Some((_, table, dimensions)) = self.existing().await? else {
            return Ok(Vec::new());
        };
        if dimensions != query.len() {
            return Err(StoreError::Backend(format!(
                "the query has {} dimensions but the embedding model has {dimensions}",
                query.len()
            )));
        }
        let kind = column_type(dimensions);
        let limit = top_k.clamp(1, 1000) as i64;
        let mut tx = self.pool.begin().await.map_err(backend)?;
        // The index covers every Project; keep scanning until the Project's
        // own nearest chunks are found instead of stopping at the first 40.
        sqlx::query("SELECT set_config('hnsw.iterative_scan', 'relaxed_order', true)")
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        sqlx::query("SELECT set_config('hnsw.ef_search', $1, true)")
            .bind(limit.max(40).to_string())
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        let rows = sqlx::query(&format!(
            "SELECT path, chunk_index, chunk_text, heading_path, d FROM (
                 SELECT p.path, c.chunk_index, c.chunk_text, c.heading_path,
                        (c.embedding <-> $2::{kind})::float4 AS d
                 FROM {table} c JOIN pages p ON p.id = c.page_id
                 WHERE c.project_id = $1 AND p.deleted_at IS NULL
                 ORDER BY c.embedding <-> $2::{kind} LIMIT $3
             ) nearest ORDER BY d, path COLLATE \"C\", chunk_index"
        ))
        .bind(project.0)
        .bind(&vector)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await
        .map_err(backend)?;
        tx.commit().await.map_err(backend)?;
        rows.iter()
            .map(|row| {
                Ok(ChunkHit {
                    path: row.try_get("path").map_err(backend)?,
                    chunk_index: row.try_get::<i32, _>("chunk_index").map_err(backend)? as u32,
                    chunk_text: row.try_get("chunk_text").map_err(backend)?,
                    heading_path: row.try_get("heading_path").map_err(backend)?,
                    distance: row.try_get("d").map_err(backend)?,
                })
            })
            .collect()
    }

    async fn delete_page(&self, project: ProjectId, page: &RelPath) -> Result<(), StoreError> {
        let Some((_, table, _)) = self.existing().await? else {
            return Ok(());
        };
        let Some(page_id) = self.page_id(project, page).await? else {
            return Ok(());
        };
        let mut tx = self.pool.begin().await.map_err(backend)?;
        sqlx::query(&format!("DELETE FROM {table} WHERE page_id = $1"))
            .bind(page_id)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        sqlx::query("DELETE FROM page_embedding_state WHERE page_id = $1")
            .bind(page_id)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        tx.commit().await.map_err(backend)
    }

    async fn count_chunks(&self, project: ProjectId) -> Result<usize, StoreError> {
        let Some((_, table, _)) = self.existing().await? else {
            return Ok(0);
        };
        let count: i64 = sqlx::query_scalar(&format!(
            "SELECT count(*) FROM {table} c JOIN pages p ON p.id = c.page_id
             WHERE c.project_id = $1 AND p.deleted_at IS NULL"
        ))
        .bind(project.0)
        .fetch_one(&self.pool)
        .await
        .map_err(backend)?;
        Ok(count as usize)
    }

    async fn clear(&self, project: ProjectId) -> Result<(), StoreError> {
        let Some((model_id, table, _)) = self.existing().await? else {
            return Ok(());
        };
        let mut tx = self.pool.begin().await.map_err(backend)?;
        sqlx::query(&format!("DELETE FROM {table} WHERE project_id = $1"))
            .bind(project.0)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        sqlx::query("DELETE FROM page_embedding_state WHERE project_id = $1 AND model_id = $2")
            .bind(project.0)
            .bind(model_id)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        tx.commit().await.map_err(backend)
    }

    async fn revision_match(
        &self,
        project: ProjectId,
        page: &RelPath,
        fingerprint: &str,
    ) -> Result<Option<usize>, StoreError> {
        let Some((model_id, table, _)) = self.existing().await? else {
            return Ok(None);
        };
        let count: Option<i64> = sqlx::query_scalar(&format!(
            "SELECT (SELECT count(*) FROM {table} c WHERE c.page_id = p.id)
             FROM pages p JOIN page_embedding_state s ON s.page_id = p.id AND s.model_id = $3
             WHERE p.project_id = $1 AND p.path = $2 AND p.deleted_at IS NULL
               AND s.fingerprint = $4"
        ))
        .bind(project.0)
        .bind(page.as_str())
        .bind(model_id)
        .bind(fingerprint)
        .fetch_optional(&self.pool)
        .await
        .map_err(backend)?;
        Ok(count.filter(|n| *n > 0).map(|n| n as usize))
    }
}

/// Brute-force vectors in memory, for tests of the callers.
#[derive(Default)]
pub struct MemoryVectors {
    // (project, page path) -> chunks and fingerprint
    pages: Mutex<BTreeMap<(Uuid, String), (Vec<ChunkInput>, Option<String>)>>,
}

impl MemoryVectors {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ChunkVectors for MemoryVectors {
    async fn upsert_chunks(
        &self,
        project: ProjectId,
        page: &RelPath,
        chunks: Vec<ChunkInput>,
        fingerprint: Option<&str>,
    ) -> Result<(), StoreError> {
        if chunks.is_empty() {
            return Ok(());
        }
        self.pages.lock().expect("lock").insert(
            (project.0, page.to_string()),
            (chunks, fingerprint.map(str::to_string)),
        );
        Ok(())
    }

    async fn search_chunks(
        &self,
        project: ProjectId,
        query: &[f32],
        top_k: usize,
    ) -> Result<Vec<ChunkHit>, StoreError> {
        let pages = self.pages.lock().expect("lock");
        let mut hits: Vec<ChunkHit> = pages
            .iter()
            .filter(|((p, _), _)| *p == project.0)
            .flat_map(|((_, path), (chunks, _))| {
                chunks
                    .iter()
                    .filter(|c| c.embedding.len() == query.len())
                    .map(|c| ChunkHit {
                        path: path.clone(),
                        chunk_index: c.chunk_index,
                        chunk_text: c.chunk_text.clone(),
                        heading_path: c.heading_path.clone(),
                        distance: c
                            .embedding
                            .iter()
                            .zip(query)
                            .map(|(a, b)| (a - b) * (a - b))
                            .sum::<f32>()
                            .sqrt(),
                    })
            })
            .collect();
        hits.sort_by(|a, b| a.distance.total_cmp(&b.distance).then(a.path.cmp(&b.path)));
        hits.truncate(top_k);
        Ok(hits)
    }

    async fn delete_page(&self, project: ProjectId, page: &RelPath) -> Result<(), StoreError> {
        self.pages
            .lock()
            .expect("lock")
            .remove(&(project.0, page.to_string()));
        Ok(())
    }

    async fn count_chunks(&self, project: ProjectId) -> Result<usize, StoreError> {
        Ok(self
            .pages
            .lock()
            .expect("lock")
            .iter()
            .filter(|((p, _), _)| *p == project.0)
            .map(|(_, (chunks, _))| chunks.len())
            .sum())
    }

    async fn clear(&self, project: ProjectId) -> Result<(), StoreError> {
        self.pages
            .lock()
            .expect("lock")
            .retain(|(p, _), _| *p != project.0);
        Ok(())
    }

    async fn revision_match(
        &self,
        project: ProjectId,
        page: &RelPath,
        fingerprint: &str,
    ) -> Result<Option<usize>, StoreError> {
        Ok(self
            .pages
            .lock()
            .expect("lock")
            .get(&(project.0, page.to_string()))
            .filter(|(_, f)| f.as_deref() == Some(fingerprint))
            .map(|(chunks, _)| chunks.len()))
    }
}

#[cfg(test)]
mod tests {
    use sqlx::postgres::PgPoolOptions;

    use super::*;
    use crate::{migrate, Body, PgFiles, Precondition, ProjectFiles, WriteCtx};

    fn chunk(index: u32, text: &str, embedding: &[f32]) -> ChunkInput {
        ChunkInput {
            chunk_index: index,
            chunk_text: text.into(),
            heading_path: format!("H{index}"),
            embedding: embedding.to_vec(),
        }
    }

    /// The behaviour both stores share.
    async fn behaves_like_a_vector_store(
        vectors: &dyn ChunkVectors,
        project: ProjectId,
        other: ProjectId,
        a: &RelPath,
        b: &RelPath,
    ) {
        vectors
            .upsert_chunks(
                project,
                a,
                vec![
                    chunk(0, "near", &[1.0, 0.0, 0.0]),
                    chunk(1, "far", &[0.0, 5.0, 0.0]),
                ],
                Some("fp-1"),
            )
            .await
            .unwrap();
        vectors
            .upsert_chunks(project, b, vec![chunk(0, "middle", &[0.0, 1.0, 0.0])], None)
            .await
            .unwrap();
        // Another Project's chunk right on top of the query never shows up.
        vectors
            .upsert_chunks(other, a, vec![chunk(0, "foreign", &[1.0, 0.1, 0.0])], None)
            .await
            .unwrap();

        let hits = vectors
            .search_chunks(project, &[1.0, 0.1, 0.0], 10)
            .await
            .unwrap();
        let texts: Vec<_> = hits.iter().map(|h| h.chunk_text.as_str()).collect();
        assert_eq!(texts, ["near", "middle", "far"]);
        assert_eq!(hits[0].path, a.as_str());
        assert_eq!(hits[0].heading_path, "H0");
        assert!(hits[0].distance < hits[1].distance);
        assert_eq!(
            vectors
                .search_chunks(project, &[1.0, 0.1, 0.0], 2)
                .await
                .unwrap()
                .len(),
            2
        );
        assert_eq!(vectors.count_chunks(project).await.unwrap(), 3);

        assert_eq!(
            vectors.revision_match(project, a, "fp-1").await.unwrap(),
            Some(2)
        );
        assert_eq!(
            vectors.revision_match(project, a, "fp-2").await.unwrap(),
            None
        );
        assert_eq!(
            vectors.revision_match(project, b, "fp-1").await.unwrap(),
            None
        );

        // A rewrite replaces the page's chunks; an empty one changes nothing.
        vectors
            .upsert_chunks(project, a, vec![chunk(0, "only", &[9.0, 9.0, 9.0])], None)
            .await
            .unwrap();
        vectors
            .upsert_chunks(project, a, vec![], None)
            .await
            .unwrap();
        assert_eq!(vectors.count_chunks(project).await.unwrap(), 2);
        assert_eq!(
            vectors.revision_match(project, a, "fp-1").await.unwrap(),
            None
        );

        vectors.delete_page(project, a).await.unwrap();
        assert_eq!(vectors.count_chunks(project).await.unwrap(), 1);
        vectors.clear(project).await.unwrap();
        assert_eq!(vectors.count_chunks(project).await.unwrap(), 0);
        assert_eq!(vectors.count_chunks(other).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn the_memory_store_behaves_like_a_vector_store() {
        let v = MemoryVectors::new();
        let (p, o) = (ProjectId(Uuid::new_v4()), ProjectId(Uuid::new_v4()));
        let a = RelPath::parse("wiki/a.md").unwrap();
        let b = RelPath::parse("wiki/b.md").unwrap();
        behaves_like_a_vector_store(&v, p, o, &a, &b).await;
    }

    async fn db() -> Option<PgPool> {
        let url = std::env::var("LLM_WIKI_TEST_DATABASE_URL").ok()?;
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .connect(&url)
            .await
            .unwrap();
        migrate(&pool).await.unwrap();
        Some(pool)
    }

    async fn project_with_pages(pool: &PgPool, pages: &[&str]) -> ProjectId {
        let id: Uuid =
            sqlx::query_scalar("INSERT INTO projects (name) VALUES ('vectors') RETURNING id")
                .fetch_one(pool)
                .await
                .unwrap();
        let files = PgFiles::new(pool.clone());
        for page in pages {
            files
                .write(
                    ProjectId(id),
                    &RelPath::parse(page).unwrap(),
                    Body::Text("# page".into()),
                    Precondition::Any,
                    &WriteCtx::new("test"),
                )
                .await
                .unwrap();
        }
        ProjectId(id)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn postgres_behaves_like_a_vector_store() {
        let Some(pool) = db().await else {
            eprintln!("skipped: LLM_WIKI_TEST_DATABASE_URL is not set");
            return;
        };
        let p = project_with_pages(&pool, &["wiki/a.md", "wiki/b.md"]).await;
        let o = project_with_pages(&pool, &["wiki/a.md"]).await;
        let model = format!("test/{}", Uuid::new_v4());
        let v = PgVectors::new(pool, ActiveModel::Fixed(model));
        let a = RelPath::parse("wiki/a.md").unwrap();
        let b = RelPath::parse("wiki/b.md").unwrap();
        behaves_like_a_vector_store(&v, p, o, &a, &b).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn postgres_refuses_what_does_not_fit() {
        let Some(pool) = db().await else {
            eprintln!("skipped: LLM_WIKI_TEST_DATABASE_URL is not set");
            return;
        };
        let p = project_with_pages(&pool, &["wiki/a.md"]).await;
        let v = PgVectors::new(pool, ActiveModel::Fixed(format!("test/{}", Uuid::new_v4())));
        let a = RelPath::parse("wiki/a.md").unwrap();
        // Nothing indexed yet: empty, not an error.
        assert!(v.search_chunks(p, &[1.0, 2.0], 5).await.unwrap().is_empty());
        v.upsert_chunks(p, &a, vec![chunk(0, "x", &[1.0, 2.0])], None)
            .await
            .unwrap();
        assert!(
            v.search_chunks(p, &[1.0, 2.0, 3.0], 5).await.is_err(),
            "wrong dimension"
        );
        assert!(v
            .upsert_chunks(p, &a, vec![chunk(0, "x", &[1.0, 2.0, 3.0])], None)
            .await
            .is_err());
        assert!(v
            .upsert_chunks(p, &a, vec![chunk(0, "x", &[f32::NAN, 1.0])], None)
            .await
            .is_err());
        assert!(v
            .upsert_chunks(
                p,
                &RelPath::parse("wiki/missing.md").unwrap(),
                vec![chunk(0, "x", &[1.0, 2.0])],
                None
            )
            .await
            .is_err());
        // A deleted page's chunks are not found.
        PgFiles::new(v.pool.clone())
            .delete(p, &a, &WriteCtx::new("test"))
            .await
            .unwrap();
        assert!(v.search_chunks(p, &[1.0, 2.0], 5).await.unwrap().is_empty());
        assert_eq!(v.count_chunks(p).await.unwrap(), 0);
    }

    #[test]
    fn the_model_in_use_is_the_first_embedding_entry_of_the_catalogue() {
        let catalogue = serde_json::json!([
            {"id": "c", "purpose": "chat", "provider": "openai", "model": "gpt"},
            {"id": "e1", "purpose": "embedding", "provider": "custom", "model": "embed-1"},
            {"id": "e2", "purpose": "embedding", "provider": "openai", "model": "embed-2"},
        ]);
        assert_eq!(
            first_embedding_model(&catalogue).as_deref(),
            Some("custom/embed-1")
        );
        assert_eq!(
            first_embedding_model(
                &serde_json::json!([{"purpose": "chat", "provider": "a", "model": "b"}])
            ),
            None
        );
        assert_eq!(first_embedding_model(&serde_json::json!({})), None);
    }
}
