---
type: concept
title: RDPSF
created: 2026-10-03
updated: 2026-10-03
tags: [反應擴散, PEB, 潛像]
related: [peb-擴散與-dpsf, 聚合物去保護反應, 光阻化學隨機建模, 19-optical-lithography--8-06-ch6--1wif8]
sources: ["optical-lithography/06 - ch6.pdf"]
---
# RDPSF

RDPSF（reaction–diffusion point spread function）是 DPSF 在 PEB 時間內的平均，用來描述催化反應所感受的有效酸濃度。

## 定義與用途

在定值酸擴散係數、理想恆溫烘烤及無酸損失的條件下：

\[
\mathrm{RDPSF}
=\frac{1}{t_{\mathrm{PEB}}}
\int_0^{t_{\mathrm{PEB}}}\mathrm{DPSF}(t)\,dt,
\]

\[
h_{\mathrm{eff}}(x)=h(x,0)\otimes\mathrm{RDPSF},
\qquad
m=e^{-K_{\mathrm{amp}}t_{\mathrm{PEB}}h_{\mathrm{eff}}}.
\]

DPSF 描述特定時刻的擴散分布；RDPSF 描述烘烤期間的時間平均，因此不能以最終 DPSF 直接替代。見[[peb-擴散與-dpsf]]。

## 平均值與變異的區別

RDPSF 決定有效酸濃度的平均值。變異須額外考慮同一分子跨時間的位置相關性，不能只將瞬時濃度變異套用相同卷積；來源使用 CovPSF 處理此問題。

原始閉式解的 OCR 存在符號風險，本頁僅保留明確的積分定義。來源：[[19-optical-lithography--8-06-ch6--1wif8]]，§6.2.2、§6.4.6。