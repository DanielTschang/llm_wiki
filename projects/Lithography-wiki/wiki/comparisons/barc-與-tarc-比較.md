---
type: comparison
title: BARC 與 TARC 比較
created: 2026-10-03
updated: 2026-10-03
tags: [抗反射塗層, 光阻, 製程整合]
related: [19-optical-lithography--8-04-ch4--ccwms, 駐波與-barc, tarc, swing-curves, barc-穩健最佳化]
sources: ["optical-lithography/04 - ch4.pdf"]
---
# BARC 與 TARC 比較

BARC 與 [[tarc]] 都能降低 [[swing-curves]]，但所控制的介面與製程效果不同。

| 比較項目 | BARC | TARC |
|---|---|---|
| 位置 | 光阻下方 | 光阻上方 |
| 主要控制量 | 有效基板反射 | 有效上表面反射 |
| swing curves | 降低底部往返反射場 | 降低上表面直接反射場 |
| 光阻內駐波 | 可直接降低基板反射造成的駐波 | 不保證消除基板反射駐波 |
| reflective notching | 可抑制地形斜面反射 | 來源未建立相同作用 |
| 設計限制 | 底膜變動、吸收、塗佈與蝕刻整合 | 低折射率材料、吸收與頂部製程 |
| 高 NA | 需角度／偏振最佳化，可能採多層 | 理想折射率與厚度同樣隨角度／偏振改變 |

## 不可混同的頂部塗層

CEL 也是頂部塗層，但透過高吸收與曝光漂白，讓亮區先變透明以提高傳入光阻的影像對比；它不是以低吸收最大化透射的 TARC。來源描述 CEL 需要約 2–3 倍曝光劑量，屬 2007 年背景下的論述。

## 結論邊界

不存在可由本章支持的單一通用替代關係。選擇應依底部反射、厚度變動、圖案及整合限制決定，並以 [[barc-穩健最佳化]] 評估。

來源：[[19-optical-lithography--8-04-ch4--ccwms]] 第 4.3–4.5 節。