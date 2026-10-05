---
type: concept
title: Byers–Petersen model
created: 2026-10-03
updated: 2026-10-03
tags: [反應擴散, 光阻模型, PEB]
related: [marian-von-smoluchowski, 聚合物去保護反應, 晶圓烘烤熱歷程建模, 19-optical-lithography--8-06-ch6--1wif8]
sources: ["optical-lithography/06 - ch6.pdf"]
---
# Byers–Petersen model

Byers–Petersen model 是化學放大型光阻的速率模型，將酸擴散至反應位點與在該位點發生化學反應視為串聯步驟。

## 總速率與控制區域

\[
K_{\mathrm{amp}}=
\frac{K_{\mathrm{react}}K_{\mathrm{diff}}D_H}
{K_{\mathrm{react}}+K_{\mathrm{diff}}D_H}.
\]

其中 \(K_{\mathrm{react}}\) 描述化學反應，\(K_{\mathrm{diff}}D_H\) 描述經正規化的擴散捕獲速率。

- 擴散捕獲很快：\(K_{\mathrm{amp}}\approx K_{\mathrm{react}}\)，為反應控制。
- 化學反應很快：\(K_{\mathrm{amp}}\approx K_{\mathrm{diff}}D_H\)，為擴散控制。

擴散步驟與 [[marian-von-smoluchowski]] 的 trap rate 理論相關。

## 溫度依存與證據邊界

反應與擴散具有不同活化能時，控制區域可隨溫度轉換，方向取決於兩者的相對活化能。本章 Figure 6.6 的 26.5 與 45 kcal/mol 是示例，不是特定保護基的實測值。

本章支持模型方程與極限推導，未逐一驗證各配方的適用性。來源：[[19-optical-lithography--8-06-ch6--1wif8]]，§6.2.5。