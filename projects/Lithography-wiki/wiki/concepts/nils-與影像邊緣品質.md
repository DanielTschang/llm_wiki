---
type: concept
title: NILS 與影像邊緣品質
created: 2026-10-03
updated: 2026-10-03
tags: [NILS, 成像品質, CD]
related: [19-optical-lithography--8-03-ch3--s477p8, kintner-成像計算法, 圖案轉移, 向量成像與偏振, 離焦與焦深]
sources: ["optical-lithography/03 - ch3.pdf"]
---
# NILS 與影像邊緣品質

Normalized Image Log-Slope（NILS）是名義線寬乘以名義邊緣的影像對數斜率，用於量化影像對局部邊緣位置的控制能力。

\[
ILS=\frac{1}{I}\frac{dI}{dx}=\frac{d\ln I}{dx},
\qquad NILS=w\frac{d\ln I}{dx}.
\]

## 計算與慣例

評估點是名義圖案邊緣，不是任意最大斜率位置。NILS 本可有正負號，通常調整為正值。

若等線／空影像只有至二次諧波，
\[
I(x)=\beta_0+\beta_1\cos(2\pi x/p)+\beta_2\cos(4\pi x/p),
\]
則在 \(w=p/2\) 的指定邊緣，
\[
NILS=-\frac{\pi\beta_1}{\beta_0-\beta_2}.
\]
[[kintner-成像計算法]] 的光束權重可轉為這些 Fourier 係數。

## 模型結果與限制

本章的同調、最佳焦點、等線／空三光束 TE 模型得到 NILS = 8。離焦及 TM 偏振會降低該模型的 NILS；完整條件及係數見 [[19-optical-lithography--8-03-ch3--s477p8]] 表 3.2。

NILS 是局部光學指標，不取代光阻化學、膜層傳播、三維輪廓或完整製程模擬。全域影像對比與全域波前誤差亦不能直接替代它。