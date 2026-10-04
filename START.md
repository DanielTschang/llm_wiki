# 啟動 LLM Wiki

LLM Wiki 有三種執行方式，背後用的是同一套前端和 Rust 後端（`src-tauri/crates/core`）：

| 版本 | 適合情境 | 指令 |
|---|---|---|
| **桌面版** | 在自己的電腦上使用，原本的用法 | `make desktop` |
| **網頁版（自架伺服器）** | 用瀏覽器操作，可放在 NAS 或伺服器上、從其他裝置連線 | `make server` |
| **網頁版（Docker）** | 同上，但用 Docker 部署 | `make docker-up` |

執行 `make help` 可以列出所有指令。架構與設計細節請見 `plans/web-server-mode.md`。

> ⚠️ **同一個專案一次只用一個版本開啟。** 桌面版和網頁版各自有自己的 ingest queue 和檔案監看。兩邊同時處理同一個專案時，`.llm-wiki/` 底下的狀態可能互相覆蓋，或造成重複 ingest。

---

## 事前準備

- **Node.js 22 以上**、**Rust**（`rustup`），以及 **protoc**（`brew install protobuf`，lancedb 編譯時需要）
- 第一次使用時安裝相依套件：

```bash
make install
```

---

## 1. 桌面版

```bash
make desktop          # 開發模式，改程式會自動重新載入
make desktop-build    # 產生安裝檔，位於 src-tauri/target/release/bundle/
```

- 設定（模型、API key、最近的專案）存在 `~/Library/Application Support/com.llmwiki.app/app-state.json`。
- 開發模式的前端使用 port 1420，同一時間只能跑一個 `tauri dev`。

---

## 2. 網頁版：自架伺服器

```bash
make server
```

這個指令會依序 build 網頁前端（`dist-web/`）和 ingest worker（`dist-worker/`），再啟動 `llm-wiki-server`。看到以下訊息就表示啟動成功：

```
LLM Wiki server listening on http://127.0.0.1:19830
  ...
  ingest:    worker dist-worker/ingest-worker.mjs
[worker] watching ingest queue of ... (0 pending)
[worker] ready
```

接著用瀏覽器打開 <http://127.0.0.1:19830>，輸入 token 登入。

### Token

第一次啟動時，server 會產生一組隨機 token，印在終端機上，並存到 `~/.llm-wiki-server/server-token`。之後要查詢可以執行：

```bash
make token
```

想自己指定 token 的話：`make server TOKEN=你的密碼`

### 專案放哪裡

為了安全，server 只能讀寫**允許的資料夾**內的檔案：

- `~/.llm-wiki-server/projects/`：預設位置，在網頁上新建的專案放這裡
- `./projects/`：repo 底下若有這個資料夾，Makefile 會自動把它加進允許的範圍
- 其他資料夾：用 `ALLOW_ROOT` 指定，例如 `make server ALLOW_ROOT=~/Documents/wikis`

開啟專案或匯入檔案時，會出現**伺服器檔案瀏覽器**，可以在允許的資料夾之間瀏覽、建立新資料夾，或直接在上方輸入伺服器上的完整路徑。

匯入來源檔案或專案 ZIP 時，還可以切換到「**從此裝置**」分頁，從你正在用的電腦或手機上傳檔案（支援拖曳，也可以上傳整個資料夾）。上傳的檔案會先暫存在伺服器上，匯入時複製進專案，暫存檔一天後自動清除。

匯出專案（設定 → 維護 → 匯出專案）會直接下載 ZIP 到你的裝置。

### 沿用桌面版的設定

網頁版的設定存在另一個檔案（`~/.llm-wiki-server/app-state.json`）。如果這台電腦上也有桌面版，server **第一次啟動、還沒有任何模型設定時**，會自動從桌面版帶入模型相關的設定（模型、provider 的 endpoint 與 API key、任務路由、embedding、多模態、MinerU、proxy、輸出語言），啟動訊息會出現：

```
  settings:  imported model settings from the desktop app (...)
```

- 只會在網頁版**還沒設定模型**時匯入一次，之後兩邊的設定各自獨立，不會互相同步，也不會覆蓋你在網頁版改過的設定。
- 專案登記、最近的專案等網頁版自己的資料不會被動到。
- 不想自動匯入的話，加上 `--no-desktop-settings`（或環境變數 `LLM_WIKI_NO_DESKTOP_SETTINGS=1`）。
- 設定檔含有 API key，server 會把它設成只有你能讀寫（`0600`）。

### 背景 ingest

ingest 由伺服器上的 worker 執行，**關掉瀏覽器分頁也會繼續跑**。worker 會自動接手最後開啟的專案的 queue：

- `watching ingest queue of … (0 pending)`：queue 是空的，worker 只是在待命，不會呼叫 LLM
- `resuming N interrupted task(s) in …`：正在接續上次沒跑完的任務

目前的限制：檔案監看（`raw/sources/` 自動匯入）仍然由瀏覽器分頁負責。沒開網頁時新增的檔案，要等下次打開網頁時才會被加入 queue。

### 常用參數

| 參數 | 預設值 | 說明 |
|---|---|---|
| `PORT` | `19830` | 監聽的 port |
| `BIND` | `127.0.0.1` | 只接受本機連線。要讓其他裝置連線請設成 `0.0.0.0`（見下方安全提醒） |
| `DATA_DIR` | `~/.llm-wiki-server` | 設定、token、預設專案資料夾的位置 |
| `TOKEN` | 自動產生 | 登入用的 token |
| `ALLOW_ROOT` | `./projects`（若存在） | 額外允許存取的專案資料夾 |

範例：`make server PORT=8080 ALLOW_ROOT=~/wikis`

其他進階選項（例如 `--allow-shell`、`--secure-cookie`）請用 `cargo run --manifest-path src-tauri/Cargo.toml -p llm-wiki-server -- --help` 查看。

### 從其他裝置連線（安全提醒）

`BIND=0.0.0.0` 會讓區網內的裝置都能連到 server，但連線是 **HTTP 明文**，token 和內容在網路上都看得到。建議：

- 只在信任的家用網路使用，或改用 Tailscale 之類的 VPN
- 要從外網連線時，在前面架 HTTPS 反向代理（例如 Caddy），並加上 `--secure-cookie`

### 開發網頁前端

改前端程式時，可以開兩個終端機，取得熱更新：

```bash
make server-only   # 終端機 1：只啟動 server（使用已 build 好的 worker）
make web-dev       # 終端機 2：Vite dev server，port 1430
```

然後打開 <http://localhost:1430>。API 請求會自動轉發到 port 19830 的 server。

---

## 3. 網頁版：Docker

```bash
make docker-up      # build image 並在背景啟動
make docker-token   # 查詢登入 token
make docker-logs    # 查看 log
make docker-down    # 停止（資料保留在 volume 中）
```

打開 <http://127.0.0.1:19830> 登入。

- 設定、token 和在網頁上新建的專案，都存在 Docker volume `llm-wiki-data` 裡（container 內的 `/data`）。
- 想使用主機上既有的專案，請編輯 `docker-compose.yml`，取消註解 `./projects:/projects` 那一行。之後在網頁上以 `/projects/<專案名稱>` 開啟。該資料夾必須能被 uid 1000 寫入。
- port 預設只對本機開放（`127.0.0.1:19830`）。要對外開放時，請參考上面的安全提醒。

> 如果 `docker compose build` 因為 buildx 出錯，可以改用 `make docker-build`，它直接呼叫 `docker buildx build`。

---

## 測試

```bash
make check   # 前端型別檢查
make test    # 前端（mock）測試 + Rust 測試
```
