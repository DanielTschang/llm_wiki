---
type: query
title: 不等酸與 quencher 擴散係數如何影響 NILS 與 LER 最佳化？
created: 2026-10-03
updated: 2026-10-03
tags: [quencher, reaction-diffusion, LER]
related: [19-optical-lithography--8-09-ch9--1xlwz30, base-quencher, rdpsf, 潛像梯度與-chemical-contrast, 線邊緣粗糙度]
sources: ["optical-lithography/09 - ch9.pdf"]
---
# 不等酸與 quencher 擴散係數如何影響 NILS 與 LER 最佳化？

## 問題背景

[[19-optical-lithography--8-09-ch9--1xlwz30]] 的含 quencher 解析近似要求酸—鹼反應遠快於擴散，且兩者擴散係數完全相同。在此條件下，可對帶符號淨酸分布作卷積，並在放大計算前將負有效酸濃度截為零。

另一方面，章節的 LER 趨勢模型忽略 quencher，因此不能直接把 quencher 改善平均梯度的結果等同於改善 LER。

## 待驗證問題

1. 酸與 quencher 擴散係數不等時，單一 [[rdpsf]] 是否仍能近似有效酸分布？
2. 固定光學 NILS 時，quencher 如何同時改變平均潛像梯度與濃度變異？
3. 最大梯度與最小 LER 的 PEB 條件是否分離？
4. 有限顯影對比度是否改變上述最佳化排序？

## 所需證據

需要含酸、quencher、去保護反應與隨機波動的耦合模型，掃描擴散係數比、quencher loading、劑量及 PEB 條件，並以相同 CD 目標比較梯度與 LER。

實驗驗證應固定光學影像與膜堆疊，交代 LER 量測定義，並區分相對趨勢與絕對預測。此問題是來源適用範圍的延伸，並非已確認的跨來源矛盾。