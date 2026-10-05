---
type: finding
title: Kodak 820 的 PAB 熱分解參數
source: "[[19-optical-lithography--8-05-ch5--x0rq4s]]"
confidence: medium
replicated: null
created: 2026-10-03
updated: 2026-10-03
tags: [Kodak-820, PAB, 熱分解]
related: [kodak-820, 以-a-量測-pac-熱分解, 晶圓烘烤熱歷程建模]
sources: ["optical-lithography/05 - ch5.pdf"]
---
# Kodak 820 的 PAB 熱分解參數

本章圖 5.3 的 Kodak 820 資料，以 365 nm 吸收參數 \(A\) 追蹤不同溫度與時間的對流 PAB，支持 PAC 熱分解的動力學擬合。

## 結果與證據層次

- **量測資料：**\(A\) 隨烘烤條件下降。
- **模型擬合：**\(E_a=30.3\) Kcal/mol，\(\ln(A_r)=35.3\)，其中 \(A_r\) 使用 1/minute。
- **來源據模型推估：**100 °C、30 分鐘對流 PAB 約分解 11% PAC。

方法見[[以-a-量測-pac-熱分解]]。參數不是直接量測的分解產物濃度，也不會獨立辨識羧酸與酯的比例。

## 限制

圖中早期延遲與升溫有關；來源描述對流烘烤厚玻璃基板及載具的升溫延遲約 11 分鐘。不可直接將此延遲或總烘烤時間套用至 silicon wafer 的 proximity hot plate。

信心為中等：有引用實驗與模型擬合，但原始資料、擬合不確定度及獨立重複驗證狀態未在本次匯入中確認。結果僅歸屬 [[kodak-820]]。