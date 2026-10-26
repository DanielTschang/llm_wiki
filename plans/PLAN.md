# LLM Wiki 平台化改寫計畫

Oct 26, 2026 · @Daniel Tschang

> 給 Claude Code：這是 LLM Wiki 平台化改寫的主計畫。架構以「已確定的架構決策」和「決策紀錄」為準，術語以 `CONTEXT.md` 為準，重大取捨見 `docs/adr/`。完成任務時把對應的 `- [ ]` 改成 `- [x]`，有新決策時更新決策紀錄。

## 目標與範圍

把 nashsu/llm_wiki 從單人桌面程式改寫成多人使用的 web 知識平台：Rust 負責資料和效能，Python 負責擴充，核心演算法和 prompt 沿用 nashsu 的設計。

**起點：**`plans/web-server-mode.md` 已完成單人自架版。有不依賴 Tauri 的 `llm-wiki-core`、axum 寫的 `llm-wiki-server`（`/rpc` 一對一對應 `invoke`、SSE、`/proxy` 轉送 LLM 請求、單一 token），以及在伺服器端用 Node 跑 TS ingest 的 worker。本計畫在這個基礎上演進：**沿用程式碼，但不沿用它的形狀**（帶路徑的遠端 invoke、瀏覽器組 LLM 請求、照 Tauri command 分類的 core）。

**要做的：**

- 拆掉 Tauri 桌面版，全面 web 化（前端沿用 React）
- 現有的 core 依領域拆成多個 crate，server 改成以 Project 為範圍的 API
- ingest 從 TypeScript 改寫成 Rust，一次改寫成可平行處理的分段流程
- retrieval 保留 nashsu 的演算法，改成伺服器端執行
- agent 沿用 nashsu 的 Rust agent，改成多人版
- 支援多人、多 Project、Project 層級權限
- 讓只會 Python 的同事能透過固定的 Extension Point 擴充平台

**不做的（目前）：**

- 不引入 iii 和 deepagents
- 不做即時協作編輯
- 不保留 Web Clipper
- 不做頁面層級權限
- 不做既有資料夾 project 的匯入工具
- 不做 LLM 配額（只統計）
- 長期分叉，不和上游 nashsu 版本同步

## 已確定的架構決策

| 主題 | 決策 | 理由 |
| --- | --- | --- |
| 部署形態 | 全面 web 化，移除 Tauri | 只剩一個部署目標，不用再處理跨平台問題 |
| 部署環境 | 公司內部自架 k8s，所有元件自己架；開發用 k3d | 公司內部使用；先在本機驗證設定 |
| 使用範圍 | 只在公司內部使用 | 不觸發 GPL v3 的散布義務 |
| 上游策略 | 長期分叉 | 架構改動大，不追上游；有價值的改進再手動評估 |
| 既有程式碼 | 在 `llm-wiki-core` / `llm-wiki-server` 上演進；沿用 agent、搜尋演算法、解析、LLM provider，重整介面形狀 | 只有一人會 Rust，最貴的部分不重寫 |
| 後端組織 | Rust 模組化單體，依領域分 crate（storage、retrieval、agent、parsing、llm、api），同一映像依角色部署（api、ingest-worker） | 只有一人會 Rust，少一個服務就少一處要維護 |
| API 形狀 | 以 Project 為範圍的路由（`/api/projects/{id}/…`）加上身分、權限 middleware；`/rpc` 只在過渡期保留，到階段 2 刪除 | 每個請求都要帶著「誰」和「哪個 Project」 |
| Ingest | 階段 1–2 沿用 TS worker；階段 3 一次改寫成 Rust，拆成「思考」（可平行）和「寫入」（每個 Project 單一寫入者） | 只寫一次 Rust，而且直接寫成最終形狀 |
| 產生階段輸出 | Changeset，不直接寫檔；預設自動提交，可還原 | 可重試、可事後審查、可偵測衝突 |
| Retrieval | 演算法沿用 nashsu，分詞搜尋改用 tantivy 預建索引，圖譜記憶體快取 | 保留品質，解決多人規模下的效能 |
| 儲存 | 頁面（markdown + frontmatter + 版本號）存 Postgres；Source 和圖片放 MinIO；`.llm-wiki/*.json` 改成資料表；向量用 pgvector（取代 LanceDB）；透過 fs 介面接縫存取（ADR-0002） | 一致的交易與鎖；少一套系統 |
| 檔案存取 | 上傳下載一律經 API 串流轉送，不使用 presigned URL | 公司規範禁止 presigned URL；權限統一由 API 檢查 |
| 登入 | 第一階段用帳號密碼，預留 OIDC 介面，之後導入 SSO | 先求上線，SSO 之後再接 |
| 權限 | 只做到 Project 層級：Platform Admin、Owner、Editor、Viewer（ADR-0001） | Derived Page 會綜合多份 Source，頁面層級擋不住外洩 |
| Agent | 沿用 nashsu 的 Rust agent，移除 shell、CLI transport 和本機專屬工具 | 共用伺服器上的 shell 等於讓使用者執行任意指令 |
| Python 擴充 | 三個固定的 Extension Point：agent 工具（MCP）、解析器（HTTP）、事件處理（佇列）；Plugin 由 Platform Admin 登記、在個別 Project 啟用，寫回時用 Service Account，呼叫 LLM 一律經過平台 | 用標準協定，外掛掛掉不影響核心；成本和模型限制只有一個入口 |
| 服務溝通 | 查詢走 HTTP/MCP，ingest 和事件從階段 1 起走 NATS JetStream，用 outbox 模式確保資料庫與訊息一致（ADR-0003）；只有 Rust 是 NATS 客戶端，直到階段 4 開放事件處理 | 提早把 JetStream 維運跑穩，日後不必遷移 |
| LLM 供應商 | Claude、DeepSeek、自架模型（OpenAI 相容 API），可依 Project 限制 chat、embedding、vision 可用的模型 | 敏感資料可限定只用自架模型 |
| LLM 呼叫 | 全部移到伺服器（取代 `/proxy`），集中統計成本、做速率限制與重試，不設配額；API key 由組織統一一把，只存在伺服器 | 瀏覽器不能拿到 key；成本仍依使用者和 Project 統計 |
| 品質驗證 | 不建人工標註的基準；改寫 ingest 和搜尋時，用新舊版本對照自動比較 | 不用另外標註，也能驗收改寫 |
| 觀測 | OpenTelemetry，Rust 和 Python 接同一個 collector | 一份文件從上傳到提交是一條完整 trace |
| Prompt | 抽成獨立模板檔 | 集中管理、方便調整與版本控制 |
| 不採用 | iii、deepagents、Web Clipper、MongoDB | 目前的需求用不到；MongoDB 見 ADR-0002 |

## 目標架構

資料、權限和核心流程全部在 Rust 核心裡，Python Plugin 只能從三個固定的 Extension Point 接入。

```text
React 前端（瀏覽器）
  │ HTTP / SSE（不直接呼叫 LLM）
  ▼
Rust 核心（同一映像，依角色部署）
  ├─ API 角色：登入、Project 權限、retrieval、agent（MCP client）、LLM 服務、遠端 MCP server
  │     │ 經 NATS JetStream 派工（outbox）
  │     ▼
  └─ Ingest worker 角色：解析 → 分析 → 產生 Changeset → 提交（每個 Project 單一提交）→ 後續處理
        （階段 1–2：Rust 取得工作後交給 Node 子程序跑 TS ingest）

Rust crate（由現有的 core / server 拆出）
  storage（頁面儲存介面，後端是 PG + MinIO）· retrieval · agent · parsing · llm · api

資料層
  ├─ Postgres：頁面、版本、wikilink、對話、Review Item、ingest 狀態、outbox、pgvector
  ├─ NATS JetStream：ingest 工作佇列、頁面提交事件
  ├─ MinIO：Source、圖片（只經 API 串流存取）
  └─ tantivy 索引：每個 Project 一份（階段 5）

Python Extension Point（同事負責）
  ├─ Agent 工具：MCP server，由 Rust agent 呼叫
  ├─ 解析器：HTTP 介面，由 ingest worker 呼叫（先接 MinerU）
  └─ 事件處理：用 nats-py 訂閱 JetStream 的頁面提交事件
```

前端只和 API 角色溝通；API 透過 NATS JetStream 把 ingest 工作派給 worker。三個 Python Extension Point 分別由 agent（MCP）、ingest worker（HTTP）和 JetStream 事件觸發。

### 現有程式碼的去留

| 現有模組 | 去向 |
| --- | --- |
| `crates/server`（axum、SSE、串流上傳下載、靜態服務、worker bridge） | 改成 `api` 角色；`auth.rs` 換成多人登入，`/proxy` 換成 LLM 服務 |
| `core/commands/fs.rs` | 被 `storage` 的頁面儲存介面取代；fs RPC 依路徑分流 |
| `core/commands/search.rs`、`page_embedding.rs` | 移到 `retrieval` |
| `core/commands/vectorstore.rs`（LanceDB） | 改寫成 pgvector，embedding 依模型分表 |
| `core/agent/`（`provider.rs` 除外） | 移到 `agent`，拿掉 shell 和本機工具 |
| `core/agent/provider.rs` | 抽成 `llm`，加上統計、模型限制、重試 |
| `core/commands/extract_images.rs`、`ebook.rs`，以及 office/docx 解析 | 移到 `parsing` |
| `core/api_server.rs` | 階段 3 改成遠端 MCP 時拆掉 |
| `src-tauri` 桌面殼、`clip_server.rs`、`file_sync.rs`、`claude_cli.rs`、`codex_cli.rs`、`cli_resolver.rs` | 刪除 |
| 前端 `src/platform/` 的 desktop 分支 | 刪除 |

## 各階段工作計畫

共六個階段，每階段完成條件達成才進下一階段。原版 llm_wiki 全程當對照組，不再維護。

### 階段 0：盤點與準備

- [x] 盤點前端經 `/rpc` 呼叫 Rust 的地方：75 個 command，標出各自的去向（`plans/inventory/rpc-commands.md`）
- [x] 盤點前端發出的 LLM 請求：`streamChat` 22 處加其他外部請求（`plans/inventory/llm-requests.md`）
- [x] 盤點 retrieval 有哪些邏輯在前端、哪些在 Rust（`plans/inventory/retrieval-split.md`）
- [x] 盤點 `.llm-wiki/*.json` 的每種狀態檔，各自對應到哪張資料表（`plans/inventory/llm-wiki-state-files.md`，欄位設計留到階段 1）
- [ ] 找出所有寫在程式碼裡的 prompt，抽成 `prompts/` 模板檔：**清單已完成**（`plans/inventory/prompts.md`），模板檔尚未抽出；`ingest.ts` 的部分等階段 3 一併處理
- [x] 架好 k3d 開發叢集：CloudNativePG（含 pgvector）、NATS JetStream 三副本、MinIO、OTel Collector（`deploy/k3d/`）

**完成條件：** 有完整的盤點清單和 prompt 模板檔，開發叢集可用。

### 階段 1：單人版伺服器（重整形狀）

- [x] core 去除 Tauri 依賴（`llm-wiki-core`）
- [x] ingest 能在伺服器端執行，不需要開著瀏覽器（Node worker）
- [x] 移除 `src-tauri` 的桌面殼（Rust）：現在 `src-tauri/` 是只含 `llm-wiki-core`、`llm-wiki-server` 的 virtual workspace；CI 改成只跑 Linux，刪掉桌面版發佈流程
- [x] 移除前端 desktop 分支：刪 `platform/tauri.ts`、`isDesktop`、自動啟動、視窗主題、開啟本機資料夾，並移除全部 `@tauri-apps/*` 套件；平台層只剩 web／worker 兩種實作，測試改為 mock `@/platform`（`no-direct-tauri-imports.test.ts` 改成全專案禁止 `@tauri-apps`）。
  - 遺留：`general`、`api-server` 設定頁在 server 版本來就隱藏，但頁面、`generalConfig`／`apiConfig` store 和 `README`／`START.md` 的桌面版說明還在；`lib/tauri-fetch.ts` 名稱沿用；`vite.config.ts` 還有 Tauri 時代的註解。這幾項跟著階段 2 的設定頁整理一起清
  - 既有問題（`main` 上就失敗，與本分支無關）：`embedding.real-llm.test.ts`、`llm-client.real-llm.test.ts` 共 6 個測試會失敗（Origin 標頭預期是桌面版行為）
- [x] 刪除 CLI transport（`claude_cli`、`codex_cli`、`cli_resolver`）：後端、前端 provider／設定 UI／聊天的 CLI 專用 fallback 路徑都已移除
- [x] 刪除 `clip_server`：連同全域「目前專案」註冊表（`current_project_path`、`all_projects`）、`current` 別名、Web Clipper 瀏覽器擴充套件、前端 clip watcher 一起拿掉；`api_server` 和 agent 的專案查詢現在只認明確的 Project id／路徑。副作用：`mcp-server/` 預設用 `current`，現在必須先 `llm_wiki_set_project`；它跟著 `api_server.rs` 在階段 3 改成遠端 MCP 時一起處理
- [x] 刪除 `file_sync` 與資料夾監控：Rust `file_sync.rs`（約 1950 行）、`fs.rs` 的 `mark_app_write_path`、`CoreContext.file_sync`、7 個 dispatch 分支、`/sources/rescan` 路由和 MCP 的 `llm_wiki_rescan_sources`；前端的 `project-file-sync`、排程匯入（`scheduled-import`）、活動面板的檔案同步區塊、兩個設定頁及其 store 欄位與 i18n 鍵都已移除。「來源」頁的重新整理現在只重新載入清單
  - 保留：`SourceWatchConfig`／`source-watch-config.ts` 其實同時是匯入篩選設定（副檔名、大小上限、`persistExtractedMarkdown`、ingest 並行數），明確匯入和 ingest 還在用，暫不改名；使用者已無法在 UI 編輯，只讀預設或舊值。階段 2 整理設定頁時改名成匯入設定
- [ ] core 依領域拆 crate：storage、retrieval、agent、parsing、llm；server 改名為 api（碰到哪裡才拆哪裡，不一次大改）
- [x] 設計 Postgres schema：Project、頁面、頁面歷史、wikilink 邊、操作紀錄、對話、Review Item、ingest 快取與狀態、outbox，另含 embedding 分表與 lint／dedup／圖片說明快取。SQL 在 `src-tauri/crates/storage/migrations/`，設計說明在 `plans/schema.md`；已在 k3d 套用並跑過冒煙測試
- [x] 盤點直接碰檔案系統的呼叫點（`plans/inventory/fs-callsites.md`）：約 300 處、26 個檔案；發現 schema 漏掉 `agent-sessions` 與 `agent-workspace/`，已補；發現 agent skills 會讀伺服器主機的 `~/.claude`／`~/.codex`／`~/.agents`，須立刻刪
- [ ] storage 頁面儲存介面：fs RPC 依路徑分流，`wiki/` 存 PG、`raw/` 存 MinIO，`.llm-wiki/*.json` 逐一換成資料表；TS worker 和前端看到的路徑語意不變
  - [x] `llm-wiki-storage` crate：`ProjectFiles` trait（read／write 帶 `Precondition`／delete／rename／stat／list）、`RelPath`（拒絕 `..`、絕對路徑等）、`ProjectLocator`（虛擬根 `/projects/<uuid>` ↔ `(ProjectId, RelPath)`，既有的 `project_path: &str` 簽名可沿用）、`route()`（Pages／Blobs／State 分流）、`MemoryStore`（語意基準實作）
  - [ ] B 類呼叫點改走 `ProjectFiles`（見 `plans/inventory/fs-callsites.md`）
  - [x] `PgFiles`（頁面部分）：`pages`＋`page_versions` 的 Postgres 實作；`MemoryStore` 與 `PgFiles` 跑同一組 13 個一致性測試（`conformance.rs`），`make test-pg` 在 k3d 上實跑。rename 與 delete 也各寫一筆版本紀錄，rename 會讓版本加一
  - [x] 頁面歷史每頁只留 8 版（常駐開啟，取代 `file_history` 的選擇性開關）
  - [x] `PgBlobs`（blobs 部分）：後端是 `object_store::ObjectStore`（測試用 `InMemory`），物件不可變、每次寫入用新鍵（公司 MinIO 每鍵最多 10 版）；舊列軟刪除，`purge_deleted`／`purge_project` 事後清掉舊物件。跑同一組一致性測試，另有 7 個 blob 專屬測試
  - [ ] `RoutedFiles`：依 `route()` 合併 `PgFiles`、`PgBlobs` 與 State 轉接層（list 合併、跨後端的子樹刪除與 rename）；連到 k3d 的真 MinIO（`AmazonS3Builder`、path-style、allow_http）；`fs.rs` 接上介面
  - [ ] 頁面寫入時一併重建 `page_links`、`page_sources`，並解析 frontmatter 填 `title`／`page_type`
  - [ ] `.llm-wiki/*` 狀態檔的轉接層（`route()` 回傳 State 的路徑，讀寫轉成對應資料表）
  - [x] 大檔案串流：`BlobStreams`（`put_stream`／`get_stream`，8 MiB 分段上傳，邊傳邊算 SHA-256）。API 的上傳／下載端點還沒接
- [ ] 所有索引、快取、檢索介面都以 Project 為鍵；檢索介面接受 Project 清單（先只傳一個），為跨 Project 檢索預留
- [ ] 向量庫從 LanceDB 換成 pgvector，embedding 依模型分表
- [ ] NATS JetStream：定義 stream 與 subject（ingest 工作、頁面提交事件），outbox relay；Rust ingest-worker 角色取得工作後交給 Node 子程序，再由 Rust 回 ack
- [ ] 檔案上傳下載一律經 API 串流轉送，不使用 presigned URL
- [ ] api：新增以 Project 為範圍的路由；`/rpc` 保留，由路徑反推出 Project
- [ ] llm：LLM 呼叫全部移到伺服器，取代 `/proxy`；支援 Claude、DeepSeek 和自架模型（OpenAI 相容 API），涵蓋 chat、embedding、vision；API key 由組織統一設定
- [ ] Agent 移除 shell 工具和 AnyTXT 等本機專屬工具
- [ ] app 部署到 k3d：api 和 ingest-worker 兩個角色

**完成條件：** 在 k3d 上，一個人用瀏覽器就能完成 ingest、聊天、檢索、看圖譜；瀏覽器不再發出任何 LLM 請求，資料都在 PG 和 MinIO 裡。

### 階段 2：多人化

- [ ] 帳號密碼登入：密碼用 Argon2 雜湊、登入失敗次數限制、session 管理；預留 OIDC 介面，未來導入 SSO
- [ ] 角色：Platform Admin（管帳號、建立 Project、設定模型）、Owner、Editor、Viewer；只有 Platform Admin 能建立 Project
- [ ] 每條 API 路由都經過 Project 權限 middleware；retrieval、圖譜、agent 上下文、MCP 查詢只會碰到使用者所屬的 Project
- [ ] 建立 Project 和加入成員時，提醒「同一個 Project 內的內容所有成員都看得到」
- [ ] 集中管理 LLM 呼叫：依使用者和 Project 統計成本、速率限制、重試；不設配額、不做成本門檻通知，只統計（Platform Admin 看用量儀表板；預算警示交給 LLM 供應商後台）
- [ ] 依 Project 設定可用模型，chat、embedding、vision 一併限制（例如敏感 Project 只能用自架模型）
- [ ] Review Item 改存伺服器，同一個 Project 的成員共用，Editor 以上的角色可以處理，並記錄是誰處理的
- [ ] SSE 即時更新：ingest 進度、聊天串流、資料已更新通知
- [ ] Agent 核准流程改成暫停、SSE 通知、API 繼續
- [ ] 前端引入 TanStack Query 管理伺服器資料，同時把 `/rpc` 呼叫全部換成 Project 範圍的路由，再刪掉 `/rpc`
- [ ] XSS 防護：markdown 渲染過濾；agent 產生的 HTML 放不同網域的 sandboxed iframe
- [ ] 檔案上傳限制大小和類型；下載經 API 時檢查權限
- [ ] 頁面編輯用版本號擋衝突，提示「已被別人修改」
- [ ] Project 匯出：打包成 zip 的 markdown 資料夾（不要求相容 Obsidian）

**完成條件：** 多位使用者能同時使用不同 Project，搜尋、圖譜和 agent 回答都不會帶出使用者不屬於的 Project 的內容。

### 階段 3：分散式 ingest 與第一個 Extension Point

- [ ] 用 Rust 一次改寫 ingest，拆成六階段：登記、解析、分析、產生 Changeset、提交、後續處理
- [ ] 產生階段的 prompt 改成輸出 Changeset
- [ ] 提交階段：用 Postgres advisory lock 確保每個 Project 同時只有一個提交，加上樂觀鎖、entity 頁面合併；人工編輯過的頁面照常合併
- [ ] Changeset 預設自動提交；可以查看 diff、可以還原。如果頁面在那之後又被改過，就列為衝突，並產生一筆 Review Item
- [ ] 刪除 Source：刪掉原始檔和它的 summary 頁，受影響的 Derived Page 各產生一筆 Review Item
- [ ] index.md、log.md 改由資料庫產生；overview.md 改為延遲批次更新
- [ ] 用 outbox 把「頁面已提交」事件送進 JetStream；設定重試上限，超過的訊息轉入失敗 stream，可在介面上手動重試
- [ ] 新舊對照：同一批 3–5 份文件分別跑 TS worker 和 Rust ingest，自動比較頁面集合、頁面類型和 wikilink 數量，必要時讓 LLM 評比內容；通過後移除 Node worker
- [ ] Ingest worker 拆成獨立 Deployment，依 JetStream consumer 堆積量擴展
- [ ] 開放解析器 Extension Point，提供 FastAPI 範本，先接 MinerU
- [ ] `api_server.rs` 改成遠端 MCP，每人用自己的 token
- [ ] 全程 OpenTelemetry，trace id 放在 NATS 訊息標頭傳遞

**完成條件：** 多份文件可同時 ingest 而不衝突；新舊對照的結果相當；一位 Python 同事能獨立寫出一個解析器。

### 階段 4：其他 Extension Point

- [ ] Plugin 管理：Platform Admin 在介面上登記 Plugin，在個別 Project 啟用；發 Service Account 給需要寫回的 Plugin
- [ ] 提供給 Plugin 用的 LLM proxy API（依 Service Account 和 Project 統計，並套用模型限制）
- [ ] Agent 加上 MCP client，開放 agent 工具 Extension Point
- [ ] 開放事件處理 Extension Point：Python 用 nats-py 訂閱 JetStream 的「頁面已提交」等事件
- [ ] 資料來源連接器（取代資料夾自動監控），由 Python 同事實作
- [ ] Skills（SKILL.md）改為網頁管理

**完成條件：** 三個 Extension Point 都有 Python 同事寫出的實際 Plugin 在運作。

### 階段 5：規模與效能

- [ ] 分詞搜尋改用 tantivy：自訂 CJK 二元組分詞器，並和原版搜尋結果自動比對；每個 Project 一份索引，由提交事件更新；API 多副本時由獨立的搜尋角色持有索引（持久化磁碟），索引可隨時從 Postgres 重建
- [ ] 每個 Project 的圖譜在記憶體快取，收到提交事件才更新
- [ ] 圖譜分析（關聯分數、Louvain、Graph Insights）移到伺服器，前端分頁載入
- [ ] 依實際資料量評估 pgvector 是否足夠

**完成條件：** 依實際使用資料訂出效能目標並達成。預估規模是 50–200 位使用者、10–50 個 Project，每個 Project 少於 5k 頁，每天 ingest 少於 200 份，所以這一階段可以視需要延後。

## 決策紀錄與待決定事項

已於 2026-10-26 決定：

| 項目 | 決定 | 影響 |
| --- | --- | --- |
| 上游策略 | 長期分叉 | 不追上游；prompt 模板檔由團隊自行維護 |
| 使用範圍 | 只在公司內部使用 | 不觸發 GPL v3 的散布義務 |
| 既有程式碼 | 在 core / server 上演進，沿用程式碼、重整形狀 | 階段 1 以重整為主；`/rpc` 到階段 2 才刪 |
| 術語 | 統一用「Project」，只有單一組織 | 見 `CONTEXT.md` |
| 權限粒度 | 只做到 Project 層級（ADR-0001） | 權限差異大的資料放不同 Project；Derived Page 對所有成員可見 |
| 角色 | Platform Admin、Owner、Editor、Viewer；只有 Platform Admin 能建立 Project；Viewer 使用上不設限 | 見 `CONTEXT.md` |
| 頁面儲存 | PG，透過 fs 介面接縫存取；不用 MongoDB（ADR-0002） | `.llm-wiki/*.json` 改成資料表 |
| 匯入／匯出 | 不做匯入；匯出成 zip 的 markdown 資料夾，不要求相容 Obsidian | 現有資料夾 project 要重新 ingest |
| Ingest 改寫 | 階段 1–2 沿用 TS worker，階段 3 一次改寫成 Rust | Node worker 要多活兩個階段 |
| Changeset | 自動提交、可還原；還原時遇到衝突就產生 Review Item | 不需要人工核准 |
| 人工編輯 vs ingest | 照常合併，不特別保護 | 靠頁面歷史和 Changeset 還原 |
| 刪除 Source | 只刪原始檔和它的 summary 頁，受影響的頁面各產生 Review Item | 內容無法徹底清除（見風險） |
| Review Item | 存伺服器、同一個 Project 共用 | 記錄是誰處理的 |
| 品質驗證 | 不做人工標註的基準；改寫時用新舊對照 | 階段 0 不建測試資料集 |
| 搜尋索引 | tantivy | 每個 Project 一份索引（階段 5） |
| 佇列與事件 | NATS JetStream，從階段 1 就導入（ADR-0003） | 資料庫與訊息的一致性靠 outbox；只有 Rust 是 NATS 客戶端 |
| 部署 | 公司內部自架 k8s；開發用 k3d；物件儲存用 MinIO | 所有元件自己架 |
| Agent 執行程式碼 | 先不提供 | 移除 shell；有需求時再評估隔離沙箱或專用工具 |
| LLM 供應商 | Claude、DeepSeek、自架模型（OpenAI 相容 API） | 依 Project 限制 chat、embedding、vision 可用的模型 |
| LLM 配額 | 不設配額，只統計 | 不做成本門檻通知（已決定）；日後要加，`llm_usage` 表不用改 |
| API key | 組織統一一把 | 成本仍依使用者和 Project 統計 |
| Plugin | Platform Admin 登記、在個別 Project 啟用、寫回用 Service Account、呼叫 LLM 走平台 | 階段 4 |
| 登入 | 先用帳號密碼，之後導入 SSO | 預留 OIDC 介面 |
| 檔案存取 | 不使用 presigned URL | 上傳下載經 API 串流轉送 |
| Web Clipper | 不保留 | 移除 `clip_server` 和瀏覽器擴充套件 |

目前沒有尚待決定的事項。

## 風險與對策

| 風險 | 影響 | 對策 |
| --- | --- | --- |
| 同一 Project 內的資訊外洩 | Derived Page 會綜合多份 Source；同一 Project 的所有成員都看得到 | 以 Project 切分，權限差異大的資料放不同 Project；建立 Project 和加入成員時提醒這個限制（ADR-0001） |
| 刪除 Source 後內容殘留 | 如果刪除是因為「這份資料不該出現在這裡」，Derived Page 裡仍留有它的內容 | 產生 Review Item，由 Owner 手動處理；必要時再做「刪除後由 LLM 重新整理」 |
| 只有一人會 Rust | 核心卡在一個人身上 | 模組化單體、少拆服務；會常變動的部分（解析、連接器、agent 工具）放到 Python Extension Point |
| 重整 core 時把功能弄壞 | 階段 1 要拆 crate、換儲存、換 API，範圍很大 | 碰到哪裡才拆哪裡；`/rpc` 保留到階段 2；每一步都要 `cargo test` 和 `npm test` 通過 |
| 多一套 NATS 要維運 | NATS 故障時 ingest 和事件全部停擺 | 三節點叢集、開啟持久化；監控 consumer 堆積與重送次數；Postgres outbox 保證事件不遺失，NATS 恢復後補送。k3d 的三個節點在同一台機器上，只能驗證設定 |
| 改寫後 ingest 品質退步 | 知識庫內容變差 | 階段 3 用新舊對照，同一批文件分別跑 TS 和 Rust 版，自動比較；沒有人工基準，所以每次改 prompt 或模型都要重跑對照 |
| 長期分叉後錯過上游改進 | nashsu 的新功能和 prompt 改進不會自動進來 | 定期瀏覽上游 release notes，有價值的改進手動評估移植 |
| 平行 ingest 寫入衝突 | 頁面內容被覆蓋 | Changeset + Postgres advisory lock 確保每個 Project 單一提交 + 樂觀鎖；entity 頁面合併而非覆寫 |
| LLM 成本失控 | 多人共用一把 key 又沒有配額，費用可能暴增 | 集中管理 LLM 呼叫，依使用者和 Project 統計，由管理員看儀表板與供應商帳單警示；SHA256 增量快取；overview.md 延遲更新 |
| 敏感資料送到外部 LLM | 不符合公司規範 | 依 Project 限制可用模型，chat、embedding、vision 一併限制；Plugin 呼叫 LLM 也只能經過平台 |
| 帳號密碼被破解 | 未授權存取 | Argon2 雜湊、登入失敗限制、密碼強度要求；盡早導入 SSO |
| 檔案經 API 轉送造成負載 | 大檔案上傳下載佔用 API 資源 | 串流處理，不整個載入記憶體；限制檔案大小；必要時拆出專門的檔案服務 |
| XSS 與資料外洩 | 使用者看到不該看的內容或被攻擊 | 權限在 Rust 服務端強制檢查；markdown 過濾；HTML 預覽放不同網域的 iframe |

## 之後重新評估的觸發條件

| 選項 | 什麼時候重新考慮 |
| --- | --- |
| 頁面層級權限 | 確實有「同一主題但權限不同」的資料無法拆成不同 Project；這是架構調整，不只是加功能（ADR-0001） |
| LLM 配額 | 實際費用超出預期，或預算有硬上限；先在 Project 層級設上限，再考慮通知 |
| Changeset 人工核准 | 自動提交的內容品質問題頻繁到要事前把關；做成依 Project 選擇 |
| 刪除 Source 後由 LLM 重新整理 | 刪除 Source 時常需要徹底清除內容 |
| 匯入既有資料夾 project | 有大量既有 wiki 不適合重新 ingest |
| 人工基準資料集 | 新舊對照不足以判斷品質，或要頻繁調整 prompt 和模型 |
| 跨 Project 檢索 | 使用者需要一次查多個 Project；做法是在使用者所屬的 Project 分別檢索，再合併排序 |
| Agent 執行程式碼 | 需要對表格做資料分析、畫圖或產生檔案；優先做專用工具，必要時才開隔離沙箱 |
| 導入 SSO | 使用者數增加、帳號管理變成負擔，或公司要求統一身分管理 |
| 引入 iii | Extension Point 超過五、六種，或同事需要自己定義新的觸發條件；先把 Rust 呼叫外掛的程式碼包在自己的介面後面，以便日後替換 |
| 向量庫換成專用方案 | pgvector 的查詢延遲或資料量撐不住 |
| 拆出專門的檔案服務 | 檔案上傳下載明顯拖慢 API |
| 拆出更多服務 | 某個模組的負載特性明顯不同，或有第二位 Rust 開發者加入 |
| 即時協作編輯 | 使用者確實常同時編輯同一頁 |
