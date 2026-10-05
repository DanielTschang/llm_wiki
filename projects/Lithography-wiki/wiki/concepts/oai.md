---
type: concept
title: OAI
created: 2026-10-03
updated: 2026-10-03
tags: [optical-lithography, OAI, NILS, DOF]
related: [19-optical-lithography--9-10-ch10--y2hwiy, ret, sraf, 空間同調與部分同調, 離焦與焦深, nils-與影像邊緣品質]
sources: ["optical-lithography/10 - ch10.pdf"]
---
# OAI

Off-Axis Illumination（OAI）是顯著減少或移除近軸照明的照明配置，透過傾斜入射光移動光罩繞射階，改善特定節距圖案的離焦表現。

## 最佳化原理

在來源的理想週期圖案模型中，使通過鏡頭的零階與一階光束對稱於鏡頭中心，可改善 DOF。理想 dipole 偏移量為：

\[
\sigma_{\mathrm{opt}}=\frac{\lambda}{2pNA},
\]

其中 \(p\) 為圖案節距，\(\sigma\) 為來源採用的正規化照明座標。此關係式為可讀整理形式，不是 OCR 逐字引文。

## 照明形狀與方向

- **dipole**：針對單一線方向最佳化；另一方向可能明顯退化。
- **quadrupole**：同時提供水平與垂直線所需的傾斜，但不保證 45° 等其他方向最佳。
- **annular illumination**：涵蓋較廣泛的線方向；其最佳配置仍取決於節距。
- 重複二維圖案也可使用客製 source shape。

## NILS 與離焦權衡

來源的三光束轉雙光束情境中，移除一個繞射階降低 in-focus NILS，卻使 NILS 隨 defocus 的變化更小。此結論限於該成像轉換，不能泛化至所有 OAI 配置。參見 [[nils-與影像邊緣品質]]。

## forbidden pitches

OAI 的收益集中於設計節距附近；孤立線不會自動取得同樣的 DOF 改善。[[sraf]] 可補足較孤立圖案，但需要配置空間。

來源將約最小節距 1.3–1.7 倍的低 NILS／DOF 區間稱為 forbidden pitches。這是照明收益與 SRAF 可配置性共同形成的經驗限制，並非不可成像的絕對物理禁區。來源見 [[19-optical-lithography--9-10-ch10--y2hwiy]]。