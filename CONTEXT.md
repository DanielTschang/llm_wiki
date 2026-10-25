# LLM Wiki

公司內部多人使用的知識平台：使用者上傳來源文件，LLM 把它們整理成互相連結的 wiki 頁面，再供檢索、聊天與 agent 使用。

## Language

### 知識的組織

**Project**:
知識隔離與權限的最小單位；一組來源文件和由它們產生的 wiki 頁面。中文寫作「知識庫」時也指它。全平台只有單一組織，Project 之上沒有其他層級。
_Avoid_: 知識庫（作為不同概念時）、workspace、wiki（指整個集合時）

**Source**:
使用者放進 Project 的原始文件（PDF、圖片、文字等），是 LLM 衍生內容的依據。
_Avoid_: raw file、上傳檔

**Derived Page**:
LLM 從一份或多份 Source 綜合產生的 wiki 頁面（entity、concept、overview 等）。能看到 Project 的人都看得到它。
_Avoid_: 生成頁、AI 頁

### 人與權限

**Platform Admin**:
管理帳號、建立 Project、設定可用模型與配額的人。只有 Platform Admin 能建立 Project。
_Avoid_: superuser、root

**Project Member**:
被加入某個 Project 的使用者，帶有 Owner、Editor、Viewer 其中一種角色。能不能存取，只看 Project 層級的角色，不看個別頁面。
_Avoid_: 協作者、共享對象

**Owner**:
管理 Project 成員和設定、可以刪除頁面的 Project Member。

**Editor**:
可以上傳 Source、觸發 ingest、編輯頁面、處理 Review Item、還原 Changeset 的 Project Member。

**Viewer**:
可以瀏覽、搜尋、聊天和使用 agent 的 Project Member，不能改動內容。

**Service Account**:
給 Plugin 用的非人類帳號，由 Platform Admin 發給，和人一樣在各 Project 有自己的角色。
_Avoid_: bot、系統帳號

### 擴充

**Extension Point**:
平台開放給 Python 接入的三個固定位置之一：agent 工具、解析器、事件處理。
_Avoid_: hook、外掛介面

**Plugin**:
接在某個 Extension Point 上的 Python 服務。由 Platform Admin 登記，在個別 Project 啟用；呼叫 LLM 一律經過平台。
_Avoid_: 擴充套件、extension

### Ingest

**Changeset**:
一次 ingest 產生、尚未寫入的頁面變更集合。預設自動提交；提交後可查看 diff 與還原，不需要事前人工核准。
_Avoid_: patch、diff（指整體時）、draft

**Review Item**:
LLM 在 ingest 過程中提出、需要人判斷的待處理事項（矛盾、重複、缺頁、確認、建議）。屬於 Project，所有 Editor 共用，並記錄是誰處理的。它和 Changeset 是不同的東西：Review Item 不阻擋提交。
_Avoid_: review（單獨使用時，容易和 Changeset 審核混淆）
