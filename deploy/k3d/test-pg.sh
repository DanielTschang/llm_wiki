#!/usr/bin/env bash
# Run the Postgres- and MinIO-backed tests against the k3d cluster.
# Creates the throwaway `llm_wiki_test` database and `llm-wiki-test` bucket on
# first use, forwards their ports for the duration of the run, and never
# touches other kube contexts. The bucket needs the `aws` CLI; without it the
# MinIO tests are skipped.
# Set RESET_TEST_DB=1 to drop and recreate the database first, which is needed
# after a migration that has already been applied is edited.
# Extra arguments go to `cargo test`, e.g. `deploy/k3d/test-pg.sh -p llm-wiki-storage`.
set -euo pipefail
cd "$(dirname "$0")/../.."

CTX=k3d-llm-wiki
NS=llm-wiki
K=(kubectl --context "$CTX" -n "$NS")
PORT="${TEST_PG_PORT:-55432}"
S3_PORT="${TEST_S3_PORT:-59000}"
DB=llm_wiki_test
BUCKET=llm-wiki-test

if [ "${RESET_TEST_DB:-}" = 1 ]; then
  "${K[@]}" exec pg-1 -c postgres -- psql -U postgres -d postgres -c \
    "DROP DATABASE IF EXISTS $DB WITH (FORCE)" >/dev/null
fi
"${K[@]}" exec pg-1 -c postgres -- psql -U postgres -d postgres -Atc \
  "SELECT 1 FROM pg_database WHERE datname = '$DB'" | grep -q 1 ||
  "${K[@]}" exec pg-1 -c postgres -- psql -U postgres -d postgres -c \
    "CREATE DATABASE $DB OWNER llm_wiki" >/dev/null
"${K[@]}" exec pg-1 -c postgres -- psql -U postgres -d "$DB" -c \
  "CREATE EXTENSION IF NOT EXISTS vector" >/dev/null

PASSWORD=$("${K[@]}" get secret pg-app -o jsonpath='{.data.password}' | base64 -d)

"${K[@]}" port-forward svc/pg-rw "$PORT":5432 >/dev/null &
PG_FORWARD=$!
"${K[@]}" port-forward svc/minio "$S3_PORT":9000 >/dev/null &
S3_FORWARD=$!
trap 'kill $PG_FORWARD $S3_FORWARD 2>/dev/null || true' EXIT
wait_for_port() {
  for _ in $(seq 1 50); do
    (exec 3<>"/dev/tcp/127.0.0.1/$1") 2>/dev/null && return 0
    sleep 0.2
  done
  return 1
}
wait_for_port "$PORT"

export LLM_WIKI_TEST_DATABASE_URL="postgres://llm_wiki:${PASSWORD}@127.0.0.1:${PORT}/${DB}"

if command -v aws >/dev/null && wait_for_port "$S3_PORT"; then
  S3_USER=$("${K[@]}" get secret minio -o jsonpath='{.data.MINIO_ROOT_USER}' | base64 -d)
  S3_PASSWORD=$("${K[@]}" get secret minio -o jsonpath='{.data.MINIO_ROOT_PASSWORD}' | base64 -d)
  AWS_ACCESS_KEY_ID="$S3_USER" AWS_SECRET_ACCESS_KEY="$S3_PASSWORD" AWS_EC2_METADATA_DISABLED=true \
    aws --endpoint-url "http://127.0.0.1:$S3_PORT" --region us-east-1 s3 mb "s3://$BUCKET" >/dev/null 2>&1 || true
  export LLM_WIKI_TEST_S3_ENDPOINT="http://127.0.0.1:$S3_PORT"
  export LLM_WIKI_TEST_S3_BUCKET="$BUCKET"
  export LLM_WIKI_TEST_S3_ACCESS_KEY="$S3_USER"
  export LLM_WIKI_TEST_S3_SECRET_KEY="$S3_PASSWORD"
else
  echo "skipping MinIO tests: need the aws CLI and a reachable svc/minio" >&2
fi

cd src-tauri
if [ "$#" -eq 0 ]; then set -- -p llm-wiki-storage; fi
cargo test "$@"
