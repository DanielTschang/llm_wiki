# 從階段 1 就導入 NATS JetStream

ingest 工作佇列和「頁面已提交」事件從階段 1 起就走 NATS JetStream，資料庫和訊息之間的一致性靠 Postgres outbox 保證。即使階段 1 只有一位使用者，也照樣導入。

這樣做的目的，是在還是單人版時，就先把 JetStream 的維運和 outbox 到 JetStream 的流程跑穩。到了階段 3 做平行 ingest、階段 4 讓 Python 外掛用 nats-py 訂閱事件時，就不必再從 Postgres 佇列遷移過去。

## Considered Options

- **階段 1–3 用 Postgres 工作表當佇列（`SKIP LOCKED` / pgmq），階段 4 才導入 JetStream**：前期少一套有狀態的叢集要維運，但等於要寫兩次佇列整合，而且要在功能最多的階段遷移。

## Consequences

- 開發用的 k3d 叢集三個 NATS 節點都在同一台機器上，只能驗證設定，不代表高可用。
- 階段 1–2 只有 Rust 是 NATS 客戶端。Rust 的 ingest-worker 角色取得工作後，交給 Node 子程序跑 TS ingest，再由 Rust 回 ack。
