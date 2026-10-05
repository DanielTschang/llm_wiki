---
type: concept
title: PSM
created: 2026-10-03
updated: 2026-10-03
tags: [optical-lithography, PSM, interference]
related: [19-optical-lithography--9-10-ch10--y2hwiy, ret, alternating-psm, attenuated-psm, alternating-psm-與-attenuated-psm-比較]
sources: ["optical-lithography/10 - ch10.pdf"]
---
# PSM

Phase-Shifting Masks（PSMs）是同時調整透射光振幅與相位的光罩，利用不同區域透射光的干涉改善微影影像。

## 相位差的形成

來源以不同厚度的玻璃或 fused silica 產生光程差。垂直傳播的幾何近似下：

\[
\Delta\phi=\frac{2\pi d(n_g-1)}{\lambda},
\qquad
d_{180}=\frac{\lambda}{2(n_g-1)}.
\]

以上為來源方程式的可讀整理形式。實際窄小蝕刻區的繞射會使相位偏離此直線傳播近似，也可能造成強度不平衡。

## 主要類型

- [[alternating-psm]]：相鄰透明區交替使用 0° 與 180° 相位，藉相位邊界形成窄暗線。
- [[attenuated-psm]]：原本遮光區保留少量、相差 180° 的透射光，改善邊緣影像。

兩者的版圖負擔、誤印機制及相位容差不同，詳見 [[alternating-psm-與-attenuated-psm-比較]]。不能因同屬 PSM 就合併其相位誤差效應。