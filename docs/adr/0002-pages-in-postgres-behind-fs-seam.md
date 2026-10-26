# Wiki 頁面存 Postgres，透過現有的 fs 介面接縫存取

Wiki 頁面的 markdown 內容、frontmatter 和版本號都存在 Postgres。原始 Source 和圖片放在 MinIO。原本 `.llm-wiki/*.json` 這些狀態檔，改成 Postgres 裡的正式資料表。

core 新增一層頁面儲存介面。原本的 fs RPC 依路徑前綴分流：`wiki/` 存 Postgres，`raw/` 存 MinIO。分流後的路徑語意不變，所以 TS ingest worker 和前端在過渡期幾乎不用改。

## Considered Options

- **MongoDB**：頁面寫入和 outbox 事件必須在同一個交易裡，而提交鎖要用到 Postgres advisory lock 和樂觀鎖。另外，pgvector、wikilink 邊、頁面歷史、權限都已經放在 Postgres，再分出第二個資料庫，就得另外處理一致性，也會多一套系統要維運。頁面只是 markdown 字串加 jsonb，用不到文件資料庫的優勢。
- **繼續放檔案系統（PVC）**：多人使用、API 多副本、單一提交者都沒辦法在檔案系統上做好，遲早還是要搬。

## Consequences

- 匯出 Project 時，要從資料庫重新組出資料夾，打包成 zip。不要求相容 Obsidian。
- 現有的資料夾 project 不做匯入工具，要重新 ingest。
- 公司的 MinIO 每個物件鍵最多只能有 10 個版本，超過就要刪除再上傳或改名。因此 MinIO 裡的物件一律**不可變**：每次寫入都用新的物件鍵（`<project_id>/<blob id>`），覆寫同一路徑是「新增一筆 blob、軟刪除舊的」，絕不對同一個鍵重複 PUT。
- 頁面的歷史放在 Postgres，每頁只留最新 8 版（含目前這一版），不受 MinIO 限制，但刻意訂得比 10 小。
