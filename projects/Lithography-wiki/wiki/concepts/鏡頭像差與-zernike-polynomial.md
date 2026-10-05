---
type: concept
title: 鏡頭像差與 Zernike polynomial
created: 2026-10-03
updated: 2026-10-03
tags: [像差, 波前, 瞳孔]
related: [19-optical-lithography--8-03-ch3--s477p8, fourier-optics, 含像差與離焦的瞳孔建模, 離焦與焦深]
sources: ["optical-lithography/03 - ch3.pdf"]
---
# 鏡頭像差與 Zernike polynomial

鏡頭像差是實際成像偏離理想繞射極限的行為，可由波前光程差 OPD 描述。Zernike polynomial 是單位圓上的完備正交展開，用係數表示瞳孔波前誤差。

## 表示方式

\[
W(R,\phi)=\frac{OPD}{\lambda}=\sum_i Z_iF_i(R,\phi),
\qquad
P=P_{\mathrm{ideal}}e^{i2\pi W}.
\]

本章採 Fringe 慣例，piston 為 Z0；其他慣例可能由 Z1 起算。完整係數基底保存在 [[19-optical-lithography--8-03-ch3--s477p8]] 的表 3.1。完整單位圓上的正交性，不應無條件延伸至任意遮蔽或加權瞳孔。

## 製程後果

| 像差 | 主要後果 |
|---|---|
| Tilt | 圖案平移；其場依賴變化形成畸變 |
| 球面像差 | 節距相關最佳焦點 |
| 像散 | 方向相關最佳焦點及焦距相關 H–V bias |
| 彗差 | 圖案相關位置偏移、左右 CD 與輪廓不對稱 |

這些效應取決於圖案如何取樣瞳孔，不可由全域 RMS 或 Strehl ratio 一概推定。像差亦隨場位置與波長變化。

## 來源與限制

依據本章第 3.1 節。像差可來自設計、製造及使用條件；圖案輪廓案例包含模擬，不能當作所有設備的實測結果。計算流程見 [[含像差與離焦的瞳孔建模]]。