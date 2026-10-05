---
type: concept
title: notch model
created: 2026-10-03
updated: 2026-10-03
tags: [development, resist-model]
related: [19-optical-lithography--8-07-ch7--dxj4cs, original-mack-model, enhanced-kinetic-model]
sources: ["optical-lithography/07 - ch7.pdf"]
---
# notch model

notch model 是在反應控制速率式上乘入額外閾值函數的半經驗式顯影模型，用來描述局部急劇下降的速率：

\[
r=r_{\max}(1-m)^n
\frac{(a+1)(1-m)^{n_{\mathrm{notch}}}}
{a+(1-m)^{n_{\mathrm{notch}}}}+r_{\min},
\]

\[
a=\frac{n_{\mathrm{notch}}+1}{n_{\mathrm{notch}}-1}
(1-m_{\mathrm{th,notch}})^{n_{\mathrm{notch}}}.
\]

\(m_{\mathrm{th,notch}}\) 控制 notch 位置；\(n_{\mathrm{notch}}\) 控制其強度。

## 使用理由與限制

來源圖 7.4 的資料約在 \(m=0.5\) 出現比 [[original-mack-model]] 與 [[enhanced-kinetic-model]] 更陡的速率下降。此區域具有最大的相對速率變化，對輪廓預測重要，不能只追求全曲線平均擬合。

來源只稱少數光阻需要此模型，未提供完整樣本或普遍適用性證據。notch 參數是半經驗式形狀描述，不足以單獨辨識分子機制。

來源：[[19-optical-lithography--8-07-ch7--dxj4cs]] §7.1.2。