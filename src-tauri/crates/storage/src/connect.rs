use std::sync::Arc;

use sqlx::postgres::PgPoolOptions;

use crate::{migrate, PgBlobs, PgFiles, RoutedFiles, S3Config, StoreError};

/// Opens the Project file tree: connects to Postgres, applies the schema, and
/// puts blob contents in the S3 bucket. The `.llm-wiki/*` state store is not
/// attached yet, so those paths are refused until it exists.
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
    Ok(RoutedFiles::new(Arc::new(PgFiles::new(pool)), blobs.clone()).with_blob_streams(blobs))
}
