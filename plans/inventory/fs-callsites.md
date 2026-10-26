# 直接碰檔案系統的呼叫點

階段 1「storage 介面」的前置盤點。頁面搬進 Postgres 後，下列每個呼叫點都要改走 storage 介面、改成資料表，或明確留在本機磁碟。

方法：掃 `src-tauri/crates/{core,server}/src` 的非測試程式碼（排除 `#[cfg(test)] mod`），比對 `std::fs`、`tokio::fs`、`WalkDir`、`.exists()`、`.is_dir()`、`.canonicalize()` 等。共 **26 個檔案、約 320 處**；其中 `server/src/dispatch.rs` 的 18 處是呼叫 `core::commands::fs::*`，不是直接碰磁碟，實際約 300 處。

## A. fs RPC 本體：接縫的位置

`/rpc` 的檔案命令全部經過 `core/commands/fs.rs`，這裡是 storage 介面要插入的地方。TS worker 和前端只看得到這一層，**只要這一層依路徑分流，它們不用改**。

| 檔案 | 處數 | 內容 |
| --- | --- | --- |
| `commands/fs.rs` | 67 | `read_file`、`write_file`、`write_file_base64`、`write_file_atomic`、`list_directory`、`copy_*`、`delete_file`、`create_directory`、`file_exists`、`get_file_modified_time`、`read_file_as_base64`、`read_cache`／`write_cache`（預處理快取）、`apply_text_selection_edit_inner`、`create_missing_wiki_page_inner`、`find_related_wiki_pages`；另有 docx／pptx／odf／org 的文字抽取（讀本機暫存檔，見 D） |
| `commands/project.rs` | 12 | `create_project_impl`、`resolve_project_folder`、`resolve_path_in_project`、`validate_wiki_project_root`、`write_file_inner`：Project 的建立與路徑解析，改成查 `projects` 表 |
| `server/upload.rs` | 6 | 上傳先寫暫存檔再搬進 Project；改成直接串流進 MinIO |
| `server/paths.rs` | 2 | Project id 到資料夾路徑的對應，改成 `projects` 表 |

## B. 繞過 RPC、直接讀寫頁面的程式碼（最大的隱性工作）

這些不經過 `fs.rs`，所以光改 `/rpc` 不夠，各自要改。

| 檔案 | 處數 | 做什麼 | 處理 |
| --- | --- | --- | --- |
| `agent/tools.rs` | 51 | `read_wiki_page`、`write_wiki_page_with_activity`（create-only，避免 prompt injection 覆寫既有頁面）、`build_knowledge_graph_snapshot`（掃 `wiki/`）、`search_sources`（掃 `raw/`）、`safe_project_join` 一整組路徑圍欄 | 讀寫改走 storage；create-only 變成「`INSERT` 不覆寫」；符號連結與 `canonicalize` 防護整組消失，只留路徑正規化 |
| `commands/search.rs` | 11 | `search_project_inner` 走訪 `wiki/` 逐檔讀內容做分詞搜尋；`get_page_links_inner`；`materialize_vector_only_results` | 暫時改成從 storage 列出並讀取頁面；階段 3 換 tantivy 後整段重寫 |
| `api_server.rs` | 20 | `handle_file_content`、`list_tree`、`graph_files`／`build_graph_from_files`（圖譜）、`load_review_items`／`read_raw_review_array`／`write_raw_review_array`（`review.json`）、`read_project_id` | 圖譜改用 `page_links`；review 改 `review_items`；其餘走 storage。此檔階段 3 會連同 MCP 一併重寫 |
| `commands/page_embedding.rs` | 19 | 讀頁面做 embedding；`embedding-revisions/` 的 `load_revision`／`save_revision`／`invalidate_page_revision`；`remove_legacy_vector_id_if_safe` 走訪 `wiki/` | revision 改 `page_embedding_state`；舊版向量 id 清理是 LanceDB 時代遺留，可直接刪 |
| `agent/context.rs` | 5 | `load_explicit_context_files`：使用者指定的檔案放進 agent 上下文 | 走 storage |
| `commands/file_history.rs` | 43 | 檔案歷史，整套自建的磁碟版本庫 | **整個刪除**，由 `page_versions` 取代（`list_file_history`、`restore_file_history`、`clear_file_history`、設定與統計改成對該表的查詢） |

## C. 還在檔案系統上、但 schema 原本沒涵蓋的狀態

盤點時新發現兩項，已補進 schema：

- **`.llm-wiki/agent-sessions/<id>.json`**（`agent/session.rs`，5 處）：agent 跨回合的短期記憶。已新增 `agent_sessions` 表（`messages` 存 jsonb）。
- **`agent-workspace/`**（`agent/tools.rs` 的 `workspace.write_file`／`append_file`）：agent 產生的 HTML、SVG、PPT 等成品。已在 `blobs.kind` 加上 `'workspace'`，存進 MinIO。

## D. 留在本機磁碟，或本來就要消失

| 檔案 | 處置 |
| --- | --- |
| `agent/skills.rs`（12） | 目前掃 `<project>/.llm-wiki/skills`，**以及執行者家目錄的 `~/.claude/skills`、`~/.codex/skills`、`~/.agents/skills`**。多人伺服器上讀的是伺服器主機的家目錄，不是使用者的，等於把維運者的 skills 暴露給所有人。**這三個家目錄來源要立刻刪**，Project skills 改成 `skills` 表（階段 4）；在那之前只剩空清單 |
| `agent/runtime.rs`（13） | `read_active_skill_file` 等 skill 檔案讀取，跟著 skills 一起處理 |
| `agent/tools.rs` 的 shell／workspace 快照部分 | `run_shell_exec`、`snapshot_workspace_files`、`workspace_rollback_snapshot` 隨 shell 工具一起刪（PLAN 已列：Agent 移除 shell 與 AnyTXT） |
| `commands/extract_images.rs`（11）、`ebook.rs`（3）、`fs.rs` 的 docx／pptx／odf 抽取、`fs_anydoc_benchmark.rs`（3） | 解析器本來就要本機檔案。做法：從 MinIO 串流到暫存目錄，解析完刪除；產生的圖片寫進 MinIO。暫存目錄是這類程式碼的唯一磁碟用途 |
| `server/auth.rs`、`config.rs`、`main.rs`、`store.rs`、`desktop_settings.rs`、`core/proxy.rs`、`server_bind.rs`、`agent/commands.rs`（`load_agent_app_state`） | 伺服器自己的設定與 token，以及 `app-state.json`。`app-state.json` 拆成 `user_preferences`、`project_settings`、`server_settings`（見 `llm-wiki-state-files.md`）；token 與 key 走 k8s Secret；`desktop_settings.rs` 直接刪 |
| `server/rpc.rs`（4）的 `review_inbox` | 直接刪（worker 改寫 `review_items`） |

## 建議的施工順序

1. 先定 storage 介面（trait）：`read`／`write`（帶 `expected_version`）／`delete`／`list`／`exists`／`stat`，以 Project 和路徑為鍵；先做**記憶體實作**，讓上面 B 類程式碼能先改成走介面、跑現有測試，不依賴資料庫。
2. 再做 **Postgres + MinIO 實作**，用整合測試驗證，跑在 k3d 的叢集上。
3. `fs.rs` 接上介面（A 類），此時 TS worker 和前端已經能用 PG 運作。
4. 逐一處理 B 類：`file_history.rs` 和 `page_embedding.rs` 的 revision 最單純，先做；`search.rs`、`api_server.rs` 的圖譜與 `agent/tools.rs` 最後。
5. D 類的刪除項（家目錄 skills、shell 工具、`desktop_settings.rs`、legacy vector 清理）彼此獨立，可以隨時先做，不阻擋主線。
