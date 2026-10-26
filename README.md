# LLM Wiki

A multi-user knowledge platform. People put source documents into a **Project**; an LLM turns them into linked wiki pages that the whole Project can search, chat with and query through an agent. Access is by role (Owner, Editor, Viewer) per Project.

This is a rewrite of [nashsu/llm_wiki](https://github.com/nashsu/llm_wiki) from a desktop app into a web platform for 50–200 users on a company Kubernetes cluster. The idea behind the wiki itself comes from [llm-wiki.md](llm-wiki.md).

- Vocabulary and the decisions behind it: [CONTEXT.md](CONTEXT.md), [docs/adr/](docs/adr/)
- Plan, status and open work: [plans/PLAN.md](plans/PLAN.md)
- Database schema: [plans/schema.md](plans/schema.md)

## Architecture

| Part | Where | What it does |
|---|---|---|
| Frontend | `src/` | React web app, built to `dist-web/` and served by the server |
| Ingest worker | `src/worker/` | Node script the server supervises; reads Sources, writes pages |
| Server | `src-tauri/crates/server` | axum: accounts and sessions, role checks, `/rpc`, uploads, SSE events, LLM gateway |
| Core | `src-tauri/crates/core` | Search, the agent, document parsing, commands |
| Storage | `src-tauri/crates/storage` | Pages, links and versions in Postgres; files in S3 (MinIO); vectors in pgvector; accounts and settings |

Browsers never hold an LLM API key: models are configured by a Platform Admin and called through the server's gateway.

## Run it locally

You need Docker, `k3d`, `kubectl`, Node.js 22+ and Rust.

```bash
make install        # npm dependencies
make k3d-up         # Postgres (pgvector), NATS, MinIO, OTel in a local k3d cluster
make web worker     # build the frontend and the ingest worker
make platform-dev   # run the server against that cluster
```

Open the address it prints and sign in as `admin` (the script prints the password). Add models under Settings → LLM Models; their API keys are read from `LLM_WIKI_KEY_*` environment variables of the server process.

[START.md](START.md) has the details (in Chinese).

## Tests

```bash
make test-pg        # Rust tests, including the Postgres and MinIO ones
make e2e-accounts   # accounts, roles, gateway, search and ingest over HTTP
make test           # frontend (mocked) and Rust tests
```

## Moving a folder project in

Projects from the old folder layout can be copied into the platform once:

```bash
llm-wiki-import --dir path/to/project --name "My project"
```

## License

GPL-3.0, see [LICENSE](LICENSE).
