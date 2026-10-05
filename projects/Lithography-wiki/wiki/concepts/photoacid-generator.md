---
type: concept
title: photoacid generator
created: 2026-10-03
updated: 2026-10-03
tags: [PAG, 光阻化學, 曝光]
related: [化學放大型光阻, dill-abc-參數, 光阻化學隨機建模, 19-optical-lithography--8-06-ch6--1wif8]
sources: ["optical-lithography/06 - ch6.pdf"]
---
# photoacid generator

photoacid generator（PAG）是曝光後產酸的感光成分，為[[化學放大型光阻]]提供催化反應所需的酸潛像。

## 曝光動力學

本章採用一階曝光模型：

\[
\frac{\partial G}{\partial t}=-CIG.
\]

若曝光期間強度不變且無酸損失：

\[
G=G_0e^{-CIt},\qquad H=G_0(1-e^{-CIt}).
\]

部分 248-nm 配方也可透過聚合物吸光後的能量轉移間接激發 PAG；來源指出其整體動力學仍可由一階模型描述。

## 配方與量測取捨

2007 年來源所述 PAG loading 為：248-nm 約 5–15 wt%，193-nm 約 1–5 wt%。較低的 193-nm loading 與吸收限制有關；PAG 數量也影響產酸上限及分子計數變異。

多數不漂白系統具有 \(A\approx0\)，傳統透射率法不能辨識 \(C\)。可改用酸滴定、螢光或已知鹼量的劑量閾值。見[[dill-abc-參數]]。

來源：[[19-optical-lithography--8-06-ch6--1wif8]]，§6.1、§6.3。