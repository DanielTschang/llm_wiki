#!/usr/bin/env bash
# End to end for accounts and permissions: the real server with a throwaway
# database, driven as an administrator, editors, viewers and an outsider.
# Needs `make test-pg` to have run once (for the bucket) and the server built
# with `cargo build -p llm-wiki-server` in src-tauri.
set -uo pipefail
cd "$(dirname "$0")/../.."
K=(kubectl --context k3d-llm-wiki -n llm-wiki)
DB=llm_wiki_e2e_accounts
PGPORT=55435; S3PORT=59004; APIPORT=19879
cleanup() {
  kill ${PF1:-} ${PF2:-} ${SRV:-} 2>/dev/null
  rm -rf "${DATA:-}"
  "${K[@]}" exec pg-1 -c postgres -- psql -U postgres -d postgres -c "DROP DATABASE IF EXISTS $DB WITH (FORCE)" >/dev/null 2>&1
}
trap cleanup EXIT

"${K[@]}" port-forward svc/pg-rw $PGPORT:5432 >/dev/null 2>&1 & PF1=$!
"${K[@]}" port-forward svc/minio $S3PORT:9000 >/dev/null 2>&1 & PF2=$!
sleep 3
PGPASS=$("${K[@]}" get secret pg-app -o jsonpath='{.data.password}' | base64 -d)
S3U=$("${K[@]}" get secret minio -o jsonpath='{.data.MINIO_ROOT_USER}' | base64 -d)
S3P=$("${K[@]}" get secret minio -o jsonpath='{.data.MINIO_ROOT_PASSWORD}' | base64 -d)
"${K[@]}" exec pg-1 -c postgres -- psql -U postgres -d postgres -c "DROP DATABASE IF EXISTS $DB WITH (FORCE)" >/dev/null
"${K[@]}" exec pg-1 -c postgres -- psql -U postgres -d postgres -c "CREATE DATABASE $DB OWNER llm_wiki" >/dev/null
"${K[@]}" exec pg-1 -c postgres -- psql -U postgres -d $DB -c "CREATE EXTENSION IF NOT EXISTS vector" >/dev/null

DATA=$(mktemp -d)
export OPERATOR_TOKEN=e2e-operator-$RANDOM$RANDOM ADMIN_USER=admin ADMIN_PASSWORD=e2e-admin-$RANDOM$RANDOM
LLM_WIKI_DATA_DIR=$DATA LLM_WIKI_PORT=$APIPORT LLM_WIKI_TOKEN=$OPERATOR_TOKEN LLM_WIKI_NO_DESKTOP_SETTINGS=true \
LLM_WIKI_ADMIN_USER=$ADMIN_USER LLM_WIKI_ADMIN_PASSWORD=$ADMIN_PASSWORD LLM_WIKI_KEY_STUB=sk-stub-secret \
LLM_WIKI_DATABASE_URL="postgres://llm_wiki:$PGPASS@127.0.0.1:$PGPORT/$DB" \
LLM_WIKI_S3_ENDPOINT=http://127.0.0.1:$S3PORT LLM_WIKI_S3_BUCKET=llm-wiki-test LLM_WIKI_S3_ACCESS_KEY=$S3U LLM_WIKI_S3_SECRET_KEY=$S3P \
  src-tauri/target/debug/llm-wiki-server >"$DATA/server.log" 2>&1 & SRV=$!
for _ in $(seq 1 100); do curl -s -o /dev/null http://127.0.0.1:$APIPORT/healthz && break; sleep 0.3; done

[ -f dist-worker/ingest-worker.mjs ] || echo "note: dist-worker/ is missing (make worker); the ingest checks will fail" >&2
BASE=http://127.0.0.1:$APIPORT python3 deploy/k3d/e2e_accounts.py
status=$?
[ $status -ne 0 ] && { echo "--- server log"; tail -20 "$DATA/server.log"; }
exit $status
