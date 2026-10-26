use std::net::IpAddr;
use std::path::PathBuf;

use clap::Parser;

/// Self-hosted LLM Wiki: serves the web frontend and the wiki backend over HTTP.
#[derive(Debug, Parser)]
#[command(name = "llm-wiki-server", version)]
pub struct Args {
    /// Server state directory: the operator token and the staging area for
    /// uploads. Everything else is in Postgres and the object store. Defaults
    /// to `~/.llm-wiki-server`.
    #[arg(long, env = "LLM_WIKI_DATA_DIR")]
    pub data_dir: Option<PathBuf>,

    /// Address to listen on. Keep the loopback default behind a reverse
    /// proxy that terminates TLS; use 0.0.0.0 only on a trusted network.
    #[arg(long, env = "LLM_WIKI_BIND", default_value = "127.0.0.1")]
    pub bind: IpAddr,

    #[arg(long, env = "LLM_WIKI_PORT", default_value_t = 19830)]
    pub port: u16,

    /// The operator token, for scripts and the ingest worker (browsers sign in
    /// with an account). When omitted, one is generated on first start and kept
    /// in `<data-dir>/server-token`.
    #[arg(long, env = "LLM_WIKI_TOKEN", hide_env_values = true)]
    pub token: Option<String>,

    /// Built web frontend (`npm run build:web` → `dist-web/`). Defaults to
    /// `./dist-web` when it exists.
    #[arg(long, env = "LLM_WIKI_WEB_DIR")]
    pub web_dir: Option<PathBuf>,

    /// Mark the session cookie `Secure`. Enable when served over HTTPS.
    #[arg(long, env = "LLM_WIKI_SECURE_COOKIE")]
    pub secure_cookie: bool,

    /// Ingest worker bundle (`npm run build:worker` →
    /// `dist-worker/ingest-worker.mjs`). Defaults to that path when it
    /// exists. Without a worker, ingest runs in the browser tab.
    #[arg(long, env = "LLM_WIKI_WORKER_SCRIPT")]
    pub worker_script: Option<PathBuf>,

    /// Node.js executable used to run the worker.
    #[arg(long, env = "LLM_WIKI_NODE", default_value = "node")]
    pub node: PathBuf,

    /// Do not start the ingest worker.
    #[arg(long, env = "LLM_WIKI_NO_WORKER")]
    pub no_worker: bool,

    /// Postgres connection string. Project files live in Postgres and the S3
    /// bucket below, every path a request names must be `/projects/<uuid>/…`,
    /// and the host disk is out of reach. Commands not yet moved onto the
    /// storage layer answer that they are unavailable.
    #[arg(long, env = "LLM_WIKI_DATABASE_URL", hide_env_values = true)]
    pub database_url: String,

    /// Username of the first Platform Admin, created at startup when the
    /// platform has none.
    #[arg(long, env = "LLM_WIKI_ADMIN_USER", default_value = "admin")]
    pub admin_user: String,

    /// Password of that first Platform Admin. When omitted a random one is
    /// generated and printed once, the first time the server starts.
    #[arg(long, env = "LLM_WIKI_ADMIN_PASSWORD", hide_env_values = true)]
    pub admin_password: Option<String>,

    /// S3 API address of the object store, e.g. `http://minio:9000`.
    #[arg(long, env = "LLM_WIKI_S3_ENDPOINT")]
    pub s3_endpoint: String,

    #[arg(long, env = "LLM_WIKI_S3_BUCKET")]
    pub s3_bucket: String,

    #[arg(long, env = "LLM_WIKI_S3_ACCESS_KEY")]
    pub s3_access_key: String,

    #[arg(long, env = "LLM_WIKI_S3_SECRET_KEY", hide_env_values = true)]
    pub s3_secret_key: String,

    #[arg(long, env = "LLM_WIKI_S3_REGION", default_value = "us-east-1")]
    pub s3_region: String,
}

impl Args {
    pub fn s3_config(&self) -> llm_wiki_storage::S3Config {
        llm_wiki_storage::S3Config {
            endpoint: self.s3_endpoint.clone(),
            bucket: self.s3_bucket.clone(),
            access_key: self.s3_access_key.clone(),
            secret_key: self.s3_secret_key.clone(),
            region: self.s3_region.clone(),
        }
    }

    pub fn resolved_data_dir(&self) -> PathBuf {
        if let Some(dir) = &self.data_dir {
            return dir.clone();
        }
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        home.join(".llm-wiki-server")
    }

    pub fn resolved_worker_script(&self) -> Option<PathBuf> {
        match &self.worker_script {
            Some(script) => Some(script.clone()),
            None => {
                let default = PathBuf::from("dist-worker/ingest-worker.mjs");
                default.is_file().then_some(default)
            }
        }
    }

    pub fn resolved_web_dir(&self) -> Option<PathBuf> {
        match &self.web_dir {
            Some(dir) => Some(dir.clone()),
            None => {
                let default = PathBuf::from("dist-web");
                default.join("index.html").is_file().then_some(default)
            }
        }
    }
}
