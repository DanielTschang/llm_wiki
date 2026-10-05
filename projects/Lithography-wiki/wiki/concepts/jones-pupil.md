---
type: concept
title: Jones pupil
created: 2026-10-03
updated: 2026-10-03
tags: [偏振, 瞳孔, 像差]
related: [19-optical-lithography--8-03-ch3--s477p8, 向量成像與偏振, flare]
sources: ["optical-lithography/03 - ch3.pdf"]
---
# Jones pupil

Jones pupil 是隨瞳孔位置變化的複數 2×2 傳輸矩陣，用於描述偏振分量的振幅、相位及交叉耦合。

\[
\begin{pmatrix}E_{oTE}\\E_{oTM}\end{pmatrix}
=
\begin{pmatrix}J_{11}&J_{12}\\J_{21}&J_{22}\end{pmatrix}
\begin{pmatrix}E_{iTE}\\E_{iTM}\end{pmatrix}.
\]

## 物理內容

- diattenuation：不同偏振的振幅透射差異。
- retardance：不同偏振的相位延遲差異。
- 雙折射：折射率隨電場方向變化，可由材料本身或應力產生。

本章以
\[
P=(J_{11}+J_{22})/2,\qquad J=P\widetilde J
\]
分離純量瞳孔與偏振相關部分；無偏振依賴時，\(\widetilde J\) 為單位矩陣。

## 原文校對

書頁 115 的擷取文字寫 `J12 = J21 = 0 and J11 = J12`，並寫 `J11 = J12 = P`。這與矩陣及式 (3.74) 不一致。

依矩陣結構推得的純量極限應為
\[
J_{12}=J_{21}=0,\qquad J_{11}=J_{22}=P.
\]
此式是明確標示的數學一致性修正，尚不能判定錯誤來自原書排印或文字擷取。

## 限制

Jones pupil 不能直接表示 depolarization。本章建議以它描述非散射光，另用 [[flare]] 模型處理散射光，並假設到達晶圓的 flare 為隨機偏振。

依據 [[19-optical-lithography--8-03-ch3--s477p8]] 第 3.6.4 節。