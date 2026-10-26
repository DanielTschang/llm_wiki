# Postgres schema（階段 1）

Migration 在 `src-tauri/crates/storage/migrations/`，純 SQL，依序套用：

| 檔案 | 內容 |
| --- | --- |
| `0001_identity.sql` | `users`、`projects`、`project_members`、`project_settings`、`user_preferences`、`server_settings`、`activity_log` |
| `0002_content.sql` | `pages`、`page_versions`、`page_links`、`blobs`、`page_sources` |
| `0003_ingest_and_review.sql` | `agent_sessions`、`ingest_jobs`、`ingest_events`、`ingest_cache`、`image_captions`、`dedup_tasks`、`dedup_exclusions`、`review_items`、`lint_findings`、`conversations`、`chat_messages` |
| `0004_embeddings.sql` | `embedding_models`、`page_embedding_state`、`register_embedding_model()`（每個模型一張 `chunks_m<id>`） |
| `0005_outbox.sql` | `outbox` |

已在 k3d 的 Postgres 17 + pgvector 0.8.7 上套用並跑過冒煙測試（路徑唯一性、軟刪除後路徑可重用、樂觀鎖、同一 Source 只能有一個進行中 job、Review Item 重複寫入冪等、模型註冊與維度衝突、依 Project 過濾的向量查詢、刪除 Project 的連鎖刪除）。之後由 storage crate 的整合測試接手。

## 設計決定

**id 與範圍。** 所有 Project 範圍的表都帶 `project_id`，並 `ON DELETE CASCADE` 到 `projects`。瀏覽器或 worker 原本就產生 id 的東西（Review Item、對話、訊息、dedup 任務）維持 `text`，主鍵是 `(project_id, id)`，這樣 fs 接縫後面看到的 id 語意不變。新產生的實體用 `uuid`。

**頁面。**
- `pages` 放目前狀態（含 `content`），讀取不用 join；`page_versions` 是 append-only 的寫入紀錄，每次寫入（含建立、刪除）都一列。同一份最新內容因此存兩次，換來歷史和還原都只查一張表；以 50–200 人的規模這個成本可以接受。
- 取代 `page-history/` 與 `history/` 兩套機制（盤點裡的「待確認」）：兩者都是「寫入前的備份」，統一成版本列；`tool` 欄位標出寫入來源。
- 刪除是軟刪除（`deleted_at`），活的路徑才有唯一性（partial unique index），所以刪掉後同一路徑可以重建，Changeset 也能還原。
- rename 與 delete 也各寫一筆版本紀錄（`op` 為 `rename`、`delete`），且都讓 `version` 加一；因此改名後，手上拿著舊版本號的寫入者會得到版本衝突，這是刻意的。
- 版本衝突靠 `version`：寫入者帶著讀到的版本，`UPDATE … WHERE version = $n` 沒命中就是衝突。階段 3 的提交鎖（advisory lock）疊在這上面。
- 路徑語意與資料夾時代相同（相對 Project 根、正斜線），CHECK 擋掉 `../`、開頭 `/` 和 `//`。目錄不是資料列，列目錄時由路徑前綴推出，副作用是空目錄不存在。
- 路徑宇宙：`wiki/**` 的文字檔加上根目錄的 `purpose.md`、`schema.md` 進 `pages`；二進位（`raw/**`、`wiki/media/**`）進 `blobs`。分流規則寫在 storage 介面，不在 schema。

**連結與出處。** `page_links` 存 `[[ ]]` 邊，`target_page_id` 為 NULL 就是斷鏈（lint 的來源）；`page_sources` 存 frontmatter `sources:` 對應的 Source，讓「刪這份 Source 會影響哪些頁面」是查詢而不是掃描。兩者都在寫頁面的同一個交易裡重建。其他頁面新增或刪除時要重新解析 `target_ref` 相符的邊，這段邏輯在 storage 介面實作。

**Blob。** MinIO 物件鍵是 `<project_id>/<blob id>`，改名、搬移只改資料列；刪 Project 時是一次前綴刪除。上傳與下載經 API 串流，不用 presigned URL。刪除 Project 時資料庫連鎖刪除，**MinIO 物件要由程式先刪**，順序是先刪物件再刪資料列，否則會留下孤兒物件。

**設定與祕密。** `project_settings`、`user_preferences`、`server_settings` 都是 key／jsonb，直接對應原本 `app_store_*` 的語意。**API key 不放資料庫**，放 k8s Secret／環境變數，`server_settings` 只放模型清單、base URL 等非機密設定。

**Ingest。** `ingest_jobs` 取代佇列、進度、中間結果三種檔案；派工走 JetStream，這張表是可持久的狀態。`generation` 欄位存可續跑的中間結果，階段 3 變成 Changeset。同一份 Source 只允許一個 `pending`／`processing` 的 job。`page_versions.ingest_job_id` 把同一次 ingest 寫的頁面串起來，這就是階段 3 Changeset 的前身，所以現在**不另建 `changesets` 表**。

**向量。**
- 每個 embedding 模型一張 `chunks_m<id>`，由 `register_embedding_model(model, dims)` 建表與 HNSW 索引（冪等；維度衝突會報錯）。2000 維以內用 `vector`，到 4000 維用 `halfvec`，再大就拒絕。
- 欄位照搬 LanceDB 的 `wiki_chunks_v2`，換庫是直接替換。距離用 **L2**，跟 LanceDB 一致，`score = 1 / (1 + distance)` 和既有門檻不變；改成 cosine 只是換運算子，之後要換很便宜。
- `page_embedding_state` 取代 `embedding-revisions/`：記錄每個（頁面，模型）最後嵌入的內容指紋。
- 依 Project 過濾的查詢要設 `hnsw.iterative_scan = relaxed_order`，否則過濾後可能回傳不足 K 筆。資料量大時的延遲在階段 3 評估 pgvector 是否足夠（PLAN 已列）。

**Outbox。** 狀態變更與事件在同一個交易寫入，relay 用 `FOR UPDATE SKIP LOCKED` 取未發布的列。`msg_id` 當作 `Nats-Msg-Id`，relay 重送時由 stream 去重。已發布的列由定期工作清掉。

**操作紀錄。** `activity_log` 記成員異動、上傳、刪除、設定變更等「誰做了什麼」；頁面內容的歷史在 `page_versions`，兩者不重複。

## 刻意沒做

- **`changesets`、`skills`、`plugins`、`service account 的憑證`、`llm_usage`（只統計，不做成本門檻通知）**：各自屬於階段 2–4，到時候加 migration。`users.kind = 'service'` 先把 Service Account 的身分位置留好。
- **Row-level security**：Project 權限在 API middleware 檢查。RLS 可以日後加成第二道防線，現在不加，免得每個連線都要設定使用者 context。
- **全文檢索**：搜尋要等階段 3 的 tantivy，PG 不放分詞索引。

## 交給 storage 介面的問題

- `search.rs`（約 80 KB）、圖譜、`file_history.rs` 等現在都直接讀 Project 資料夾。換成 PG 之後它們必須改走 storage 介面，這是「storage 介面」那一項最大的工作量，應先列出所有直接碰檔案系統的呼叫點。
- 認證方式（OIDC／LDAP）還沒決定，`users.external_subject` 先留著，階段 2 做多人化時才定。
