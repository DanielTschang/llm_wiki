---
type: finding
title: 無酸損失 RDPSF 保留較多 Fourier 振幅
source: "[[19-optical-lithography--8-09-ch9--1xlwz30]]"
confidence: high
replicated: null
created: 2026-10-03
updated: 2026-10-03
tags: [RDPSF, DPSF, analytical-model]
related: [19-optical-lithography--8-09-ch9--1xlwz30, rdpsf, peb-擴散與-dpsf]
sources: ["optical-lithography/09 - ch9.pdf"]
---
# 無酸損失 RDPSF 保留較多 Fourier 振幅

在無酸損失、常數擴散係數與週期圖案模型中，reaction–diffusion 的時間平均核比 PEB 結束時的純擴散核保留更多非零 Fourier 分量。

## 直接證據

來源式（9.21）、（9.33）及圖 9.14 給出，令 \(q=2(\pi n\sigma_D/p)^2\)：

\[
A_{\mathrm{DPSF}}=e^{-q},\qquad
A_{\mathrm{RDPSF}}=\frac{1-e^{-q}}{q}.
\]

對 \(q>0\)，後者大於前者。小衰減近似為：

\[
A_{\mathrm{DPSF}}\approx1-q,\qquad
A_{\mathrm{RDPSF}}\approx1-\frac q2.
\]

因此相同小衰減下，RDPSF 可容許約 \(\sqrt2\) 倍擴散長度。來源另指出，在特定分量允許下降 20% 時，可容許約 50% 更多擴散長度。

## 適用範圍

此為解析模型結果，不是產品間實測比較，也不是含一般酸損失或 quencher 動力學系統的保證。高信心針對模型內的數學關係；來源未提供獨立實驗重現，故重現狀態未知。

概念背景見 [[rdpsf]] 與 [[peb-擴散與-dpsf]]。