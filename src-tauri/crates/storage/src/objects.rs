use std::fmt;
use std::sync::Arc;

use object_store::aws::AmazonS3Builder;
use object_store::ObjectStore;

use crate::StoreError;

/// Where blob contents live: a bucket on an S3-compatible server, the
/// company MinIO. Everything goes through the API with these credentials;
/// there are no presigned URLs.
#[derive(Clone)]
pub struct S3Config {
    /// Base URL of the S3 API, e.g. `http://minio:9000`.
    pub endpoint: String,
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
    /// MinIO ignores it but the signature needs one.
    pub region: String,
}

// Keeps the secret out of logs.
impl fmt::Debug for S3Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("S3Config")
            .field("endpoint", &self.endpoint)
            .field("bucket", &self.bucket)
            .field("access_key", &self.access_key)
            .field("secret_key", &"<redacted>")
            .field("region", &self.region)
            .finish()
    }
}

impl S3Config {
    /// Reads `<prefix>_ENDPOINT`, `_BUCKET`, `_ACCESS_KEY`, `_SECRET_KEY` and
    /// the optional `_REGION`. `Ok(None)` when no endpoint is set; an error
    /// when an endpoint is set but something else is missing.
    pub fn from_env(prefix: &str) -> Result<Option<Self>, StoreError> {
        let var = |name: &str| std::env::var(format!("{prefix}_{name}")).ok();
        let Some(endpoint) = var("ENDPOINT") else {
            return Ok(None);
        };
        let need = |name: &str| {
            var(name).ok_or_else(|| StoreError::Backend(format!("{prefix}_{name} is not set")))
        };
        Ok(Some(Self {
            endpoint,
            bucket: need("BUCKET")?,
            access_key: need("ACCESS_KEY")?,
            secret_key: need("SECRET_KEY")?,
            region: var("REGION").unwrap_or_else(|| "us-east-1".to_string()),
        }))
    }

    /// The object store for this bucket. Path-style addressing, since MinIO
    /// is reached by service name rather than a bucket subdomain; plain HTTP
    /// is allowed only when the endpoint itself says `http://`.
    pub fn build(&self) -> Result<Arc<dyn ObjectStore>, StoreError> {
        let store = AmazonS3Builder::new()
            .with_endpoint(&self.endpoint)
            .with_bucket_name(&self.bucket)
            .with_access_key_id(&self.access_key)
            .with_secret_access_key(&self.secret_key)
            .with_region(&self.region)
            .with_virtual_hosted_style_request(false)
            .with_allow_http(self.endpoint.starts_with("http://"))
            .build()
            .map_err(|error| StoreError::Backend(format!("object store: {error}")))?;
        Ok(Arc::new(store))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_output_hides_the_secret() {
        let config = S3Config {
            endpoint: "http://minio:9000".into(),
            bucket: "b".into(),
            access_key: "key".into(),
            secret_key: "hunter2".into(),
            region: "us-east-1".into(),
        };
        let shown = format!("{config:?}");
        assert!(!shown.contains("hunter2"), "{shown}");
        assert!(shown.contains("key"));
    }

    #[test]
    fn an_endpoint_without_credentials_is_an_error_not_a_silent_skip() {
        // Distinct prefix so parallel tests cannot see each other's variables.
        std::env::set_var("LLM_WIKI_UNIT_S3_ENDPOINT", "http://x:9000");
        let err = S3Config::from_env("LLM_WIKI_UNIT_S3").unwrap_err();
        assert!(err.to_string().contains("LLM_WIKI_UNIT_S3_BUCKET"), "{err}");
        assert_eq!(
            S3Config::from_env("LLM_WIKI_UNIT_NONE")
                .unwrap()
                .map(|c| c.bucket),
            None
        );
    }

    #[test]
    fn a_config_builds_a_store_without_contacting_the_server() {
        let config = S3Config {
            endpoint: "http://minio:9000".into(),
            bucket: "b".into(),
            access_key: "k".into(),
            secret_key: "s".into(),
            region: "us-east-1".into(),
        };
        assert!(config.build().is_ok());
    }
}
