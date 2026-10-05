---
type: finding
title: 酸擴散造成 LER 雜訊與梯度權衡
source: "[[19-optical-lithography--8-09-ch9--1xlwz30]]"
confidence: medium
replicated: null
created: 2026-10-03
updated: 2026-10-03
tags: [LER, acid-diffusion, stochastic-modeling]
related: [19-optical-lithography--8-09-ch9--1xlwz30, 線邊緣粗糙度, 光阻化學隨機建模, 潛像梯度與-chemical-contrast]
sources: ["optical-lithography/09 - ch9.pdf"]
---
# 酸擴散造成 LER 雜訊與梯度權衡

來源的簡化模型預測，酸擴散同時降低化學濃度波動與平均潛像梯度，因此 LER 隨擴散長度可能先改善、再惡化。

## 直接證據

式（9.93）–（9.99）將濃度變異透過一階位置誤差傳播轉換為：

\[
LER\propto\frac{\sigma_{m^*}}{|dm^*/dx|}.
\]

圖 9.28 對 45 nm 特徵與 1、2、3 nm reaction capture range 的二維及三維模型，均呈現有限的 LER 最佳擴散長度。

## 限制與解讀

圖中的縱軸為任意單位，不能當成絕對 LER 預測。簡化分析忽略 quencher，部分步驟忽略 photon shot noise，並採無限對比度閾值顯影。

由模型可推論，僅最大化確定性梯度的最佳化策略可能錯過最低 LER；實際位置仍須以有限對比度顯影與實驗量測確認。來源未提供獨立重現證據。