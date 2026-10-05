---
type: methodology
title: 以 A 量測 PAC 熱分解
created: 2026-10-03
updated: 2026-10-03
tags: [PAB, 量測, 熱分解]
related: [dill-abc-參數, kodak-820-的-pab-熱分解參數, 晶圓烘烤熱歷程建模, 19-optical-lithography--8-05-ch5--x0rq4s]
sources: ["optical-lithography/05 - ch5.pdf"]
---
# 以 A 量測 PAC 熱分解

此方法利用可漂白吸收係數 \(A\) 與剩餘感光成分的比例關係，間接量測 PAB 引起的 PAC 熱分解。

\[
\frac{A}{A_{\mathrm{NB}}}=m'=\exp(-K_Tt_b),\qquad
K_T=A_r\exp(-E_a/RT).
\]

## 流程與理由

在多個固定溫度下量測不同烘烤時間的 \(A\)。繪製 \(\ln(A/A_{\mathrm{NB}})\) 對時間的曲線，以斜率估計 \(-K_T\)，再以 \(\ln K_T\) 對 \(1/T\) 擬合 \(E_a\) 與 \(A_r\)。

此方法將光學量測連接至組成變化，不必直接量測每一種分解產物。

## 重要控制

- \(A_{\mathrm{NB}}\) 與量測 \(A\) 必須使用相同波長。
- 烘烤初期的升溫延遲不能算成已在目標溫度下反應。
- Arrhenius 假設須在所用溫度範圍成立；接近玻璃轉移時應謹慎。
- 水分可改變分解產物比例；\(A\) 的減少本身不會完整辨識羧酸與酯。
- 非恆溫情況應採[[晶圓烘烤熱歷程建模]]，而非只使用設定溫度。

## 應用

[[kodak-820-的-pab-熱分解參數]]使用本章圖 5.3 的分析。來源：[[19-optical-lithography--8-05-ch5--x0rq4s]]，第 5.2.1 節。