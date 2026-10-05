---
type: concept
title: Original Mack model
created: 2026-10-03
updated: 2026-10-03
tags: [development, kinetics, resist-model]
related: [19-optical-lithography--8-07-ch7--dxj4cs, enhanced-kinetic-model, notch-model, critical-ionization-model]
sources: ["optical-lithography/07 - ch7.pdf"]
---
# Original Mack model

Original Mack model 是將顯影劑輸送與光阻表面反應串聯，並加入未曝光光阻溶解項的顯影速率模型。

\[
r=r_{\max}\frac{(a+1)(1-m)^n}{a+(1-m)^n}+r_{\min},
\qquad
a=\frac{k_D}{k_RM_0^n}.
\]

\(m=M/M_0\) 是相對抑制劑濃度；\(n\) 是溶解選擇性；\(k_D\) 與 \(k_R\) 分別描述輸送與反應。當反曲點存在且參數適用時：

\[
a=\frac{n+1}{n-1}(1-m_{\mathrm{th}})^n.
\]

## 參數與極限

- \(a\) 大：反應控制，輸送較快。
- \(a\) 小：輸送限制較重要。
- \(m=1\)：\(r=r_{\min}\)。
- \(m=0\)：\(r=r_{\max}+r_{\min}\)，不是單獨 \(r_{\max}\)。
- \(a\gg1\)：\(r\approx r_{\max}(1-m)^n+r_{\min}\)。

高 \(n\) 使響應更接近閾值，但仍需要足夠的最大／最小速率比。高分子量可能提高選擇性，卻降低速率與感度，不能只最佳化單一參數。

## 證據與限制

模型忽略快速產物移除，並將表面反應簡化為宏觀事件階數。聚合物分布使擬合 \(n\) 不宜視為每條鏈固定的確切事件數。與 [[enhanced-kinetic-model]] 的同名 \(r_{\max}\) 定義不同；比較時必須先轉換端點速率。

來源：[[19-optical-lithography--8-07-ch7--dxj4cs]] §7.1.1。