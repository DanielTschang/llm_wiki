---
type: finding
title: 化學放大型光阻存在梯度最佳 PEB
source: "[[19-optical-lithography--8-09-ch9--1xlwz30]]"
confidence: medium
replicated: null
created: 2026-10-03
updated: 2026-10-03
tags: [PEB, reaction-diffusion, optimization]
related: [19-optical-lithography--8-09-ch9--1xlwz30, 潛像梯度與-chemical-contrast, rdpsf, byers-petersen-model]
sources: ["optical-lithography/09 - ch9.pdf"]
---
# 化學放大型光阻存在梯度最佳 PEB

來源的簡化 reaction–diffusion 模型預測：在擴散不可忽略時，最大化 PEB 後潛像梯度的烘烤程度為有限值，而非持續增加放大程度。

## 直接證據

式（9.43）與圖 9.15–9.17 顯示，短 PEB 受放大不足限制，長 PEB 受酸擴散造成的梯度平滑限制。競爭由：

\[
\eta=\frac{\pi^2D}{L^2K_{\mathrm{amp}}}
\]

表徵。模型在 \(\eta\approx0.1\text{–}0.2\) 時給出最佳放大因子約 3、邊緣剩餘 blocked polymer 濃度約 0.45、最佳 LIG/ILS 約 0.25。

這些數字不是獨立驗證的商用光阻規格。

## 條件與推論

推導假設無酸損失，並以主導 Fourier 分量及邊緣有效酸濃度近似描述梯度。

由 \(\eta\propto L^{-2}\) 可推論，縮小特徵時須降低 \(D/K_{\mathrm{amp}}\) 才能維持相同梯度損失。溫度效果取決於反應與擴散活化能差；[[byers-petersen-model]] 的擴散控制區域可能降低此比值對溫度的敏感度。

最大潛像梯度不等於完整製程最佳，也不保證最小 LER。來源未提供独立實驗重現。