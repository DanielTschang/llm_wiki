---
type: methodology
title: practical contrast 量測
created: 2026-10-03
updated: 2026-10-03
tags: [contrast, metrology, development]
related: [19-optical-lithography--8-07-ch7--dxj4cs, 光阻理論對比度與量測對比度, development-rate-monitor]
sources: ["optical-lithography/07 - ch7.pdf"]
---
# practical contrast 量測

practical contrast 量測以不同曝光劑量的清除時間取代固定時間後的剩餘膜厚，旨在減少深度效應對傳統對比度估計的扭曲。來源將此名稱歸於 Peter Gwozdz。

## 必要假設與計算

若速率可分離為 \(r(E,z)=f(E)g(z)\)：

\[
t_{\mathrm{clear}}=\frac{1}{f(E)}
\int_0^D\frac{dz}{g(z)},
\qquad
\gamma_{\mathrm{th}}
=-\frac{d\ln t_{\mathrm{clear}}}{d\ln E}.
\]

因此 time-to-clear 對 dose 的 log-log 曲線負斜率估計理論對比度。

## 執行與限制

在多個 open-frame 劑量量測清除時間，再取對數斜率。慢速、較厚的 g-line 或 i-line 光阻可用目視與碼表估計；快速化學放大型光阻可能在不到一秒內清除，需要自動化設備。

高對比度區的清除時間通常較接近正常顯影時間，估計最大對比度可能比完整曲線容易。但若劑量改變深度響應形狀，可分離條件不成立，不能直接將斜率視為材料理論對比度。

參見 [[光阻理論對比度與量測對比度]]；來源：[[19-optical-lithography--8-07-ch7--dxj4cs]] §7.2.3。