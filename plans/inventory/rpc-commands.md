# 前端 `invoke` command 盤點

階段 0 產出。範圍：`src/` 裡所有非測試的 `invoke("…")` 呼叫，共 **75 個不同的 command**。server 的 `dispatch.rs` 有 65 個 match 分支；前端用到但不在 `dispatch.rs` 的有 20 個（見文末）。

「去向」指階段 1 重整後該 command 屬於哪個 crate／路由；**刪除**表示隨桌面版或不再保留的功能一起移除。

## 檔案與專案（→ `storage`）

| command | 呼叫位置 | 去向 |
| --- | --- | --- |
| `read_file`、`write_file`、`write_file_atomic`、`delete_file`、`copy_file`、`copy_directory`、`create_directory`、`list_directory`、`file_exists`、`get_file_size`、`get_file_modified_time` | `commands/fs.ts` | `storage` 頁面儲存介面：`wiki/` 進 PG、`raw/` 進 MinIO。**路徑語意不變**（ADR-0002） |
| `apply_text_selection_edit`、`create_missing_wiki_page` | `commands/fs.ts` | `storage`；編輯要帶版本號（階段 2） |
| `preprocess_file` | `commands/fs.ts` | `parsing`（Source 解析） |
| `create_project`、`open_project`、`open_project_folder`、`open_path_in_project` | `commands/fs.ts` | 改成 Project 範圍的路由；`open_*_folder`／`open_path_in_project` 是桌面行為，**刪除** |
| `server_locations` | `server-path-picker.tsx`、`maintenance-section.tsx` | **刪除**（伺服器檔案瀏覽器；Project 不再對應伺服器路徑） |
| `export_project_archive`、`import_project_archive` | `maintenance-section.tsx` | 匯出保留（階段 2，zip）；**匯入刪除** |
| `rebuild_wiki_index` | `maintenance-section.tsx` | `storage`／後續處理（index.md 改由 DB 產生，階段 3） |
| `get_page_links`、`find_related_wiki_pages` | `commands/fs.ts` | `retrieval`（wikilink 邊改查 DB） |

## 檔案歷史（→ `storage`）

`list_file_history`、`restore_file_history`、`clear_file_history`、`get_file_history_settings`、`set_file_history_settings`（`commands/fs.ts`）→ 併入頁面版本歷史表；`history-settings` 改成 Project 設定。

## 檔案監控（→ 刪除）

`start_project_file_watcher`、`stop_project_file_watcher`、`rescan_project_files`、`get_file_change_queue`、`retry_file_change_task`、`ignore_file_change_task`、`invalidate_project_file_snapshot_paths`（`commands/file-sync.ts`）。資料夾自動監控不再保留，階段 4 由資料來源連接器取代。

## 檢索與向量（→ `retrieval`）

| command | 呼叫位置 | 去向 |
| --- | --- | --- |
| `search_project` | `lib/search.ts` | `retrieval`；介面接受 Project 清單 |
| `vector_upsert_chunks`、`vector_search_chunks`、`vector_delete_page`、`vector_clear_chunks`、`vector_count_chunks`、`vector_optimize_chunks` | `lib/embedding.ts` | `retrieval`，後端從 LanceDB 換成 pgvector |
| `vector_drop_legacy`、`vector_legacy_row_count` | `lib/embedding.ts` | **刪除**（舊版 LanceDB 資料表遷移） |
| `embedding_fetch`、`embedding_fetch_batch` | `lib/embedding.ts` | `llm`（embedding 呼叫，受 Project 模型限制） |

## Agent（→ `agent`）

`agent_start_turn`、`agent_start_turn_stream`、`agent_cancel_turn`、`agent_list_skills`（`chat-panel.tsx`、`skills-section.tsx`）→ `agent`；核准流程改成暫停＋SSE（階段 2）；Skills 改為網頁管理（階段 4）。

## LLM 與外部請求（→ `llm`）

| command | 呼叫位置 | 去向 |
| --- | --- | --- |
| `web_search` | `lib/web-search.ts` | `llm`／`retrieval` 旁的外部搜尋服務，key 放伺服器 |
| `anytxt_search` | `lib/anytxt-search.ts` | **刪除**（本機專屬工具） |
| `claude_cli_*`（detect／spawn／kill）、`codex_cli_*`（同） | `claude-cli-transport.ts`、`codex-cli-transport.ts`、`llm-provider-section.tsx` | **刪除**（server 的 `SERVER_DISABLED` 已停用） |
| `set_proxy_env` | `settings-view.tsx` | 改成伺服器端設定，由 Platform Admin 管理 |

## Ingest worker 協定（→ `api` + ingest-worker）

`ingest_command`、`review_inbox_take`（`ingest-queue-remote.ts`）、`worker_heartbeat`、`worker_publish`、`worker_reply`、`review_inbox_append`（`worker/main.ts`）→ 階段 1 改接 JetStream（Rust 取得工作後交給 Node 子程序，再回 ack）；`review_inbox_*` 隨 Review Item 一起改存 PG。

## 設定與 API（→ `api`）

| command | 呼叫位置 | 去向 |
| --- | --- | --- |
| `app_store_get`／`set`／`delete` | `platform/web.ts` | 拆成「使用者偏好」和「Project 設定」兩類，存 PG；LLM 設定與 key 移到伺服器端設定 |
| `api_server_status`、`api_server_reload_config` | `commands/fs.ts`、`settings-view.tsx` | **刪除**（階段 3 的遠端 MCP 取代） |
| `clip_server_status` | `commands/fs.ts` | **刪除**（Web Clipper） |
| `mcp_server_entry_path` | `commands/fs.ts` | **刪除**（本機 MCP；階段 3 改遠端 MCP） |
| `set_close_behavior` | `App.tsx`、`settings-view.tsx` | **刪除**（桌面行為） |

## 小結

- 刪除：**25 個**（桌面行為、檔案監控 7 個、CLI transport 6 個、Clipper、本機 MCP、API server 相關、舊版向量遷移、`anytxt_search`、`server_locations`、`import_project_archive`）
- 保留並重整：**50 個**，其中檔案與專案、檔案歷史合計約 25 個，多半是路徑語意不變的搬遷

## 前端用到、但不在 `dispatch.rs` 的 command（20 個）

這些由 server 的其他程式碼處理（`rpc.rs`、`worker.rs`、`store.rs` 等），或是桌面專屬所以根本沒有 server 實作：

- 設定儲存：`app_store_get`／`set`／`delete`
- Ingest worker 協定：`ingest_command`、`worker_heartbeat`、`worker_publish`、`worker_reply`、`review_inbox_append`、`review_inbox_take`
- 檔案瀏覽：`server_locations`
- CLI transport（`SERVER_DISABLED`）：`claude_cli_*`、`codex_cli_*`（共 6 個）
- 桌面專屬：`open_project_folder`、`open_path_in_project`、`set_close_behavior`、`mcp_server_entry_path`

## `dispatch.rs` 有、但前端沒在用的 command（10 個）

`agent_get_session`、`agent_list_sessions`、`extract_pdf_images_cmd`、`extract_office_images_cmd`、`extract_and_save_pdf_images_cmd`、`extract_and_save_office_images_cmd`、`vector_count`、`vector_delete`、`vector_search`、`vector_upsert`。

- 後四個是舊版（非 chunk）向量介面，已被 `vector_*_chunks` 取代，可以直接刪除。
- 圖片抽取的四個 command 前端沒有直接呼叫，用途尚待確認（可能由 `read_file` 的 `extractImages` 參數在 Rust 內部使用），確認後歸入 `parsing`；agent session 的兩個是否保留，要看階段 2 的對話儲存設計。
