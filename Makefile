# LLM Wiki — shortcuts. See START.md.
#
#   make help            list targets
#   make k3d-up          create the dev cluster (Postgres, NATS, MinIO, OTel)
#   make platform-dev    run the server against that cluster
#   make test-pg         run the Postgres/MinIO-backed tests

SHELL := /bin/bash

.DEFAULT_GOAL := help

.PHONY: help
help: ## List targets
	@awk 'BEGIN {FS = ":.*## "} /^[a-zA-Z0-9_-]+:.*## / {printf "  \033[36m%-16s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

# ── Setup ────────────────────────────────────────────────────────────────

.PHONY: install
install: ## Install npm dependencies (app + bundled MCP server)
	npm install
	npm --prefix mcp-server ci
	npm run mcp:build

# ── Frontend ─────────────────────────────────────────────────────────────

.PHONY: web
web: ## Build the browser frontend (dist-web/)
	npm run build:web

.PHONY: worker
worker: ## Build the server-side ingest worker (dist-worker/)
	npm run build:worker

.PHONY: web-dev
web-dev: ## Frontend dev server on :1430 against a running `make platform-dev`
	npm run dev:web

# ── Container image ──────────────────────────────────────────────────────

.PHONY: docker-build
docker-build: ## Build the server image (llm-wiki-server)
	docker buildx build --load -t llm-wiki-server .

# ── k3d dev cluster (deploy/k3d, plans/PLAN.md 階段 0) ─────────────────

.PHONY: k3d-up
k3d-up: ## Create the k3d dev cluster with Postgres, NATS, MinIO, OTel
	deploy/k3d/up.sh

.PHONY: k3d-down
k3d-down: ## Delete the k3d dev cluster and all its data
	deploy/k3d/down.sh

.PHONY: platform-dev
platform-dev: ## Run the platform server (accounts, projects, roles) on the k3d Postgres and MinIO
	deploy/k3d/dev-platform.sh

.PHONY: e2e-accounts
e2e-accounts: ## Check accounts, roles and permissions against the k3d Postgres and MinIO
	deploy/k3d/e2e-accounts.sh

.PHONY: e2e-k3d
e2e-k3d: ## Drive the built server over HTTP against the k3d Postgres and MinIO
	deploy/k3d/e2e-platform.sh

.PHONY: test-pg
test-pg: ## Run the Postgres- and MinIO-backed tests against the k3d cluster
	deploy/k3d/test-pg.sh

.PHONY: k3d-status
k3d-status: ## Show pods in the k3d dev cluster
	kubectl --context k3d-llm-wiki get pods -A

# ── Checks ───────────────────────────────────────────────────────────────

.PHONY: check
check: ## Type-check the frontend
	npm run typecheck

.PHONY: test
test: ## Run frontend (mocked) and Rust tests
	npm run test:mocks
	cargo test --manifest-path src-tauri/Cargo.toml
