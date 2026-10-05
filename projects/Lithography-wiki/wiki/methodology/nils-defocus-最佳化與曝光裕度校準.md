---
type: methodology
title: NILS–defocus 最佳化與曝光裕度校準
created: 2026-10-03
updated: 2026-10-03
tags: [NILS, process-window, optimization]
related: [19-optical-lithography--8-09-ch9--1xlwz30, nils-與影像邊緣品質, 焦距與曝光製程窗口, lumped-parameter-model]
sources: ["optical-lithography/09 - ch9.pdf"]
---
# NILS–defocus 最佳化與曝光裕度校準

此方法利用名義邊緣的 NILS 隨離焦變化曲線，篩選光學參數，再以曝光裕度校準最低可接受 NILS。其目的在於以快速、具物理意義的局部指標引導實驗，而非取代完整製程窗口量測。

## 操作程序

1. 固定目標圖案、線寬、pitch、光罩與膜堆疊；不同圖案分別評估。
2. 選定空中像或膜內影像。使用膜內影像時，明確記錄計算深度及膜內最佳焦面。
3. 掃描離焦、NA、照明與其他待最佳化參數，計算名義邊緣 NILS。
4. 在指定 CD 容許範圍下，取得曝光裕度隨離焦變化的實驗資料，或明確標示為模擬資料。
5. 配對相同離焦條件的 NILS 與曝光裕度，擬合製程相依關係：
   \[
   \%EL\approx\alpha(NILS-\beta).
   \]
6. 由最低曝光裕度要求換算 NILS 門檻；在該門檻下最大化焦深，或在固定離焦要求下最大化 NILS。
7. 以完整 focus–exposure matrix 驗證 CD、輪廓及其他製程限制。

## 為何需要校準

理想光阻的 \(\%EL\approx10NILS\) 假設無限對比度、CD 容許範圍 ±10%，且區間內梯度近似固定。實際製程的曝光、PEB、顯影路徑與有效高寬比使此關係改變。

圖 9.6 的 UV6 示例是模擬校準，而非實測通用標準；其係數不得跨光阻或跨膜堆疊套用。

## 判讀規則與限制

- 不同 NA 的 NILS–defocus 曲線可能交叉，故最佳 NA 取決於圖案、離焦要求及 NILS 門檻。
- 不把 CD 劑量敏感度與曝光裕度混用：前者是相對導數大小，後者隨其倒數增加。
- 梯度法不能捕捉 isofocal bias 對焦深的影響。
- 來源部分離焦單位的 OCR 異常，未核對原 PDF 前不採用其數字作設備設定。

理論依據見 [[19-optical-lithography--8-09-ch9--1xlwz30]]；完整窗口背景見 [[焦距與曝光製程窗口]]。