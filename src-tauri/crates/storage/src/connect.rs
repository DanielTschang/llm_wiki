use std::sync::Arc;

use sqlx::postgres::PgPoolOptions;

use crate::{
    migrate, Accounts, ActiveModel, LlmUsage, PgBlobs, PgFiles, PgState, PgVectors, Projects,
    RoutedFiles, S3Config, ServerSettings, StoreError,
};

/// Everything a platform server needs from the database and the object store.
#[derive(Clone)]
pub struct Services {
    /// The Project file tree: pages, blobs and `.llm-wiki/*` state files.
    pub files: Arc<RoutedFiles>,
    /// The blob store on its own, for what the file tree does not do: removing
    /// a Project's objects when the Project is deleted.
    pub blobs: Arc<PgBlobs>,
    /// Chunk embeddings, for the embedding model the catalogue designates.
    pub vectors: Arc<PgVectors>,
    pub accounts: Accounts,
    pub projects: Projects,
    pub settings: ServerSettings,
    pub usage: LlmUsage,
}

/// Connects to Postgres, applies the schema, and puts blob contents in the S3
/// bucket. `.llm-wiki/*` state files are rows of `state_files`.
pub async fn open_services(
    database_url: &str,
    s3: &S3Config,
    max_connections: u32,
) -> Result<Services, StoreError> {
    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(database_url)
        .await
        .map_err(|error| StoreError::Backend(format!("connect to postgres: {error}")))?;
    migrate(&pool).await?;
    let blobs = Arc::new(PgBlobs::new(pool.clone(), s3.build()?));
    let pages = Arc::new(PgFiles::new(pool.clone()));
    // Pages stored before the link tables existed get their links and frontmatter now.
    pages.reindex_stale().await?;
    let files = RoutedFiles::new(pages, blobs.clone())
        .with_state(Arc::new(PgState::new(pool.clone())))
        .with_blob_streams(blobs.clone());
    Ok(Services {
        files: Arc::new(files),
        blobs,
        vectors: Arc::new(PgVectors::new(
            pool.clone(),
            ActiveModel::Catalogue(ServerSettings::new(pool.clone())),
        )),
        accounts: Accounts::new(pool.clone()),
        projects: Projects::new(pool.clone()),
        settings: ServerSettings::new(pool.clone()),
        usage: LlmUsage::new(pool),
    })
}

/// [`open_services`] for callers that only need the file tree.
pub async fn open_store(
    database_url: &str,
    s3: &S3Config,
    max_connections: u32,
) -> Result<RoutedFiles, StoreError> {
    let services = open_services(database_url, s3, max_connections).await?;
    Arc::try_unwrap(services.files)
        .map_err(|_| StoreError::Backend("the file tree is still shared".into()))
}
