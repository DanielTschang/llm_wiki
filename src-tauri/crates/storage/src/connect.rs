use std::sync::Arc;

use sqlx::postgres::PgPoolOptions;

use crate::{migrate, PgBlobs, PgFiles, PgState, RoutedFiles, S3Config, StoreError};

/// Opens the Project file tree: connects to Postgres, applies the schema, and
/// puts blob contents in the S3 bucket. `.llm-wiki/*` state files are rows of
/// `state_files`.
pub async fn open_store(
    database_url: &str,
    s3: &S3Config,
    max_connections: u32,
) -> Result<RoutedFiles, StoreError> {
    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(database_url)
        .await
        .map_err(|error| StoreError::Backend(format!("connect to postgres: {error}")))?;
    migrate(&pool).await?;
    let blobs = Arc::new(PgBlobs::new(pool.clone(), s3.build()?));
    Ok(
        RoutedFiles::new(Arc::new(PgFiles::new(pool.clone())), blobs.clone())
            .with_state(Arc::new(PgState::new(pool)))
            .with_blob_streams(blobs),
    )
}
