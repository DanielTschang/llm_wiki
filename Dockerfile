# syntax=docker/dockerfile:1
#
# Self-hosted LLM Wiki: web frontend + llm-wiki-server in one image.
# See plans/web-server-mode.md and docker-compose.yml.
#
#   docker build -t llm-wiki-server .
#   docker run -p 127.0.0.1:19830:19830 -v llm-wiki-data:/data llm-wiki-server

# ── Web frontend + ingest worker ───────────────────────────────────────
FROM node:22-bookworm-slim AS web
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY index.html vite.config.ts tsconfig.json tsconfig.app.json tsconfig.node.json components.json .env.web .env.worker ./
COPY src ./src
COPY public ./public
RUN npm run build:web && npm run build:worker

# ── Server binary ──────────────────────────────────────────────────────
FROM rust:1-bookworm AS server
# lancedb's protobuf code generation needs protoc and the well-known
# types (google/protobuf/*.proto) from libprotobuf-dev.
RUN apt-get update \
    && apt-get install -y --no-install-recommends protobuf-compiler libprotobuf-dev \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
# The whole workspace manifest is needed to resolve the lockfile, and core
# embeds src/lib/source-watch-defaults.json at compile time.
COPY src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/build.rs ./src-tauri/
COPY src-tauri/src ./src-tauri/src
COPY src-tauri/crates ./src-tauri/crates
COPY src/lib/source-watch-defaults.json ./src/lib/source-watch-defaults.json
WORKDIR /app/src-tauri
# The workspace release profile (fat LTO, one codegen unit, size-optimized)
# is tuned for the desktop installer; for the server it mostly costs build
# time and memory.
ENV CARGO_PROFILE_RELEASE_LTO=thin \
    CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 \
    CARGO_PROFILE_RELEASE_OPT_LEVEL=2
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/src-tauri/target \
    cargo build --release --locked -p llm-wiki-server \
    && cp target/release/llm-wiki-server /usr/local/bin/llm-wiki-server

# ── PDFium for the target architecture (same binaries the desktop ships) ──
FROM debian:bookworm-slim AS pdfium
ARG TARGETARCH
COPY src-tauri/pdfium/libpdfium.so src-tauri/pdfium/libpdfium-arm64.so /pdfium/
RUN case "$TARGETARCH" in \
        amd64) cp /pdfium/libpdfium.so /libpdfium.so ;; \
        arm64) cp /pdfium/libpdfium-arm64.so /libpdfium.so ;; \
        *) echo "unsupported architecture: $TARGETARCH" >&2; exit 1 ;; \
    esac

# ── Runtime ────────────────────────────────────────────────────────────
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 1000 llmwiki \
    && mkdir -p /data /projects \
    && chown llmwiki:llmwiki /data /projects
COPY --from=server /usr/local/bin/llm-wiki-server /usr/local/bin/llm-wiki-server
COPY --from=pdfium /libpdfium.so /opt/llm-wiki/lib/libpdfium.so
COPY --from=web /app/dist-web /opt/llm-wiki/web
# The ingest worker (Phase 4) is a bundled Node script; only the node
# binary is needed at runtime.
COPY --from=web /usr/local/bin/node /usr/local/bin/node
COPY --from=web /app/dist-worker /opt/llm-wiki/worker
COPY docker/entrypoint.sh /usr/local/bin/docker-entrypoint

# Starts as root only to take ownership of a root-owned volume (Fly.io),
# then runs the server as `llmwiki` (docker/entrypoint.sh).
# Inside the container the server must listen on all interfaces; keep it
# private with the host port mapping (docker-compose.yml publishes to
# 127.0.0.1 only).
ENV HOME=/home/llmwiki \
    LLM_WIKI_DATA_DIR=/data \
    LLM_WIKI_BIND=0.0.0.0 \
    LLM_WIKI_PORT=19830 \
    LLM_WIKI_WEB_DIR=/opt/llm-wiki/web \
    LLM_WIKI_WORKER_SCRIPT=/opt/llm-wiki/worker/ingest-worker.mjs \
    LLM_WIKI_NODE=/usr/local/bin/node \
    PDFIUM_DYNAMIC_LIB_PATH=/opt/llm-wiki/lib/libpdfium.so
VOLUME ["/data"]
EXPOSE 19830
ENTRYPOINT ["docker-entrypoint"]
CMD ["llm-wiki-server"]
