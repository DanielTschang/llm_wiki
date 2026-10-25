# 權限只做到 Project 層級，不做頁面層級

能不能存取，只看使用者在 Project 裡的角色（Owner、Editor、Viewer），不替個別頁面或 Source 設權限。資料的權限差異大時，就拆成不同的 Project。

做這個決定，是因為 LLM 產生的 Derived Page 會綜合多份 Source。只要能看到 Derived Page，就等於看到受限 Source 的內容，頁面層級權限根本擋不住這種外洩。另一方面，要做頁面層級權限，retrieval、圖譜、agent 上下文和 MCP 都得逐頁過濾，工作量會翻倍。

## Considered Options

- **頁面層級，但只套用在 Source 和它的 summary 頁**：保護範圍太小，成本卻和完整的頁面層級差不多。
- **Derived Page 的權限取所有來源權限的交集**：不會外洩，但每次 ingest 都要重算權限，而且使用者常會遇到「頁面看得到一半」的狀況，實作也最重。

## Consequences

- 建立 Project 時要提醒使用者：同一個 Project 內的所有內容，對所有成員都可見。
- 刪除 Source 時，不會自動把它的內容從 Derived Page 清除（見 PLAN.md 的風險表）。
- 如果之後真的需要頁面層級權限，那是一次架構調整，不只是加一個功能。
