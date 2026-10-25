# 前端發出的 LLM／外部請求盤點

階段 0 產出。階段 1 的目標：**瀏覽器不再發出任何 LLM 請求，也不再持有 API key。**

## 現況

- LLM 請求由前端的 `streamChat`（`lib/llm-client.ts`）組出，經 `platform.httpFetch` 送出。web 版這一步走 server 的 `/proxy?url=…`，也就是伺服器只負責轉送。
- 設定（含 API key、自訂 header）存在 `app-state.json`，由前端讀出後放進每個請求。
- Rust 這邊也有一份：`LlmConfig`（`core/src/agent/provider.rs`）從請求本文反序列化 `api_key`、`model`、`custom_endpoint` 等欄位，所以 agent 的 LLM 呼叫雖然在伺服器端發出，key 仍由前端帶來。
- server 端 ingest worker（`src/worker/main.ts`，Node）同樣呼叫 `streamChat`，經 `/proxy` 回到本機。

## `streamChat` 呼叫點（共 22 處）

| 任務 | 位置 | 說明 |
| --- | --- | --- |
| 聊天（非 agent 模式） | `components/chat/chat-panel.tsx:1184` | 組好檢索上下文後直接呼叫 |
| Ingest：分析、產生、審查建議、截斷修復 | `lib/ingest.ts:1087、1142、1223、1301` | 兩步流程；階段 3 改寫成 Rust |
| Ingest：長文件分段分析 | `lib/ingest.ts:3170、3298` | 見 `LongSourcePlan` |
| Ingest：頁面合併、其他 | `lib/ingest.ts:3571、3688` | `buildPageMergeSystemPrompt` |
| 去重偵測與合併 | `lib/dedup-runner.ts:84` | 偵測器＋合併器兩種 prompt |
| Lint | `lib/lint.ts:298` | 語意 lint |
| Review 掃描 | `lib/sweep-reviews.ts:240` | ingest 佇列清空後觸發 |
| Deep Research | `lib/deep-research.ts:118、607` | 研究與整理 |
| 研究主題最佳化 | `lib/optimize-research-topic.ts:58` | |
| 補 wikilink | `lib/enrich-wikilinks.ts:46` | |
| 圖片說明（vision） | `lib/vision-caption.ts:204` | 多模態；需受 Project 模型限制 |
| 編輯器：選取文字改寫 | `components/editor/wiki-editor.tsx:220` | |
| 頁面連結面板 | `components/editor/page-links-panel.tsx:200` | |
| 連線測試 | `lib/connection-tests.ts:70、97` | 改成 Platform Admin 的設定頁功能 |
| AnyTXT 搜尋 | `lib/anytxt-search.ts:107` | **刪除** |

## 其他由前端發出的 HTTP 請求

| 請求 | 位置 | 處理 |
| --- | --- | --- |
| Embedding | `lib/embedding.ts`（經 `embedding_fetch*` RPC） | 已在伺服器端發出，但 key 由前端帶來；改走 `llm` crate |
| 外部網頁搜尋 | `lib/web-search.ts`（`web_search` RPC） | 同上，key 放伺服器 |
| MinerU 解析 | `lib/mineru.ts`（約 14 處 `httpFetch`） | 階段 3 改成解析器 Extension Point，由 Python Plugin 負責 |
| 版本更新檢查 | `lib/update-check.ts` | **刪除** |
| 網址匯入 | `lib/url-source-import.ts:103` | 在瀏覽器直接 `fetch` 任意網址（有 CORS 與 SSRF 問題）；改成伺服器端抓取，並限制目的位址 |
| Clipper | `App.tsx:477、486`、`lib/clip-watcher.ts` | **刪除** |
| 本機 API 健康檢查 | `about-section.tsx`、`api-server-section.tsx` | **刪除** |

## 階段 1 的做法（建議）

1. `llm` crate 提供「以 Project＋任務類型呼叫」的介面（chat、embedding、vision），由伺服器依 Project 設定決定模型，並記錄使用者、Project、token 數。
2. 保留 `streamChat` 的函式簽章，但內部改成呼叫 `POST /api/projects/{id}/llm/chat`（SSE 串流），前端不再傳 key、endpoint 或 header。
3. 先改前端的 22 處和 ingest worker，兩邊共用同一個 `streamChat` 實作，所以只要改一個檔案。
4. 改完後，`/proxy` 只留給還沒搬的外部請求，最後一併移除。
