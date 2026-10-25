# k3d dev cluster

本機開發用的叢集，包含 LLM Wiki 需要的基礎設施（plans/PLAN.md 階段 0）。三個節點都在同一台機器上，所以只能用來驗證設定，不代表高可用。

```bash
make k3d-up       # 建立或更新叢集（可重複執行）
make k3d-status   # 查看 pod
make k3d-down     # 刪除叢集和所有資料
```

kubectl context 是 `k3d-llm-wiki`，namespace 是 `llm-wiki`。腳本不會切換你目前的 context；請一律明確指定 `--context`。

| 元件 | 叢集內位址 | 帳密 |
| --- | --- | --- |
| Postgres 17 + pgvector（CloudNativePG） | `pg-rw:5432`，資料庫 `llm_wiki` | Secret `pg-app`（`username` / `password` / `uri`） |
| NATS JetStream（3 副本） | `nats://nats:4222` | 無 |
| MinIO | `http://minio:9000`（S3 API），console 在 `:9001` | Secret `minio` |
| OTel Collector | `otel-opentelemetry-collector:4317`（gRPC）/ `:4318`（HTTP） | 無 |

從本機連線：

```bash
kubectl --context k3d-llm-wiki -n llm-wiki port-forward svc/pg-rw 5432
kubectl --context k3d-llm-wiki -n llm-wiki port-forward svc/nats 4222
kubectl --context k3d-llm-wiki -n llm-wiki port-forward svc/minio 9000 9001
```

MinIO 已不再在 Docker Hub 和 quay.io 公開提供社群版 image，所以這裡改用 Chainguard 以上游原始碼建置的版本，並用 digest 固定。到了公司叢集，請換成內部維護的 MinIO image。
