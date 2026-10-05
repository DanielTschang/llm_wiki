---
type: concept
title: Lumped Parameter Model
created: 2026-10-03
updated: 2026-10-03
tags: [resist-model, development, cd]
related: [19-optical-lithography--8-07-ch7--dxj4cs, 顯影路徑與最短時間原理, variable-threshold-resist-model]
sources: ["optical-lithography/07 - ch7.pdf"]
---
# Lumped Parameter Model

Lumped Parameter Model（LPM）以固定理論對比度、有效膜厚及分段顯影路徑，近似空中像到 CD–dose 響應的轉換。

模型假定路徑先垂直到指定深度，再水平到輪廓邊緣，而不完整求解 [[顯影路徑與最短時間原理]]。強度的橫向與深度依賴亦假定可分離。

## 有效膜厚與響應

若只有吸收造成深度強度變化：

\[
D_{\mathrm{eff}}=\frac{1-e^{-\alpha\gamma D}}{\alpha\gamma}.
\]

無吸收時其極限為實際厚度 \(D\)。LPM 響應為：

\[
\left[\frac{E(x)}{E(0)}\right]^\gamma
=1+\frac{1}{D_{\mathrm{eff}}}
\int_{x_0}^{x}
\left[\frac{I(x')}{I(x_0)}\right]^{-\gamma}dx'.
\]

其中 \(CD=2x\)；\(E(0)\) 是產生零 CD 的劑量，一般不等於 open-frame dose-to-clear。

## 限制

固定對比度只適合相關劑量落在 log-rate／log-dose 曲線近似線性的區域。分段路徑也不是一般速率場的精確最短時間路徑。

來源圖 7.20 的 Gaussian 範例中，Dawson’s integral 近似 LPM 相對完整 LPM 的劑量差約 1%。這是模型內比較，不是普遍實驗準確度。

進一步忽略垂直顯影時間得到 [[variable-threshold-resist-model]]。來源：[[19-optical-lithography--8-07-ch7--dxj4cs]] §7.3.6–7.3.7。