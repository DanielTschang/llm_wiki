---
type: finding
title: 低 k1 光阻對比度影響 MEEF
created: 2026-10-03
updated: 2026-10-03
tags: [模擬, 光阻對比度, MEEF]
related: [mask-error-enhancement-factor, original-mack-model, 光阻理論對比度與量測對比度]
sources: ["optical-lithography/08 - ch8.pdf"]
source: "[[19-optical-lithography--8-08-ch8--1az2fh0]]"
confidence: medium
replicated: null
---
# 低 k1 光阻對比度影響 MEEF

## 直接證據

本章 §8.7.6、Figure 8.37 使用虛擬光阻模擬等線／間距圖形，條件為 248-nm 曝光、NA = 0.6、partial coherence = 0.5、500-nm 光阻，基板為 BARC／silicon。

以 [[original-mack-model]] 的 dissolution selectivity parameter \(n\) 控制對比度：

| 虛擬光阻 | n | k1 = 0.5 時的 MEEF |
|---|---:|---:|
| 高對比度 | 25 | 約 1.5 |
| 中對比度 | 10 | 約 2.0 |
| 低對比度 | 5 | 大於 3.0 |

較大圖形的 MEEF 接近 1，對比度影響較小；接近解析度極限時，不同對比度的差異迅速增大。

## 推論與限制

此結果支持在該模型下，提高光阻對比度能減少低 \(k_1\) 的 mask 誤差放大。它不是具名商用材料實測，也不是所有照明、膜厚或化學系統的通則。

來源未提供獨立重現資訊，因此重現狀態未知。