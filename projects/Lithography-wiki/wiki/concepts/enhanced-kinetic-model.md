---
type: concept
title: enhanced kinetic model
created: 2026-10-03
updated: 2026-10-03
tags: [development, kinetics, resist-model]
related: [19-optical-lithography--8-07-ch7--dxj4cs, original-mack-model, notch-model, dnq, novolac]
sources: ["optical-lithography/07 - ch7.pdf"]
---
# enhanced kinetic model

enhanced kinetic model 同時描述抑制劑造成的溶解抑制，以及移除抑制劑後產物造成的溶解增強：

\[
r=r_{\mathrm{resin}}
\frac{1+k_{\mathrm{enh}}(1-m)^n}{1+k_{\mathrm{inh}}m^l}.
\]

\(r_{\mathrm{resin}}\) 是純樹脂速率；\(n\)、\(l\) 分別為增強與抑制反應階數。端點為：

\[
r_{\min}=\frac{r_{\mathrm{resin}}}{1+k_{\mathrm{inh}}},
\qquad
r_{\max}=r_{\mathrm{resin}}(1+k_{\mathrm{enh}}).
\]

## 適用範圍

五個參數 \(r_{\max},r_{\min},r_{\mathrm{resin}},n,l\) 提供較大的擬合彈性，可呈現樹脂速率附近的平台及雙閾值行為。原式假定顯影劑質量輸送不重要；不能把所有曲線形狀都解釋成已驗證的雙機制。

這裡 \(r_{\max}\) 是真正最大速率，與 [[original-mack-model]] 加入 \(r_{\min}\) 後的參數定義不同。

## Meyerhofer plot

Meyerhofer plot 以初始抑制劑濃度展示速率變化。在 [[dnq]]／[[novolac]] 範例中，添加更多抑制劑使 \(r_{\max}\) 上升、\(r_{\min}\) 下降。來源以 \(k_{\mathrm{enh}}\propto M_0\)、\(k_{\mathrm{inh}}\propto M_0^2\) 近似解釋觀察到的 log-rate 趨勢；這是合理模型解釋，不是唯一機制證明。

來源：[[19-optical-lithography--8-07-ch7--dxj4cs]] §7.1.2。