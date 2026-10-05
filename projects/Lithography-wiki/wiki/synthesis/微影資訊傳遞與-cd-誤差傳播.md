---
type: synthesis
title: 微影資訊傳遞與 CD 誤差傳播
created: 2026-10-03
updated: 2026-10-03
tags: [optical-lithography, error-propagation, NILS]
related: [19-optical-lithography--8-09-ch9--1xlwz30, nils-與影像邊緣品質, 光阻內成像, peb-擴散與-dpsf, rdpsf, original-mack-model, lumped-parameter-model, 線邊緣粗糙度]
sources: ["optical-lithography/09 - ch9.pdf"]
---
# 微影資訊傳遞與 CD 誤差傳播

第 9 章將微影視為設計、光罩、光學影像、化學潛像、顯影速率與輪廓之間的連續轉換，並以局部導數追蹤小誤差如何影響 CD。此處「資訊」是製程組織框架，不是正式的 Shannon 資訊量計算。

本綜整以 [[19-optical-lithography--8-09-ch9--1xlwz30]] 為證據；下列既有頁面作為概念入口，不表示已核對其全文或新增跨來源驗證。

## 梯度傳遞鏈

\[
I(x)\rightarrow m(x)\ \text{或}\ h(x)
\rightarrow m^*(x)\rightarrow r(x)
\rightarrow\text{光阻邊緣}.
\]

對以 \(m\) 表達的轉換鏈，可寫為：

\[
\frac{\partial\ln r}{\partial x}
=
\frac{\partial\ln r}{\partial m^*}
\frac{\partial m^*}{\partial m}
\frac{\partial m}{\partial\ln E}
\frac{\partial\ln I}{\partial x}.
\]

最後的光學項由 [[nils-與影像邊緣品質]] 描述；中間項包含曝光及 [[peb-擴散與-dpsf]] 或 [[rdpsf]]；顯影轉換可由 [[original-mack-model]] 分析。三個化學導數合併後，對應章節的局部光阻對比度 \(\gamma\)。

## 邊緣位置仍取決於顯影路徑

高顯影速率梯度並不足以完整決定 CD 敏感度。[[lumped-parameter-model]] 的近似顯示，除 \(2/NILS\) 的光學項外，仍有包含有效高寬比與邊緣／空間中心強度比的路徑修正。

因此最佳化至少需區分：

- 光學邊緣品質；
- 化學潛像品質；
- 顯影對化學差異的辨識力；
- 最終輪廓對劑量與焦距誤差的敏感度。

膜堆疊與計算深度亦不可省略，因為 [[光阻內成像]] 不必等於空中像。

## 確定性與隨機目標

CD 劑量控制關注平均響應的局部導數；[[線邊緣粗糙度]] 同時取決於濃度變異與梯度。減少擴散可改善確定性梯度，卻可能失去降低化學雜訊的效果。

此框架適合引導篩選與診斷，但一階近似不涵蓋大誤差、任意三維路徑或 isofocal bias。完整製程窗口與 LER 驗證仍不可省略。