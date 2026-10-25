# `.llm-wiki/` 狀態檔盤點

階段 0 產出。ADR-0002 決定這些檔案全部改成 PG 的正式資料表，不保留成 JSON。下表列出每個檔案的擁有者與建議的歸宿，**資料表欄位尚未設計**，那是階段 1 的 schema 工作。

「擁有者」是讀寫該檔的主要模組（`TS` 在 `src/`，`Rust` 在 `src-tauri/crates/`）。

| 檔案 | 擁有者 | 內容 | 建議歸宿 |
| --- | --- | --- | --- |
| `project.json` | TS `project-identity.ts`、`types/wiki.ts`；Rust `agent/commands.rs`、`api_server.rs` | Project 身分與設定（含 per-project LLM override） | `projects` 表＋`project_settings` |
| `review.json` | TS `persist.ts`；Rust `api_server.rs` | Review Item | `review_items` 表（Project 共用，記錄處理者） |
| `review-inbox.json` | Rust `server/rpc.rs`、`main.rs` | worker 用 `review_inbox_append` 寫入，瀏覽器用 `review_inbox_take` 取走 | **刪除**（直接寫 `review_items`） |
| `ingest-queue.json` | TS `ingest-queue.ts` | Ingest 佇列 | **刪除**，由 JetStream＋`ingest_jobs` 表取代 |
| `ingest-cache.json` | TS `ingest-cache.ts`；Rust `project_maintenance.rs` | SHA256 增量快取，避免重複 ingest | `ingest_cache` 表 |
| `ingest-progress/*.json` | TS `ingest.ts` | 每份 Source 的進度 | 併入 `ingest_jobs` |
| `ingest-generation/*.json` | TS `ingest.ts` | 產生階段中間結果（可續跑） | 併入 `ingest_jobs`，階段 3 變成 Changeset |
| `ingest-warnings.log` | TS `ingest.ts` | ingest 警告 | `ingest_events` 表（或只進 OTel） |
| `image-caption-cache.json` | TS `image-caption-pipeline.ts`、`ingest.ts` | 圖片說明快取 | `image_captions` 表（以圖片雜湊為鍵） |
| `dedup-queue.json` | TS `dedup-queue.ts` | 去重佇列 | `dedup_tasks` 表 |
| `dedup-not-duplicates.json` | TS `dedup-storage.ts` | 使用者標記「不是重複」的配對 | `dedup_exclusions` 表 |
| `conversations.json` | TS `persist.ts` | 對話列表 | `conversations` 表（屬於使用者＋Project） |
| `chats/<id>.json` | TS `persist.ts`、`chat-panel.tsx` | 單一對話內容 | `chat_messages` 表 |
| `chat-preferences.json` | TS `persist.ts` | 聊天偏好 | 使用者偏好 |
| `chat-history.json` | TS `persist.ts` | 舊版聊天紀錄 | **刪除**（舊格式） |
| `lint.json` | TS `persist.ts` | Lint 結果 | `lint_findings` 表 |
| `lint-config.json` | TS `lint-config.ts` | Lint 設定 | `project_settings` |
| `history-settings.json` | Rust `file_history.rs`、`agent/tools.rs` | 檔案歷史設定 | `project_settings` |
| `page-history/` | TS `ingest.ts`、`dedup.ts`、`dedup-runner.ts` | 頁面被覆寫前的備份 | `page_versions` 表（頁面歷史） |
| `history/`（`file_history.rs`） | Rust `file_history.rs` | 檔案歷史 | 併入 `page_versions` |
| `file-snapshot.json` | Rust `file_sync.rs` | 資料夾監控的快照 | **刪除**（資料夾監控不保留） |
| `file-change-queue.json` | Rust `file_sync.rs`、`api_server.rs` | 資料夾監控的變更佇列 | **刪除** |
| `scheduled-import-db.json` | TS `scheduled-import.ts` | 排程匯入 | 待確認：階段 4 的資料來源連接器取代，先**刪除** |
| `lancedb/` | Rust `vectorstore.rs` | 向量庫 | pgvector，embedding 依模型分表 |
| `embedding-revisions/` | Rust `page_embedding.rs` | 頁面嵌入的版本標記 | 併入 `page_embeddings` 的 revision 欄位 |
| `skills/**/SKILL.md` | Rust `agent/skills.rs` | Project 內的 Skills | `skills` 表（階段 4 網頁管理） |

另外兩個不在 `.llm-wiki/` 但同性質：`app-state.json`（全域設定，`app_store_*` RPC）要拆成「使用者偏好」「Project 設定」「伺服器端 LLM 設定（含 key）」三類。

## 待確認

- `scheduled-import-db.json`（TS `scheduled-import.ts`，約 650 行）：看起來是排程匯入資料夾，尚未讀完內容；暫定隨資料夾監控一起刪除，階段 4 由資料來源連接器取代。
- `history/`（`file_history.rs` 的檔案歷史）與 `page-history/`（ingest／dedup 覆寫前的備份）是兩套機制，併入 `page_versions` 時要確認不會重複。
- `.llm-wiki/` 底下的檔案本來就不允許 agent 讀取（`agent/context.rs` 有測試保證），改成資料表後這條保護自然成立，但要確保 agent 工具碰不到這些表。
- `wiki/` 與 `raw/` 底下的頁面本身不在此表，由 `storage` 介面處理（ADR-0002）。
