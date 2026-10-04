# LLM Wiki — shortcuts for running each edition. See START.md.
#
#   make help            list targets
#   make desktop         desktop app (Tauri) in dev mode
#   make server          self-hosted web edition on http://127.0.0.1:19830
#   make docker-up       web edition in Docker

SHELL := /bin/bash

# Self-hosted server settings (override on the command line, e.g.
# `make server PORT=8080 ALLOW_ROOT=~/wikis`).
PORT       ?= 19830
BIND       ?= 127.0.0.1
DATA_DIR   ?=
TOKEN      ?=
# Folder with existing wiki projects the server may open. Defaults to
# ./projects when it exists.
ALLOW_ROOT ?= $(wildcard $(CURDIR)/projects)

CARGO_SERVER := cargo run --manifest-path src-tauri/Cargo.toml -p llm-wiki-server --
SERVER_ARGS  := --port $(PORT) --bind $(BIND) \
	$(if $(DATA_DIR),--data-dir $(DATA_DIR)) \
	$(if $(TOKEN),--token $(TOKEN)) \
	$(if $(ALLOW_ROOT),--allow-root $(ALLOW_ROOT))

DOCKER_IMAGE := llm-wiki-server

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

# ── Desktop (Tauri) ──────────────────────────────────────────────────────

.PHONY: desktop
desktop: ## Run the desktop app in dev mode (hot reload)
	npm run tauri dev

.PHONY: desktop-build
desktop-build: ## Build the desktop installer (src-tauri/target/release/bundle)
	npm run tauri build

# ── Self-hosted web edition ──────────────────────────────────────────────

.PHONY: web
web: ## Build the browser frontend (dist-web/)
	npm run build:web

.PHONY: worker
worker: ## Build the server-side ingest worker (dist-worker/)
	npm run build:worker

.PHONY: server
server: web worker ## Build web + worker, then run the server
	$(CARGO_SERVER) $(SERVER_ARGS)

.PHONY: server-only
server-only: ## Run the server without rebuilding the frontend or worker
	$(CARGO_SERVER) $(SERVER_ARGS)

.PHONY: web-dev
web-dev: ## Frontend dev server on :1430 against a running `make server-only`
	npm run dev:web

.PHONY: token
token: ## Print the server access token
	@cat $(if $(DATA_DIR),$(DATA_DIR),$(HOME)/.llm-wiki-server)/server-token; echo

# ── Docker ───────────────────────────────────────────────────────────────

.PHONY: docker-build
docker-build: ## Build the Docker image
	docker buildx build --load -t $(DOCKER_IMAGE) .

.PHONY: docker-up
docker-up: docker-build ## Start the web edition in Docker (127.0.0.1:19830)
	docker compose up -d
	@echo "Open http://127.0.0.1:19830 — token: make docker-token"

.PHONY: docker-down
docker-down: ## Stop the Docker container (data volume is kept)
	docker compose down

.PHONY: docker-logs
docker-logs: ## Follow the container logs
	docker compose logs -f

.PHONY: docker-token
docker-token: ## Print the access token stored in the Docker volume
	@docker compose exec llm-wiki cat /data/server-token; echo

# ── Checks ───────────────────────────────────────────────────────────────

.PHONY: check
check: ## Type-check the frontend
	npm run typecheck

.PHONY: test
test: ## Run frontend (mocked) and Rust tests
	npm run test:mocks
	cargo test --manifest-path src-tauri/Cargo.toml
