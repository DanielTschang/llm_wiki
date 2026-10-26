#!/usr/bin/env bash
# Run the Postgres-backed tests against the k3d cluster's Postgres.
# Creates the throwaway `llm_wiki_test` database on first use, forwards the
# port for the duration of the run, and never touches other kube contexts.
# Extra arguments go to `cargo test`, e.g. `deploy/k3d/test-pg.sh -p llm-wiki-storage`.
set -euo pipefail
cd "$(dirname "$0")/../.."

CTX=k3d-llm-wiki
NS=llm-wiki
K=(kubectl --context "$CTX" -n "$NS")
PORT="${TEST_PG_PORT:-55432}"
DB=llm_wiki_test

"${K[@]}" exec pg-1 -c postgres -- psql -U postgres -d postgres -Atc \
  "SELECT 1 FROM pg_database WHERE datname = '$DB'" | grep -q 1 ||
  "${K[@]}" exec pg-1 -c postgres -- psql -U postgres -d postgres -c \
    "CREATE DATABASE $DB OWNER llm_wiki" >/dev/null
"${K[@]}" exec pg-1 -c postgres -- psql -U postgres -d "$DB" -c \
  "CREATE EXTENSION IF NOT EXISTS vector" >/dev/null

PASSWORD=$("${K[@]}" get secret pg-app -o jsonpath='{.data.password}' | base64 -d)

"${K[@]}" port-forward svc/pg-rw "$PORT":5432 >/dev/null &
FORWARD=$!
trap 'kill $FORWARD 2>/dev/null || true' EXIT
for _ in $(seq 1 50); do
  (exec 3<>"/dev/tcp/127.0.0.1/$PORT") 2>/dev/null && break
  sleep 0.2
done

export LLM_WIKI_TEST_DATABASE_URL="postgres://llm_wiki:${PASSWORD}@127.0.0.1:${PORT}/${DB}"
cd src-tauri
if [ "$#" -eq 0 ]; then set -- -p llm-wiki-storage; fi
cargo test "$@"
