# Web server mode: self-hosted backend + browser frontend

**Status:** Phases 1–4 done on `DanielTschang/second`, plus a Dockerfile. Phase 5 not started.

**Goal:** Run LLM Wiki as a headless server (NAS / VPS / Docker, no
desktop session) and use the existing React UI from any browser. The
desktop Tauri app keeps working unchanged; both shells share one Rust
core and one frontend build.

**Non-goals (this round):**
- Multi-user accounts / per-user permissions. One server = one owner,
  protected by a single bearer token (same model as the existing
  `api_server.rs` token).
- Pure static hosting (Vercel etc.). LLM CORS, LanceDB, pdfium and the
  agent shell tool all need a backend.
- Rewriting the ingest pipeline in Rust. See Phase 4 for how headless
  ingest is handled without a rewrite.

---

## Current state (audit, not assumption)

### Frontend → Tauri coupling

| Surface | Where | Count |
|---|---|---|
| `invoke(...)` | mostly via `src/commands/fs.ts` (33 exports) + 17 other files | ~68 distinct commands |
| `listen(...)` events | `chat-panel.tsx` (`agent-event`), `project-file-sync.ts` (`file-sync://*`), `claude-cli-transport.ts`, `codex-cli-transport.ts` | 4 files |
| `@tauri-apps/plugin-http` | only through `src/lib/tauri-fetch.ts::getHttpFetch()` | 1 seam |
| `plugin-store` (`app-state.json`) | `src/lib/project-store.ts`, `src/lib/project-identity.ts` | 2 files |
| `plugin-dialog` (`open`/`save`/`message`) | App, sources-view, create-project, maintenance, scheduled-import, file-tree | 6 files |
| `plugin-opener` (`openUrl`/`openPath`) | about, api-server, chat-message, update-banner, file-preview, frontmatter-panel | 6 files |
| `convertFileSrc` (asset protocol) | file-preview, chat-panel, chat-message | 3 files |
| `plugin-autostart`, `api/window` (theme) | App, settings-view, theme.ts | desktop-only |

**Critical:** ingest queue, two-step ingest, LLM calls, graph/relevance,
lint and deep research all run **in the frontend** (`src/lib/ingest-queue.ts`
etc.), using `getHttpFetch()` to bypass CORS. In a browser they must go
through a server-side proxy, and they stop when the tab is closed.

### Rust → Tauri coupling

- `commands/fs.rs`, `vectorstore.rs`, `file_history.rs`, `search.rs`,
  `project_maintenance.rs`, `external_search.rs`, `extract_images.rs`:
  **only the `#[tauri::command]` attribute**, function bodies are plain.
- Real `AppHandle` / `State` / `Emitter` use:
  - `file_sync.rs` (watcher state + emits `file-sync://queue-updated`, `file-sync://changed`)
  - `claude_cli.rs`, `codex_cli.rs` (child-process state + per-stream emits)
  - `lib.rs` agent commands (`agent-event` emits, `AgentSessionStore`, `AgentCancellationRegistry`)
  - `api_server.rs` (`app.path().app_data_dir()` for `app-state.json`, `app.state::<…>()`)
  - `project.rs::open_project_folder` / `open_path_in_project` (OS file manager — desktop only)
  - `tray.rs`, autostart, close behavior — desktop only
- `api_server.rs` is `tiny_http`, 4k lines, exposes a narrow `/api/v1`
  surface (search, chat, pages read/write/embed). Not enough to drive the UI.

---

## Target architecture

```
                 ┌──────────── frontend (one Vite build) ────────────┐
                 │  src/platform/  ← the only place that knows the shell │
                 │    invoke · listen · httpFetch · store · dialogs     │
                 │    opener · fileSrc                                  │
                 └───────┬───────────────────────────┬─────────────────┘
              Tauri IPC  │                           │ HTTP + SSE
        ┌────────────────▼──────┐        ┌───────────▼────────────────┐
        │ src-tauri (desktop)   │        │ server/ (axum binary)      │
        │ thin #[tauri::command]│        │ POST /rpc/:command         │
        │ wrappers, tray, etc.  │        │ GET  /events (SSE)         │
        └────────────┬──────────┘        │ POST /proxy (LLM fetch)    │
                     │                   │ GET  /files/*  (fileSrc)   │
                     │                   │ static frontend, auth      │
                     │                   └───────────┬────────────────┘
                     └────────────┬──────────────────┘
                         ┌────────▼─────────┐
                         │ core/ (no tauri) │  commands, agent, vectorstore,
                         │ EventSink trait  │  file_sync, cli transports,
                         │ AppConfigStore   │  api_server routes
                         └──────────────────┘
```

Key decisions:
1. **RPC mirrors `invoke` 1:1.** `POST /rpc/<command>` with the same
   camelCase JSON args. The frontend change becomes a transport swap,
   not 68 call-site rewrites; new commands work in both shells for free.
2. **Events → one SSE stream** (`GET /events`), each message
   `{ event, payload }`. `platform.listen(name, cb)` filters client-side.
3. **`app-state.json` moves server-side** behind `get/set` RPCs
   (`AppConfigStore` trait in core; desktop impl keeps plugin-store's file
   so existing installs are not migrated).
4. **Paths are server paths.** Projects live under a configured
   `--data-dir`; dialogs become an in-app server directory browser plus
   browser upload for importing sources.

---

## Phases

Each phase ends green on `npm test` + `cargo test` and keeps the desktop
app shippable.

### Phase 1 — Frontend platform seam (desktop behavior unchanged) ✅
- `src/platform/{types,tauri,web,index}.ts` expose `invoke`, `listen`,
  `convertFileSrc`, `createHttpFetch`, `loadStore`, `openDialog` /
  `saveDialog` / `messageDialog`, `openUrl`, `openPath`,
  `isAutostartEnabled` / `setAutostart`, `setWindowTheme`, `isDesktop`.
- Every direct `@tauri-apps/*` import moved behind `@/platform`.
  The repo has no ESLint, so `src/platform/no-direct-tauri-imports.test.ts`
  enforces it instead.
- Desktop is the default implementation (also under vitest, so existing
  `vi.mock("@tauri-apps/...")` mocks still apply). `npm run build:web`
  (`vite build --mode web`, reads `.env.web`) selects the web
  implementation; the Tauri plugins are tree-shaken out of that bundle.
- `web.ts` already implements the HTTP contract the Phase 3 server must
  serve (`/rpc`, `/events`, `/files`, `/proxy`, `app_store_*`), covered
  by `web.test.ts`.
- Desktop-only capabilities degrade to no-ops in web (autostart, window
  theme) or browser equivalents (`openUrl` → new tab, `openPath` → open
  the file via `/files`, dialogs → `window.prompt` for a server path).
  **Deferred to Phase 5:** hiding desktop-only UI (autostart toggle,
  reveal-in-folder, Claude/Codex CLI providers) when `!isDesktop`.

### Phase 2 — Rust core crate ✅
- `src-tauri/Cargo.toml` is now also the workspace root
  (`members = ["crates/core"]`, `default-members = [".", "crates/core"]`),
  so `src-tauri/target`, `src-tauri/Cargo.lock`, CI's `cargo build` and
  release paths are unchanged, and `cargo test` in `src-tauri` covers core.
- `src-tauri/crates/core` (`llm-wiki-core`, no `tauri` dependency) holds
  `agent/`, `commands/`, `api_server`, `clip_server`, `proxy`, `cors`,
  `server_bind`, `panic_guard`, `types`. It builds standalone
  (`cargo check -p llm-wiki-core`).
- `CoreContext` (core `context.rs`) replaces `AppHandle`/`State<…>`:
  app data dir (`app-state.json`), host app version, an `EventEmitter`
  (`EventSink` trait; desktop impl forwards to `AppHandle::emit`) and the
  long-lived state (file watcher, CLI children, agent sessions/cancellation).
- `runtime.rs` replaces `tauri::async_runtime` for the tiny_http threads:
  the desktop installs Tauri's tokio handle; without one (server, tests)
  core owns a runtime.
- The agent chat commands moved from `lib.rs` to `agent/commands.rs`;
  status/proxy commands to `commands/app.rs`. Project "open in file
  manager" was split: path validation in core
  (`resolve_project_folder`, `resolve_path_in_project`), the OS opener
  stays in the desktop shell.
- `src-tauri/src/commands.rs` holds one-line `#[tauri::command]` wrappers.
  All 78 IPC commands were checked against the pre-move source: same
  names, sync/async, argument names and types.
- Desktop-only and left in `src-tauri/src/lib.rs`: tray, close behavior,
  autostart/dialog/opener plugins, `mcp_server_entry_path` (resolves
  bundle resources), `open_project_folder`, `open_path_in_project`.

### Phase 3 — `server` binary ✅
- `src-tauri/crates/server` (`llm-wiki-server`): axum on `127.0.0.1:19830`
  by default. Run: `npm run build:web`, then
  `cargo run -p llm-wiki-server -- --web-dir ../dist-web` from `src-tauri`
  (`--help` lists `--data-dir`, `--allow-root`, `--token`, `--bind`,
  `--allow-shell`, `--secure-cookie`; each has an `LLM_WIKI_*` env var).
- Routes: `/rpc/{command}` (`dispatch.rs`, one arm per core command;
  `dispatches_every_desktop_command` fails if it drifts from the desktop
  handler list), `/events` (SSE from an `EventSink` broadcast), `/files`
  (`ServeFile`, range requests), `/proxy` (reqwest via
  `core::proxy::configure_http_client`, streamed both ways), static
  `dist-web` with SPA fallback, `/login`, `/logout`, `/healthz`.
- `app_store_get/set/delete` keep `app-state.json` in the data dir in
  plugin-store's format, so core code that reads it directly works.
- Security:
  - Token auth: generated on first start into `<data-dir>/server-token`
    (0600) unless `--token`; browsers get an `HttpOnly; SameSite=Strict`
    session cookie, scripts use `Authorization: Bearer`.
  - CSRF: cookie-authenticated `/rpc`, `/proxy` and non-GET requests must
    send `X-LLM-Wiki-Client` (no CORS preflight is ever approved).
  - Every path argument (`path`, `projectPath`, `filePath`, `source`,
    `destination`, …, `paths[]`) and every project path written to the
    project registry must resolve (symlinks followed) inside
    `<data-dir>/projects` or an `--allow-root`. The token file and
    `app-state.json` are outside those roots.
  - `/files` sends `nosniff`, and `CSP: sandbox` for HTML/SVG/XML so
    agent-generated pages cannot act with the owner's session.
  - Agent `approvedShellCommands` are cleared unless `--allow-shell`.
  - Claude Code / Codex CLI commands are disabled in server mode.
- Frontend: `X-LLM-Wiki-Client` on RPC/proxy calls, 401 → `/login`;
  desktop-only startup calls (`set_close_behavior`, clip server
  notifications/polling) skipped when `!isDesktop`. `npm run dev:web`
  serves the web build on :1430 proxying to a local server.
- Verified end to end: 21 curl checks (auth, CSRF, confinement, store,
  `/files` headers, proxy) plus a browser run (login → open project →
  preview page; SSE connected, file watcher events delivered).
- Docker: `Dockerfile` (web + worker build, server build with
  protoc/libprotobuf-dev, the repo's Linux pdfium per `TARGETARCH`,
  Debian slim runtime with the node binary for the worker) and
  `docker-compose.yml` (data volume, loopback-only port, optional
  `/projects` bind mount as an allowed root).
- **Not done yet:** running the tiny_http API/MCP and clip servers inside
  the server process, and a documented HTTPS reverse-proxy setup.

### Phase 4 — Headless ingest ✅
Goal: closing every browser tab no longer stops ingest.

Audit: `ingest-queue.ts` / `ingest.ts` (~5k lines) depend only on
zustand stores (plain JS) and `@/platform`, so they run in Node. The UI
polls `getQueue()` in-process; `ingest.ts` adds review items and updates
activity items through stores; the browser auto-saves the *whole* review
list to `.llm-wiki/review.json`.

Design:
- **Worker process.** `src/worker/main.ts`, bundled to
  `dist-worker/ingest-worker.mjs`, supervised by `llm-wiki-server`
  (spawned with Node, restarted on exit). It reaches the server over
  loopback with the Bearer token using the existing web platform
  (`/rpc`, `/events`), with Node's `fetch` instead of `/proxy` (no CORS
  in Node). It hydrates settings from `app-state.json` and re-hydrates
  on `app-store://changed`.
- **Single owner of the queue.** The web build swaps
  `@/lib/ingest-queue` for a remote module with the same API: it
  mirrors `ingest://queue` snapshots from SSE and forwards mutations
  (enqueue, cancel, retry, pause, …) as `ingest_command` RPCs that the
  server relays to the worker. The worker publishes queue and activity
  snapshots back through `worker_publish`.
- **Review items without lost writes.** The worker never writes
  `review.json`. It appends new items to a server-side inbox
  (`review_inbox_append`, serialized by a mutex in the server); the
  browser drains it (`review_inbox_take`) on project load and on
  `ingest://review-inbox` events, then its usual auto-save persists them.
- **v1 limits:** file watching stays browser-driven (the Rust watcher
  holds one project at a time); files added while no tab is open are
  picked up by the startup rescan when a tab next opens. `withProjectLock`
  is per process, so edits in the UI during ingest are as safe as
  concurrent edits from two tabs, not as safe as on desktop.
- Docker image gains a Node runtime for the worker.

As built:
- `npm run build:worker` bundles `src/worker/main.ts` with all
  dependencies into `dist-worker/ingest-worker.mjs` (Vite SSR build,
  `--mode worker`, `.env.worker`). The platform's `worker` mode is the web
  implementation with a Bearer token, direct `fetch` and a fetch-based SSE
  client (`node-event-source.ts`).
- The server spawns it when the bundle exists (`--worker-script`,
  `--node`, `--no-worker`), with a per-run worker token
  (`Credential::Worker`), keeps its stdin open as a liveness pipe, and
  restarts it with backoff. `worker_*` RPCs are worker-only;
  `ingest_command` fails fast when no heartbeat arrived in 30s.
- `settings-hydration.ts` is shared by App.tsx and the worker; the worker
  never writes settings and re-hydrates on `app-store://changed`.
- The worker follows the project the tab last opened (`restoreQueue`
  forwarded from the tab, `lastProject` on worker start).
- Worker activity is mirrored under a `worker:` id prefix; review items
  go through the inbox and are re-keyed by content id (`reviewIdFor`), so
  reviews the owner already resolved stay resolved.
- `ingest-queue-remote.test.ts` pins the remote module's exports to the
  real queue's.
- Verified end to end with a real model: a file imported in the web UI,
  tab closed immediately, the worker finished the ingest alone (source
  summary, concept pages, index/log, embeddings).
- Still in the tab: the dedup queue and file watching (see v1 limits).

Earlier alternatives kept for reference: keep ingest in the tab (resumes
on reopen) or port ingest to Rust (largest cost).

### Phase 5 — Web UX polish
- Server directory picker + drag-and-drop upload into `raw/sources/`.
- Export downloads the ZIP; import uploads it.
- PWA manifest; "open in system browser" links instead of `openUrl`.
- Surface file-tree load failures instead of keeping an empty path
  index: a tab opened while the server could not read the project kept
  every Sources/Related link marked missing until a manual reload.

---

## Risks

- **Path-traversal / arbitrary FS access.** Desktop commands accept
  absolute paths by design. Server must confine every path argument to
  registered project roots (central check in the RPC dispatcher).
- **Agent shell tool** becomes remote code execution on the server for
  anyone with the token. Off by default in server mode.
- **Concurrency:** two browser tabs = two ingest runners in Phase 1–3.
  Needs a server-side lease on the queue (`project-mutex.ts` is per-tab).
- **Big payloads:** `read_file_as_base64` / image extraction over HTTP;
  prefer `/files/*` streaming.
