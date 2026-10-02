# scripts

LLM Wiki 的輔助腳本。這些腳本都不是 app 的一部分，只在需要時手動執行。

| 腳本 | 用途 | 執行方式 |
|---|---|---|
| [`split_pdf_chapters.py`](#split_pdf_chapterspy) | 把大型 PDF（例如教科書）依書籤拆成章節檔，再匯入 | `uv run`（自動安裝 `pypdf`） |
| [`debug_ollama_tokens.py`](#debug_ollama_tokenspy) | 用 Ollama 重現 ingest 生成請求，找出「too many tokens」的原因 | `python3`（純標準函式庫） |

---

## split_pdf_chapters.py

依 PDF 的書籤（目錄）把一本書拆成一章一個 PDF。

### 為什麼要拆

單一 source 超過模型的 context 預算時，ingest 會走長文件分段分析（`src/lib/ingest.ts` 的 `analyzeLongSourceInChunks`）。整本書會被壓縮成一份全域摘要，最後只產生**一份**資料摘要頁，很多章節的細節會在過程中遺失。

先拆成章節再用資料夾匯入，每章就是獨立的 source，各自有摘要頁，保留的內容多很多。資料夾路徑也會當成分類參考傳給 LLM。

### 需求

- [uv](https://docs.astral.sh/uv/)。腳本開頭用 PEP 723 宣告了 `pypdf` 依賴，`uv run` 會自動安裝。
- 不想用 uv 的話：`pip install "pypdf>=5"`，再用 `python3` 執行。

### 用法

```bash
# 1. 先預覽會怎麼切，不寫入任何檔案
uv run scripts/split_pdf_chapters.py 教科書.pdf --list

# 2. 確認沒問題再實際切，預設輸出到與 PDF 同名的資料夾（教科書/）
uv run scripts/split_pdf_chapters.py 教科書.pdf
```

預覽輸出範例：

```
01 - 封面.pdf  (p.1-2, 2 pages)
02 - 第1章 緒論.pdf  (p.3-8, 6 pages)
03 - 第2章 熱力學 定律 應用.pdf  (p.9-18, 10 pages)
04 - 附錄.pdf  (p.19-20, 2 pages)
```

### 參數

| 參數 | 預設 | 說明 |
|---|---|---|
| `--list` | | 只印出切分計畫，不寫入檔案 |
| `--out DIR` | `<PDF 檔名>/` | 輸出資料夾 |
| `--level N` | `1` | 依第幾層書籤切。`1` 是章，`2` 是小節 |
| `--min-pages N` | `1` | 少於 N 頁的段落併進前一段 |
| `--skip-front` | | 丟掉第一個書籤之前的頁面（封面、版權頁等） |
| `--every N` | | 不看書籤，每 N 頁切一份。PDF 沒有書籤時使用 |

### 行為說明

- 檔名格式是 `NN - 標題.pdf`，前面補零讓檔案依順序排列。中文會保留，`/ : * ? " < > |` 等不能放在檔名的字元會換成空白。
- 每個輸出 PDF 的 Title metadata 會設成原始書籤標題。
- 多個書籤指向同一頁時（例如章和它的第一個小節），標題會合併成 `第2章 … - 2.1 …`，同一段頁面只會輸出一次。
- 第一個書籤前面的頁面預設會輸出成 `Front Matter`，加 `--skip-front` 就不輸出。

### 匯入到 LLM Wiki

切完後進入 **资料源 → 导入 → 文件夹**，選輸出的資料夾。

### 常見問題

- **`No bookmarks found`**：掃描版 PDF 常沒有書籤，改用 `--every 30` 這類固定頁數切法。
- **切出來的頁碼不對**：有些 PDF 的書籤指向錯誤的頁面，請先用 `--list` 檢查，必要時改用 `--every`。
- **`PDF is encrypted`**：需要密碼的 PDF 要先解密才能切。
- **公式或表格很多**：拆分不會改善文字抽取品質。這類書建議在設定裡把 PDF 解析改成 MinerU。

---

## debug_ollama_tokens.py

用 Ollama 重現 app 的 Step 2「生成 wiki 頁面」請求，診斷生成時出現「too many tokens」的原因。

背景：app 走 OpenAI 相容的 `/v1/chat/completions`，這個 endpoint 沒辦法設定 `num_ctx`。Ollama 會用模型載入時的 context 視窗（通常比模型支援的上限小），所以 `prompt_tokens + max_tokens` 超過這個視窗就會報錯。注意 app 的 `maxContextSize` 單位是**字元**，不是 token。

### 用法

```bash
# 用預設模型和 prompt 大小送一次請求
python3 scripts/debug_ollama_tokens.py --model gemma4:12b

# 掃過多種 prompt 大小（2k～200k 字元），找出開始失敗的門檻
python3 scripts/debug_ollama_tokens.py --model gemma4:12b --sweep

# 改用原生 /api/chat 並指定 num_ctx，確認 num_ctx 是否就是問題所在
python3 scripts/debug_ollama_tokens.py --model gemma4:12b --native --num-ctx 131072
```

### 參數

| 參數 | 預設 | 說明 |
|---|---|---|
| `--base` | `http://localhost:11434` | Ollama 位址 |
| `--model` | `gemma4:12b` | 模型名稱 |
| `--prompt-chars` | `60000` | 填充 prompt 的大約字元數 |
| `--prompt-file` | | 改用指定檔案的內容當 user prompt |
| `--max-context-chars` | `204800` | 對應 app 的 `maxContextSize`，用來推算 app 會送出的 `max_tokens` |
| `--max-tokens` | 依上一項推算 | 直接覆寫 `max_tokens` |
| `--native` | | 改走原生 `/api/chat` |
| `--num-ctx` | | 只在 `--native` 時有效，指定要配置的 context 大小 |
| `--sweep` | | 依序測試多種 prompt 大小 |
| `--timeout` | `180` | 每次請求的逾時秒數 |
