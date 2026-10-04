//! Self-hosted LLM Wiki server. See plans/web-server-mode.md (Phase 3).

mod auth;
mod config;
mod dispatch;
mod events;
mod files;
mod paths;
mod proxy;
mod rpc;
mod store;
mod worker;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::extract::{DefaultBodyLimit, Request, State};
use axum::http::{header, HeaderValue, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{any, get, post};
use axum::{Form, Json, Router};
use clap::Parser;
use llm_wiki_core::{CoreContext, EventEmitter};
use serde::Deserialize;
use serde_json::json;
use tower_http::services::{ServeDir, ServeFile};

use crate::auth::{Auth, Credential, CLIENT_HEADER};
use crate::events::EventHub;
use crate::paths::AllowedRoots;
use crate::store::AppStore;
use crate::worker::WorkerBridge;

const MAX_RPC_BODY_BYTES: usize = 256 * 1024 * 1024;

#[derive(Clone)]
pub struct AppState {
    core: Arc<CoreContext>,
    events: EventHub,
    store: Arc<AppStore>,
    roots: Arc<AllowedRoots>,
    auth: Arc<Auth>,
    allow_shell: bool,
    worker: Arc<WorkerBridge>,
    /// Serializes `.llm-wiki/review-inbox.json` read-modify-writes.
    inbox_lock: Arc<Mutex<()>>,
}

#[tokio::main]
async fn main() {
    if let Err(message) = run(config::Args::parse()).await {
        eprintln!("llm-wiki-server: {message}");
        std::process::exit(1);
    }
}

async fn run(args: config::Args) -> Result<(), String> {
    let data_dir = args.resolved_data_dir();
    let projects_dir = data_dir.join("projects");
    std::fs::create_dir_all(&projects_dir)
        .map_err(|e| format!("Cannot create {}: {e}", projects_dir.display()))?;
    let data_dir = data_dir
        .canonicalize()
        .map_err(|e| format!("Cannot resolve {}: {e}", data_dir.display()))?;

    let (token, generated) = auth::load_or_create_token(&data_dir, args.token.clone())?;
    let roots = AllowedRoots::new(
        std::iter::once(data_dir.join("projects")).chain(args.allow_roots.iter().cloned()),
    )?;

    // Same startup step as the desktop app: honor the saved proxy settings
    // before any outbound request.
    if let Some(proxy) =
        llm_wiki_core::proxy::read_proxy_config_from_store(&data_dir.join("app-state.json"))
    {
        eprintln!("[proxy] {}", llm_wiki_core::proxy::apply_proxy_env(&proxy));
    }
    llm_wiki_core::runtime::install(tokio::runtime::Handle::current());

    let events = EventHub::new();
    let worker_token = uuid::Uuid::new_v4().simple().to_string();
    let state = AppState {
        core: Arc::new(CoreContext::new(
            Some(data_dir.clone()),
            env!("CARGO_PKG_VERSION"),
            EventEmitter::new(events.clone()),
        )),
        store: Arc::new(AppStore::new(&data_dir)),
        roots: Arc::new(roots),
        auth: Arc::new(Auth::new(&token, args.secure_cookie).with_worker_token(&worker_token)),
        allow_shell: args.allow_shell,
        worker: Arc::new(WorkerBridge::new(events.clone())),
        inbox_lock: Arc::new(Mutex::new(())),
        events,
    };

    let web_dir = args.resolved_web_dir();
    let app = router(state.clone(), web_dir.clone());
    let addr = SocketAddr::new(args.bind, args.port);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Cannot listen on {addr}: {e}"))?;

    eprintln!("LLM Wiki server listening on http://{addr}");
    eprintln!("  data dir:  {}", data_dir.display());
    for root in state.roots.roots() {
        eprintln!("  allowed:   {}", root.display());
    }
    match &web_dir {
        Some(dir) => eprintln!("  web app:   {}", dir.display()),
        None => eprintln!("  web app:   not found — run `npm run build:web` and pass --web-dir"),
    }
    if generated {
        eprintln!(
            "  token:     {token}   (saved to {})",
            data_dir.join("server-token").display()
        );
    } else if args.token.is_none() {
        eprintln!(
            "  token:     from {}",
            data_dir.join("server-token").display()
        );
    }
    if args.allow_shell {
        eprintln!("  WARNING: --allow-shell lets approved agent commands run on this machine");
    }

    match args.resolved_worker_script() {
        Some(script) if !args.no_worker => {
            eprintln!("  ingest:    worker {}", script.display());
            // The worker always connects over loopback, whatever --bind is.
            let host = if args.bind.is_unspecified() || args.bind.is_loopback() {
                "127.0.0.1".to_string()
            } else {
                args.bind.to_string()
            };
            worker::supervise(worker::WorkerLaunch {
                node: args.node.clone(),
                script,
                server_url: format!("http://{host}:{}", args.port),
                token: worker_token,
            });
        }
        _ => eprintln!(
            "  ingest:    in the browser tab (no worker; build it with `npm run build:worker`)"
        ),
    }

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .map_err(|e| e.to_string())
}

fn router(state: AppState, web_dir: Option<PathBuf>) -> Router {
    let api = Router::new()
        .route("/rpc/{command}", post(rpc::handle))
        .route("/events", get(events::sse))
        .route("/files", get(files::serve))
        .route("/proxy", any(proxy::forward))
        .layer(DefaultBodyLimit::max(MAX_RPC_BODY_BYTES));

    let app = Router::new()
        .route("/login", get(login_page).post(login))
        .route("/logout", post(logout))
        .route("/healthz", get(|| async { Json(json!({ "ok": true })) }))
        .merge(api);

    let app = match web_dir {
        Some(dir) => {
            let index = dir.join("index.html");
            app.fallback_service(ServeDir::new(dir).fallback(ServeFile::new(index)))
        }
        None => app.fallback(|| async {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Html("<p>The web frontend is not installed. Run <code>npm run build:web</code> and start the server with <code>--web-dir dist-web</code>.</p>"),
            )
        }),
    };

    app.layer(middleware::from_fn_with_state(state.clone(), require_auth))
        .with_state(state)
}

/// Everything except the login page and health check needs the token.
async fn require_auth(State(state): State<AppState>, mut request: Request, next: Next) -> Response {
    let path = request.uri().path().to_owned();
    let path = path.as_str();
    if path == "/login" || path == "/healthz" {
        return next.run(request).await;
    }
    let is_api =
        path.starts_with("/rpc/") || matches!(path, "/events" | "/files" | "/proxy" | "/logout");
    let credential = state.auth.credential(request.headers());
    request.extensions_mut().insert(credential);
    match credential {
        Credential::Bearer | Credential::Worker => next.run(request).await,
        Credential::Cookie => {
            let changes_state = request.method() != Method::GET && request.method() != Method::HEAD;
            let needs_client_header =
                path.starts_with("/rpc/") || path == "/proxy" || changes_state;
            if needs_client_header && !request.headers().contains_key(CLIENT_HEADER) {
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({ "error": "Missing X-LLM-Wiki-Client header" })),
                )
                    .into_response();
            }
            next.run(request).await
        }
        Credential::None if is_api => (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Not signed in" })),
        )
            .into_response(),
        Credential::None => Redirect::to("/login").into_response(),
    }
}

#[derive(Deserialize)]
struct LoginForm {
    token: String,
}

async fn login_page() -> Html<String> {
    Html(login_html(None))
}

async fn login(State(state): State<AppState>, Form(form): Form<LoginForm>) -> Response {
    if !state.auth.token_matches(form.token.trim()) {
        return (
            StatusCode::UNAUTHORIZED,
            Html(login_html(Some("Wrong token."))),
        )
            .into_response();
    }
    let mut response = Redirect::to("/").into_response();
    if let Ok(cookie) = HeaderValue::from_str(&state.auth.session_cookie()) {
        response.headers_mut().insert(header::SET_COOKIE, cookie);
    }
    response
}

async fn logout(State(state): State<AppState>) -> Response {
    let mut response = StatusCode::NO_CONTENT.into_response();
    if let Ok(cookie) = HeaderValue::from_str(&state.auth.clear_cookie()) {
        response.headers_mut().insert(header::SET_COOKIE, cookie);
    }
    response
}

fn login_html(error: Option<&str>) -> String {
    let error = error
        .map(|message| format!(r#"<p class="error">{message}</p>"#))
        .unwrap_or_default();
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>LLM Wiki · Sign in</title>
<style>
  :root {{ color-scheme: light dark; --bg: #ffffff; --fg: #1f2023; --muted: #6b6f76; --border: #d9dbe0; --accent: #2f6fed; }}
  @media (prefers-color-scheme: dark) {{ :root {{ --bg: #27282b; --fg: #ececee; --muted: #a0a3aa; --border: #3d3f44; --accent: #6b9bff; }} }}
  body {{ margin: 0; min-height: 100vh; display: grid; place-items: center; background: var(--bg); color: var(--fg); font: 15px/1.5 system-ui, sans-serif; }}
  form {{ width: min(360px, calc(100vw - 32px)); display: grid; gap: 12px; }}
  h1 {{ font-size: 20px; margin: 0; }}
  p {{ margin: 0; color: var(--muted); }}
  input {{ padding: 10px 12px; border: 1px solid var(--border); border-radius: 8px; background: transparent; color: inherit; font: inherit; }}
  button {{ padding: 10px 12px; border: 0; border-radius: 8px; background: var(--accent); color: #fff; font: inherit; cursor: pointer; }}
  .error {{ color: #d64545; }}
</style>
</head>
<body>
<form method="post" action="/login">
  <h1>LLM Wiki</h1>
  <p>Enter the server access token. It is printed when the server first starts and stored in <code>server-token</code> in the data directory.</p>
  {error}
  <input name="token" type="password" autocomplete="current-password" autofocus required aria-label="Access token">
  <button type="submit">Sign in</button>
</form>
</body>
</html>"#
    )
}
