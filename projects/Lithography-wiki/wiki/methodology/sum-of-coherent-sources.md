---
type: methodology
title: Sum of Coherent Sources
created: 2026-10-03
updated: 2026-10-03
tags: [成像計算, SOCS, 核分解]
related: [hopkins-成像計算法, abbe-成像計算法, 19-optical-lithography--8-02-ch2--k73j1g]
sources: ["optical-lithography/02 - ch2.pdf"]
---
# Sum of Coherent Sources

Sum of Coherent Sources（SOCS）將部分同調成像核分解為加權同調響應，以有限項強度和近似影像。

## 計算與理由

對固定來源及瞳孔，求得核的特徵函數 \(\phi_n\) 與特徵值 \(\lambda_n\)，再計算：

\[
I(x)\approx\sum_{n=1}^{N}\lambda_n
\left|\phi_n\otimes t_m\right|^2.
\]

每個分量先與遮罩電場透射率卷積，轉成強度後才加權加總。選擇有效的分解可減少所需分量，而不只是把每個來源點各列為一項。

## 誤差與重用

有限 \(N\) 是近似來源；應檢查增加項數時的收斂。來源或瞳孔改變後，必須重新建立分解。

[[19-optical-lithography--8-02-ch2--k73j1g]] 稱所需項數可減少約一個數量級，但沒有給定誤差容忍值或運算環境，不能解讀為普遍的十倍執行速度。此方法尤其適合固定光學配置、大量遮罩的工作負載。