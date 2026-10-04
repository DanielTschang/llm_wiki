use std::net::IpAddr;
use std::path::PathBuf;

use clap::Parser;

/// Self-hosted LLM Wiki: serves the web frontend and the wiki backend over HTTP.
#[derive(Debug, Parser)]
#[command(name = "llm-wiki-server", version)]
pub struct Args {
    /// Server state directory: app settings (`app-state.json`), the access
    /// token, and `projects/` for wiki projects. Defaults to
    /// `~/.llm-wiki-server`.
    #[arg(long, env = "LLM_WIKI_DATA_DIR")]
    pub data_dir: Option<PathBuf>,

    /// Address to listen on. Keep the loopback default behind a reverse
    /// proxy that terminates TLS; use 0.0.0.0 only on a trusted network.
    #[arg(long, env = "LLM_WIKI_BIND", default_value = "127.0.0.1")]
    pub bind: IpAddr,

    #[arg(long, env = "LLM_WIKI_PORT", default_value_t = 19830)]
    pub port: u16,

    /// Access token. When omitted, one is generated on first start and kept
    /// in `<data-dir>/server-token`.
    #[arg(long, env = "LLM_WIKI_TOKEN", hide_env_values = true)]
    pub token: Option<String>,

    /// Extra directories (besides `<data-dir>/projects`) that projects and
    /// imported files may live in. Every path a request names must be
    /// inside one of them. Repeat the flag or separate with commas.
    #[arg(
        long = "allow-root",
        env = "LLM_WIKI_ALLOW_ROOTS",
        value_delimiter = ','
    )]
    pub allow_roots: Vec<PathBuf>,

    /// Built web frontend (`npm run build:web` → `dist-web/`). Defaults to
    /// `./dist-web` when it exists.
    #[arg(long, env = "LLM_WIKI_WEB_DIR")]
    pub web_dir: Option<PathBuf>,

    /// Let the agent run shell commands the user approves in chat. Off by
    /// default: on a server this is remote code execution for anyone holding
    /// the token.
    #[arg(long, env = "LLM_WIKI_ALLOW_SHELL")]
    pub allow_shell: bool,

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
}

impl Args {
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
