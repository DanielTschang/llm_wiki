//! Self-hosted LLM Wiki server. See plans/web-server-mode.md (Phase 3).

mod auth;
mod config;
mod desktop_settings;
mod dispatch;
mod events;
mod files;
mod paths;
mod proxy;
mod rpc;
mod store;
mod upload;
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
use tower::ServiceExt;
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
    /// Staging area for files uploaded from the browser's device.
    upload_root: Arc<PathBuf>,
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
    let upload_root = data_dir.join("projects").join(upload::UPLOAD_DIR);
    std::fs::create_dir_all(&upload_root)
        .map_err(|e| format!("Cannot create {}: {e}", upload_root.display()))?;
    upload::sweep_stale(&upload_root);
    let roots = AllowedRoots::new(
        std::iter::once(data_dir.join("projects")).chain(args.allow_roots.iter().cloned()),
    )?;

    // On a machine that also runs the desktop app, start from its models.
    let mut desktop_import = None;
    if !args.no_desktop_settings {
        if let Some(desktop) = desktop_settings::desktop_app_state_path() {
            match desktop_settings::seed_from_desktop(&data_dir.join("app-state.json"), &desktop) {
                Ok(Some(_)) => desktop_import = Some(desktop),
                Ok(None) => {}
                Err(e) => eprintln!("[settings] could not import desktop settings: {e}"),
            }
        }
    }

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
        upload_root: Arc::new(upload_root),
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
    if let Some(desktop) = &desktop_import {
        eprintln!(
            "  settings:  imported model settings from the desktop app ({})",
            desktop.display()
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
        .layer(DefaultBodyLimit::max(MAX_RPC_BODY_BYTES))
        // Streams to disk with its own size check.
        .route("/upload", post(upload::receive));

    let app = Router::new()
        .route("/login", get(login_page).post(login))
        .route("/logout", post(logout))
        .route("/healthz", get(|| async { Json(json!({ "ok": true })) }))
        .merge(api);

    let app = match web_dir {
        Some(dir) => app.merge(static_routes(&dir)),
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

/// The built web app. Hashed `/assets/*` files are immutable and a missing
/// one is a 404, so a tab still running an older build fails clearly (and
/// reloads, see main.tsx) instead of receiving `index.html` as JavaScript.
/// Everything else falls back to `index.html`, which is never cached so a
/// reload always picks up the current build.
fn static_routes<S: Clone + Send + Sync + 'static>(dir: &std::path::Path) -> Router<S> {
    fn immutable<B>(mut response: Response<B>) -> Response<B> {
        if response.status().is_success() {
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=31536000, immutable"),
            );
        }
        response
    }
    fn revalidate<B>(mut response: Response<B>) -> Response<B> {
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
        response
    }
    let assets = ServiceExt::<Request>::map_response(ServeDir::new(dir.join("assets")), immutable);
    let app = ServiceExt::<Request>::map_response(
        ServeDir::new(dir).fallback(ServeFile::new(dir.join("index.html"))),
        revalidate,
    );
    Router::new()
        .nest_service("/assets", assets)
        .fallback_service(app)
}

/// Everything except the login page, health check and PWA assets needs the token.
async fn require_auth(State(state): State<AppState>, mut request: Request, next: Next) -> Response {
    let path = request.uri().path().to_owned();
    let path = path.as_str();
    // The PWA manifest and icons are fetched without cookies by browsers,
    // and are not sensitive.
    let public = path == "/login"
        || path == "/healthz"
        || path == "/manifest.webmanifest"
        || path.starts_with("/icons/");
    if public {
        return next.run(request).await;
    }
    let is_api = path.starts_with("/rpc/")
        || matches!(
            path,
            "/events" | "/files" | "/proxy" | "/upload" | "/logout"
        );
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
<link rel="icon" type="image/png" href="/icons/icon-192.png">
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request as HttpRequest;

    async fn get(router: &Router, path: &str) -> (StatusCode, Option<String>, String) {
        let response = router
            .clone()
            .oneshot(HttpRequest::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let cache = response
            .headers()
            .get(header::CACHE_CONTROL)
            .map(|v| v.to_str().unwrap().to_string());
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .unwrap()
            .to_bytes();
        (status, cache, String::from_utf8_lossy(&body).into_owned())
    }

    #[tokio::test]
    async fn serves_assets_immutably_and_404s_stale_ones() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("assets")).unwrap();
        std::fs::write(dir.path().join("index.html"), "<html>app</html>").unwrap();
        std::fs::write(dir.path().join("assets/mermaid-NEW.js"), "export {}").unwrap();
        let router: Router = static_routes(dir.path());

        let (status, cache, _) = get(&router, "/assets/mermaid-NEW.js").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            cache.as_deref(),
            Some("public, max-age=31536000, immutable")
        );

        let (status, _, body) = get(&router, "/assets/mermaid-OLD.js").await;
        assert_eq!(
            status,
            StatusCode::NOT_FOUND,
            "stale chunk must not get index.html"
        );
        assert!(!body.contains("<html>"));

        let (status, cache, body) = get(&router, "/some/client/route").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(cache.as_deref(), Some("no-cache"));
        assert_eq!(body, "<html>app</html>");
    }
}
