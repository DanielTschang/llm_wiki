#!/usr/bin/env bash
# Runs the platform server against the k3d Postgres and MinIO, so you can use
# the web app with accounts, projects and permissions.
#
#   make web                       # build the frontend once (dist-web/)
#   cargo build -p llm-wiki-server # in src-tauri
#   deploy/k3d/dev-platform.sh     # then open the address it prints
#
# Models: the server holds the API keys. Export them before starting, under
# the names the model list refers to, e.g.
#   export LLM_WIKI_KEY_ANTHROPIC=sk-ant-...
# and then, signed in as the admin, add the models in Settings → LLM Models.
#
# The data lives in the `llm_wiki_dev` database and the `llm-wiki-test` bucket
# and is kept between runs. Press Ctrl-C to stop. Settings (all optional):
#   DEV_PORT=19830  DEV_DB=llm_wiki_dev  DEV_ADMIN_USER=admin  DEV_ADMIN_PASSWORD=...
#   DEV_RESET=1     drop the database first, for a clean start
set -euo pipefail
cd "$(dirname "$0")/../.."

K=(kubectl --context k3d-llm-wiki -n llm-wiki)
PORT="${DEV_PORT:-19830}"
DB="${DEV_DB:-llm_wiki_dev}"
ADMIN_USER="${DEV_ADMIN_USER:-admin}"
ADMIN_PASSWORD="${DEV_ADMIN_PASSWORD:-admin-password-dev}"
PG_PORT="${DEV_PG_PORT:-55436}"
S3_PORT="${DEV_S3_PORT:-59005}"
BUCKET=llm-wiki-test
BIN=src-tauri/target/debug/llm-wiki-server
[ -x "$BIN" ] || { echo "build the server first: (cd src-tauri && cargo build -p llm-wiki-server)" >&2; exit 1; }
[ -f dist-web/index.html ] || echo "note: dist-web/ is missing, run 'make web' to build the frontend" >&2

if [ "${DEV_RESET:-}" = 1 ]; then
  "${K[@]}" exec pg-1 -c postgres -- psql -U postgres -d postgres -c "DROP DATABASE IF EXISTS $DB WITH (FORCE)" >/dev/null
fi
"${K[@]}" exec pg-1 -c postgres -- psql -U postgres -d postgres -Atc "SELECT 1 FROM pg_database WHERE datname = '$DB'" | grep -q 1 ||
  "${K[@]}" exec pg-1 -c postgres -- psql -U postgres -d postgres -c "CREATE DATABASE $DB OWNER llm_wiki" >/dev/null
"${K[@]}" exec pg-1 -c postgres -- psql -U postgres -d "$DB" -c "CREATE EXTENSION IF NOT EXISTS vector" >/dev/null

PG_PASSWORD=$("${K[@]}" get secret pg-app -o jsonpath='{.data.password}' | base64 -d)
S3_USER=$("${K[@]}" get secret minio -o jsonpath='{.data.MINIO_ROOT_USER}' | base64 -d)
S3_PASSWORD=$("${K[@]}" get secret minio -o jsonpath='{.data.MINIO_ROOT_PASSWORD}' | base64 -d)

# `kubectl port-forward` exits when one connection is reset, and the server
# then hangs on its next query (logins time out). Restart each forward when it
# dies; the server's pool reconnects through the same local port.
forward() {
  while true; do
    "${K[@]}" port-forward "$@" >/dev/null 2>&1
    sleep 1
  done
}
forward svc/pg-rw "$PG_PORT":5432 &
PG_FORWARD=$!
forward svc/minio "$S3_PORT":9000 &
S3_FORWARD=$!
DATA=$(mktemp -d)
# Stopping the loop leaves its kubectl child behind, so stop the children too.
stop_forwards() {
  for loop in "$PG_FORWARD" "$S3_FORWARD"; do
    kill "$loop" 2>/dev/null || true
    pkill -P "$loop" 2>/dev/null || true
  done
}
trap 'stop_forwards; rm -rf "$DATA"' EXIT
for p in "$PG_PORT" "$S3_PORT"; do
  for _ in $(seq 1 50); do (exec 3<>"/dev/tcp/127.0.0.1/$p") 2>/dev/null && break; sleep 0.2; done
done
if command -v aws >/dev/null; then
  AWS_ACCESS_KEY_ID="$S3_USER" AWS_SECRET_ACCESS_KEY="$S3_PASSWORD" AWS_EC2_METADATA_DISABLED=true \
    aws --endpoint-url "http://127.0.0.1:$S3_PORT" --region us-east-1 s3 mb "s3://$BUCKET" >/dev/null 2>&1 || true
fi

echo
echo "  LLM Wiki (platform) on http://localhost:$PORT"
echo "  sign in as  $ADMIN_USER  with the password  $ADMIN_PASSWORD  (the first run creates it)"
echo
LLM_WIKI_DATA_DIR="$DATA" LLM_WIKI_PORT="$PORT" \
LLM_WIKI_TOKEN="dev-operator-token-$PORT" \
LLM_WIKI_ADMIN_USER="$ADMIN_USER" LLM_WIKI_ADMIN_PASSWORD="$ADMIN_PASSWORD" \
LLM_WIKI_DATABASE_URL="postgres://llm_wiki:$PG_PASSWORD@127.0.0.1:$PG_PORT/$DB" \
LLM_WIKI_S3_ENDPOINT="http://127.0.0.1:$S3_PORT" LLM_WIKI_S3_BUCKET="$BUCKET" \
LLM_WIKI_S3_ACCESS_KEY="$S3_USER" LLM_WIKI_S3_SECRET_KEY="$S3_PASSWORD" \
  "$BIN" --web-dir dist-web
