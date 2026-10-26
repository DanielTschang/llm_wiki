#!/usr/bin/env bash
# Keeps Postgres (localhost:15432) and the MinIO console (http://localhost:9001)
# reachable from local GUI tools. `kubectl port-forward` exits whenever one
# connection is reset, so each forward is restarted in a loop.
set -u
CTX=k3d-llm-wiki
NS=llm-wiki

forward() {
  while true; do
    kubectl --context "$CTX" -n "$NS" port-forward "$@" >/dev/null 2>&1
    sleep 1
  done
}

forward svc/pg-rw 15432:5432 &
forward svc/minio 9001:9001 &
trap 'kill $(jobs -p) 2>/dev/null' EXIT INT TERM

echo "Postgres : localhost:15432  user llm_wiki  db llm_wiki_dev  (SSL off)"
echo "  password: kubectl --context $CTX -n $NS get secret pg-app -o jsonpath='{.data.password}' | base64 -d"
echo "MinIO    : http://localhost:9001  user llm-wiki  password llm-wiki-dev-only"
echo "Ctrl-C to stop."
wait
