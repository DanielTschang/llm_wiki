---
type: methodology
title: composite CD 空間分解
created: 2026-10-03
updated: 2026-10-03
tags: [微影, 統計, CD]
related: [mask-error-enhancement-factor, stepper-與-step-and-scan-成像比較, 19-optical-lithography--8-08-ch8--1az2fh0]
sources: ["optical-lithography/08 - ch8.pdf"]
---
# composite CD 空間分解

composite CD 空間分解以相同位置的跨晶圓平均抑制隨機變化，再逐層區分晶圓、場內、slit 及 scan 特徵，以支援 CD 變異的根因分析。

## 方法與理由

1. 在短時間內以相同設備、材料及模組處理多片晶圓，減少跨晶圓系統性差異。
2. 在完全相同的座標量測 CD。
3. 按座標平均並扣除目標 CD，形成 composite wafer。
4. 在各場的相同相對座標平均，建立 composite field。
5. 分析 slit、scan 特徵，並以獨立 reticle CD map 輔助分離 mask 來源。

若零均值誤差相互獨立且穩定，平均的不確定度約為 \(s_R(x,y)/\sqrt N\)。來源以常態誤差作示範，但真正關鍵仍是取樣與穩定性條件。

## 可識別性限制

composite field 假定全晶圓誤差在單一場內變化緩慢。若 reticle 本身具有 slit 或 scan 方向特徵，單靠輸出 CD 無法可靠區分來源。

reticle CD map 配合 [[mask-error-enhancement-factor]]可預測 mask 引起的晶圓誤差，再從輸出資料扣除，改善分解。殘差仍須檢查空間相關性與未建模特徵，不能直接宣稱純隨機。

來源：[[19-optical-lithography--8-08-ch8--1az2fh0]]，§8.3。