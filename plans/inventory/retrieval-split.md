# Retrieval 的前端／Rust 分工盤點

階段 0 產出。結論：**檢索的核心演算法已經在 Rust**，前端只剩薄薄一層包裝，加上「嵌入（embedding）流程」和「圖譜」兩塊還在 TS。

## 已在 Rust（`core/src/commands/search.rs`，2.4k 行）

| 項目 | 位置 | 備註 |
| --- | --- | --- |
| 查詢分詞 | `tokenize_query` | 與 TS 的 `tokenizeQuery` 重複實作（見下） |
| 分詞比對評分 | `score_file`、`token_match_score` | 掃描 project 資料夾逐檔評分；階段 5 換成 tantivy |
| 向量搜尋 | `search_by_embedding` | 呼叫 `vectorstore.rs`（LanceDB），階段 1 換成 pgvector |
| 融合排序 | `apply_rrf_scores`（RRF） | 保留 |
| 圖譜補充結果 | `blend_graph_results`、`graph_result_quota`、`extract_related_frontmatter` | 依 frontmatter 的 related 與 wikilink 補入結果 |
| 檢索模式判斷 | `search_mode` | |
| wikilink 解析 | `get_page_links`、`resolve_reader_wikilink` | |
| 入口 | `search_project` → `search_project_inner` | 前端、agent、本機 API 共用 |
| Agent 的檢索 | `agent/tools.rs`（`graph.search` 等）、`agent/runtime.rs` | 有 smart／faithful 兩種檢索模式 |

## 還在前端 TS

| 項目 | 位置 | 說明 | 去向 |
| --- | --- | --- | --- |
| `searchWiki` | `lib/search.ts` | 只是 `invoke("search_project")` 的包裝 | 保留為薄 client |
| `tokenizeQuery` | `lib/search.ts:38` | 與 Rust `tokenize_query` 重複；`search-view.tsx` 用它做結果高亮 | 改成由搜尋 API 回傳命中的 token，刪掉 TS 這份，避免兩份分詞邏輯分歧 |
| 嵌入流程 | `lib/embedding.ts`（870 行）：`embedPage`、`embedAllPages`、`searchByEmbedding`、切塊、重試、重新索引狀態 | 切塊與批次在前端，向量寫入經 `vector_*_chunks` RPC | 移到 `retrieval`／ingest 後續處理，由「頁面已提交」事件觸發 |
| 檢索圖譜 | `lib/graph-relevance.ts`（`buildRetrievalGraph`、`calculateRelevance`、`getRelatedNodes`） | 計算節點關聯分數；目前只被 `wiki-graph.ts` 用來決定圖譜邊的權重 | 階段 5 移到伺服器並在記憶體快取 |
| 圖譜組裝與分析 | `lib/wiki-graph.ts`（349 行）、`wiki-graph-analysis.ts`（含 worker，Louvain 等）、`graph-insights.ts` | 在瀏覽器計算整張圖 | 階段 5 移到伺服器，前端分頁載入 |
| 聊天的最終訊息組裝 | `components/chat/chat-panel.tsx:1125–1160` | 檢索上下文由後端（`agent_start_turn` 的回傳）提供，前端只用它組 system／user 訊息再呼叫 `streamChat` | 移到伺服器；前端只送問題，收串流結果 |
| 編輯器、搜尋頁的呼叫 | `wiki-editor.tsx`、`search-view.tsx` | 呼叫 `searchWiki` | 不變 |
| Ingest 內的嵌入 | `lib/ingest.ts`（兩處 `embedPage`）、`lib/deep-research.ts`（一處） | 寫入頁面後同步嵌入 | 改成事件觸發 |

## 對階段 1 的影響

1. `retrieval` crate 的主體可以直接從 `search.rs`、`page_embedding.rs`、`vectorstore.rs` 搬出，這是現有程式碼裡最乾淨的一塊。
2. 要改的是**資料來源**：`score_file` 現在掃資料夾，要改成讀 `storage` 的頁面介面。
3. 嵌入流程是最大的缺口：切塊與批次在 TS，向量庫在 Rust，兩邊用 RPC 溝通；要整段搬到 Rust，不然向量寫入無法跟頁面提交在同一個流程裡。
4. 分詞邏輯有兩份（TS／Rust），階段 5 換 tantivy 時要確認只剩一份，並用新舊搜尋結果自動比對。
