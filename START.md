# 啟動 LLM Wiki

LLM Wiki 是多人使用的網頁平台：頁面存在 Postgres、檔案存在 S3（MinIO）、向量存在 pgvector。本機開發用 k3d 叢集提供這些服務。架構與進度請見 `plans/PLAN.md`，用語請見 `CONTEXT.md`。

執行 `make help` 可以列出所有指令。

## 事前準備

- Docker、`k3d`、`kubectl`
- **Node.js 22 以上**與 **Rust**（`rustup`）
- 第一次使用時安裝相依套件：

```bash
make install
```

## 1. 建立開發叢集

```bash
make k3d-up
```

會建立 k3d 叢集 `llm-wiki`（kubectl context 為 `k3d-llm-wiki`），裡面有 Postgres 17（含 pgvector）、NATS、MinIO、OTel Collector。`make k3d-status` 看狀態，`make k3d-down` 連資料一起刪除。

> 腳本一律指定 `--context k3d-llm-wiki`，不會切換你目前的 kubectl context。

## 2. 啟動伺服器

```bash
make web worker                       # 建前端（dist-web/）和 ingest worker（dist-worker/）
(cd src-tauri && cargo build -p llm-wiki-server)
make platform-dev
```

`make platform-dev` 會：

- 在 `llm_wiki_dev` 資料庫與 `llm-wiki-test` bucket 上跑伺服器（資料會保留），網址 `http://localhost:19830`
- 自動轉發 Postgres 與 MinIO 的連線，轉發中斷會自動重連
- 第一次啟動時建立 Platform Admin：帳號 `admin`、密碼 `admin-password-dev`

用瀏覽器開網址，以 `admin` 登入。

## 3. 設定模型

模型與 API key 由伺服器統一管理，瀏覽器看不到 key。

1. 啟動伺服器前，把 key 放進環境變數，名稱自己取，例如 `export LLM_WIKI_KEY_ANTHROPIC=sk-ant-...`
2. 以 Platform Admin 登入，到「設定 → LLM 模型」新增模型，並指定它用哪個環境變數
3. 語意搜尋用模型清單裡**第一個** embedding 模型；換模型後要在設定頁「重建索引」

## 4. 帳號與專案

只有 Platform Admin 能建立專案與帳號。專案成員有三種角色：Owner、Editor、Viewer。Platform Admin 不會自動看到專案內容，要把自己加成成員才行。

## 把資料夾專案搬進來

舊的資料夾版專案可以一次搬進平台（需要先連上資料庫與 MinIO，環境變數同伺服器）：

```bash
src-tauri/target/debug/llm-wiki-import --dir path/to/project --name "專案名稱"
```

重跑只會補缺的檔案。向量索引不會搬，匯入後在設定頁重建索引。

## 測試

| 指令 | 內容 |
|---|---|
| `make test-pg` | Rust 測試，含需要 Postgres／MinIO 的部分 |
| `make e2e-accounts` | 帳號、權限、LLM 閘道、搜尋、ingest 的端對端檢查（先 `cargo build -p llm-wiki-server`） |
| `make e2e-k3d` | 以 HTTP 驅動已建好的伺服器 |
| `make test` | 前端（mock）與 Rust 測試 |

## 查看資料庫與 MinIO

`deploy/k3d/ui.sh` 會轉發 Postgres（`localhost:15432`）與 MinIO 主控台（`http://localhost:9001`）給 GUI 工具使用。

## 部署

`make docker-build` 會建出 `llm-wiki-server` 映像。伺服器啟動時需要 `LLM_WIKI_DATABASE_URL` 與 `LLM_WIKI_S3_ENDPOINT`／`_BUCKET`／`_ACCESS_KEY`／`_SECRET_KEY`，完整選項見 `llm-wiki-server --help`。正式環境請放在會終止 TLS 的反向代理後面，並設定 `LLM_WIKI_SECURE_COOKIE=true`。
