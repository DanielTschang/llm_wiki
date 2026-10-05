---
type: comparison
title: overlay 與 pattern placement error 比較
created: 2026-10-03
updated: 2026-10-03
tags: [overlay, PPE, 像差]
related: [overlay-控制, 鏡頭像差與-zernike-polynomial, 關鍵形狀誤差與邊緣位置誤差, 19-optical-lithography--8-08-ch8--1az2fh0]
sources: ["optical-lithography/08 - ch8.pdf"]
---
# overlay 與 pattern placement error 比較

overlay 與 pattern placement errors（PPE）都影響位置，但本章將通用的層間位置差，與依圖形尺寸及 pitch 改變的額外位置誤差分開。

| 面向 | overlay | PPE |
|---|---|---|
| 主要意義 | 新層相對既有層的位置差 | 特定圖形額外的位置差 |
| 圖形依賴性 | 本章視為各圖形共用的誤差 | 隨尺寸與 pitch 改變 |
| 像差例子 | 隨場位置變化的 tilt，即 distortion | coma 等高階奇像差 |
| 常見取得方式 | 專用較大 overlay 標靶 | 類裝置標靶或像差成像計算 |

## 工程意義

大型標靶量測合格，不保證接近解析度極限的装置圖形位置合格。PPE 可以疊加在通用 overlay 上，但必須使用相容的座標與符號定義。

CSE／EPE 是輪廓逐點差異的量化方式，不等同 PPE 的物理來源分類。

來源：[[19-optical-lithography--8-08-ch8--1az2fh0]]，§8.4.5。