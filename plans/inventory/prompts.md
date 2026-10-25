# 寫死在程式碼裡的 prompt 盤點

階段 0 產出。**這份只完成「找出來」，還沒有「抽成 `prompts/` 模板檔」。**抽取要改動 `ingest.ts`（147 KB）等核心檔案，建議等階段 3 把 ingest 改寫成 Rust 時一併處理，以免同一段 prompt 搬兩次。目前先抽出不會被改寫的部分即可（見文末）。

## TypeScript

| 任務 | 位置 | 形式 |
| --- | --- | --- |
| Ingest：分析 | `lib/ingest.ts:2328` `buildAnalysisPrompt` | 函式，動態拼入 purpose／index／source |
| Ingest：產生 | `lib/ingest.ts:2391` `buildGenerationPrompt` | 函式，最長的一段 |
| Ingest：審查建議 | `lib/ingest.ts:2565` `buildReviewSuggestionPrompt` | 函式 |
| Ingest：截斷檔案修復 | `lib/ingest.ts:2633` `buildTruncatedFileRepairPrompt` | 函式 |
| Ingest：長文件分段分析 | `lib/ingest.ts:3051、3090` `buildChunkAnalysisSystemPrompt`／`UserPrompt` | 函式 |
| Ingest：頁面合併 | `lib/ingest.ts:3328` `buildPageMergeSystemPrompt` | 函式 |
| Ingest：其他 system 訊息 | `lib/ingest.ts:1227、1305、3173、3301、3574、3690` | 呼叫點內嵌 |
| 去重偵測 | `lib/dedup.ts:171` `DETECTOR_SYSTEM_PROMPT` | 常數（template literal） |
| 去重合併 | `lib/dedup.ts:321` `MERGER_SYSTEM_PROMPT` | 常數 |
| 圖片說明 | `lib/vision-caption.ts:70` `CAPTION_PROMPT`、`:90` `buildCaptionPromptWithContext` | 常數＋函式 |
| Lint | `lib/lint.ts:265` | 陣列 join |
| Review 掃描 | `lib/sweep-reviews.ts:217` | 陣列 join |
| Deep Research | `lib/deep-research.ts:155、584` | 陣列 join |
| 研究主題最佳化 | `lib/optimize-research-topic.ts:27` | 陣列 join |
| 補 wikilink | `lib/enrich-wikilinks.ts:50` | 內嵌 |
| 編輯器選取改寫 | `components/editor/wiki-editor.tsx:192` | 陣列 join |
| 聊天（非 agent 模式）的最終 system 訊息 | `components/chat/chat-panel.tsx:1154` | 一句字串 |
| 連線測試 | `lib/connection-tests.ts:73、101` | 一句字串 |
| CLI transport 的 prompt 組裝 | `lib/codex-cli-transport.ts:51` | **刪除**（CLI transport 不保留） |

## Rust

| 任務 | 位置 | 形式 |
| --- | --- | --- |
| Agent 的系統上下文 | `core/src/agent/context.rs:64` `build_system_context` | 函式 |
| Agent 迴圈的 system | `core/src/agent/runtime.rs:2924` `build_agent_loop_system` | 函式，依檢索模式（smart／faithful）分支 |
| Agent 最後回答的 system | `core/src/agent/runtime.rs:2962` `build_agent_final_system` | 函式 |
| 其餘（工具規劃、續答、重試等） | `core/src/agent/runtime.rs`（5.9k 行） | 大量 `format!` 內嵌，未逐一列出 |
| Skills | `core/src/agent/skills.rs` | 讀 `SKILL.md`，本身就是外部檔案 |

## 注意

- 這份清單是用 `role: "system"`、`systemPrompt`、`*_PROMPT`、`build*Prompt` 等字樣搜出來的，**可能漏掉沒有這些字樣的 prompt**，尤其是 `runtime.rs` 裡內嵌的字串，以及 user 訊息的模板。
- Prompt 內容會依「輸出語言」切換（`output-language.ts`、`language-metadata.ts`），模板檔需要支援語言參數。
- 測試 `ingest.prompt.test.ts` 對 prompt 內容有斷言，抽出模板時要一起搬。

## 建議的抽取順序

1. **現在就可以抽（不受 ingest 改寫影響）：** 去重兩段常數、圖片說明、Lint、Review 掃描、Deep Research、研究主題最佳化、補 wikilink、編輯器改寫。
2. **階段 3 再抽：** `ingest.ts` 的所有 prompt（會被改寫成 Rust 的六階段版本）。
3. **階段 1 隨 agent 整理時抽：** `agent/runtime.rs`、`context.rs`。
