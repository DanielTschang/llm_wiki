---
type: concept
title: flare
created: 2026-10-03
updated: 2026-10-03
tags: [雜散光, 散射, 成像品質]
related: [19-optical-lithography--8-03-ch3--s477p8, 大型島形圖案-flare-量測, 點擴散函數, jones-pupil]
sources: ["optical-lithography/03 - ch3.pdf"]
---
# flare

flare 是成像系統中非預期反射與散射到達晶圓的光，會曝光原應為暗區的位置，降低影像品質。

## 圖案與場依賴性

flare 不只是固定设备常數；它依賴局部圖案、曝光場總透光量、場位置及晶圓反射率。表面粗糙、污染、玻璃不均勻與非理想抗反射膜皆可能產生雜散光。

## 模型

均勻長程散射的近似為
\[
I(x,y)=SF\cdot EF+(1-SF)I_0(x,y),
\]
其中 \(SF\) 是散射比例，\(EF\) 是相對全透明光罩的到達晶圓能量比例。以透光面積比例 \(CF\) 取代 \(EF\) 是近似，因繞射及瞳孔裁切會改變實際能量。

短程散射可用
\[
I_{\mathrm{scat}}=PSF_{\mathrm{scat}}*I_0
\]
表示。散射 PSF 與理想成像 PSF、向量 PSF 的用途不同，不應混用。

## 量測與限制

[[大型島形圖案-flare-量測]] 以 \(E_0/E_{0\text{-island}}\) 估算暗區雜散曝光。均勻 DC 模型不能涵蓋完整短程及場依賴行為。

依據 [[19-optical-lithography--8-03-ch3--s477p8]] 第 3.3 節。