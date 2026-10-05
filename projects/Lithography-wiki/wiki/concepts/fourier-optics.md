---
type: concept
title: Fourier optics
created: 2026-10-03
updated: 2026-10-03
tags: [繞射, 成像理論]
related: [光學微影, 數值孔徑與解析度, 點擴散函數, 19-optical-lithography--8-02-ch2--k73j1g]
sources: ["optical-lithography/02 - ch2.pdf"]
---
# Fourier optics

Fourier optics 以空間頻率描述繞射與成像：遮罩下方電場經 Fourier transform 成為繞射頻譜，再由瞳孔濾波及反變換得到影像。

## 理想標量模型

\[
T_m=\mathcal F\{E_i t_m\},\qquad
E=\mathcal F^{-1}\{T_mP\}.
\]

空氣中的空中像強度為 \(I=|E|^2\)。有限瞳孔丟失頻譜資訊，即使鏡頭理想仍不會完整重建一般遮罩。

週期圖案產生離散繞射級次；孤立開口則產生連續頻譜。因此密集圖案與孤立特徵的解析度不能直接互換，詳見 [[數值孔徑與解析度]]。

## 線性與邊界

同調成像對電場透射率線性，其實空間形式是遮罩透射率與電場 [[點擴散函數]] 的卷積；強度通常不是同一線性算子的輸出。

[[19-optical-lithography--8-02-ch2--k73j1g]] 使用理想鏡頭與標量近似。若採用 Kirchhoff boundary condition，遮罩立體繞射及偏振依賴也被忽略。原文 Fourier scaling 敘述與公式疑似不一致，應回查 PDF，不能沿用其縮放文字作為規則。