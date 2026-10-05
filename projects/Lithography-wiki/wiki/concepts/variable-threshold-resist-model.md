---
type: concept
title: variable-threshold resist model
created: 2026-10-03
updated: 2026-10-03
tags: [resist-model, threshold, cd]
related: [19-optical-lithography--8-07-ch7--dxj4cs, lumped-parameter-model, nils-與影像邊緣品質]
sources: ["optical-lithography/07 - ch7.pdf"]
---
# variable-threshold resist model

variable-threshold resist model（VTR）是由 [[lumped-parameter-model]] 再近似而得的閾值光阻模型，閾值隨 image log-slope 改變，而非固定常數。

在來源的 Gaussian 近似下，忽略垂直顯影時間可得：

\[
E(x)I(x)=E_{\mathrm{th}}(x),
\qquad
E_{\mathrm{th}}(x)
=E(0)I_0
\left[\frac{1}{D_{\mathrm{eff}}\gamma\,ILS(x)}\right]^{1/\gamma}.
\]

此處 \(ILS(x)\) 使用正的斜率大小。對 Gaussian 強度，有號導數與斜率大小須明確區分。

## 適用邊界

VTR 保留材料對比度與有效厚度的物理連結，但承接固定對比度、影像近似及分段路徑假設。在欠曝光或接近可印出極限時，垂直時間未必可忽略，誤差可能增加。

來源圖 7.20 的特定 Gaussian 範例中，相對完整 LPM 的劑量差在 nominal CD 約 2%；CD 偏小 10%、20% 時約 3.5%、6.5%。這些數字不能視為所有圖案的實驗準確度。

來源：[[19-optical-lithography--8-07-ch7--dxj4cs]] §7.3.7。