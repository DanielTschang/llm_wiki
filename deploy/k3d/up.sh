#!/usr/bin/env bash
# Create (or update) the LLM Wiki k3d dev cluster and its infrastructure.
# Every command names the context explicitly so other clusters in the
# kubeconfig are never touched.
set -euo pipefail
cd "$(dirname "$0")"

CTX=k3d-llm-wiki
NS=llm-wiki
K=(kubectl --context "$CTX")
H=(helm --kube-context "$CTX")

if ! k3d cluster list llm-wiki >/dev/null 2>&1; then
  k3d cluster create --config cluster.yaml
fi

"${K[@]}" create namespace "$NS" --dry-run=client -o yaml | "${K[@]}" apply -f -

helm repo add cnpg https://cloudnative-pg.github.io/charts >/dev/null
helm repo add nats https://nats-io.github.io/k8s/helm/charts >/dev/null
helm repo add open-telemetry https://open-telemetry.github.io/opentelemetry-helm-charts >/dev/null
helm repo update cnpg nats open-telemetry >/dev/null

"${H[@]}" upgrade --install cnpg cnpg/cloudnative-pg \
  --namespace cnpg-system --create-namespace --wait
"${K[@]}" apply -f postgres.yaml

"${H[@]}" upgrade --install nats nats/nats --namespace "$NS" \
  -f nats-values.yaml --wait

"${K[@]}" apply -f minio.yaml

"${H[@]}" upgrade --install otel open-telemetry/opentelemetry-collector \
  --namespace "$NS" -f otel-values.yaml --wait

"${K[@]}" -n "$NS" wait --for=condition=Ready cluster.postgresql.cnpg.io/pg --timeout=300s
"${K[@]}" -n "$NS" rollout status statefulset/minio --timeout=300s

echo "LLM Wiki dev cluster is ready (context $CTX, namespace $NS)."
