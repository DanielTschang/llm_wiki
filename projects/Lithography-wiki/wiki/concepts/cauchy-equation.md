---
type: concept
title: Cauchy equation
created: 2026-10-03
updated: 2026-10-03
tags: [色散, 折射率, 光阻]
related: [19-optical-lithography--8-04-ch4--ccwms, 光阻內成像]
sources: ["optical-lithography/04 - ch4.pdf"]
---
# Cauchy equation

Cauchy equation 是在有限波長範圍內，以經驗係數近似折射率色散的方程。本章使用：

\[
n(\lambda)=C_1+\frac{C_2}{\lambda^2}+\frac{C_3}{\lambda^4}.
\]

## 使用邊界

此式適合指定波段內無吸收或弱吸收的介電材料，不能任意外插，也不能單憑實部擬合取代吸收材料的完整複數光學常數。

若波長以 nm 表示，\(C_2\) 與 \(C_3\) 的單位分別為 nm² 與 nm⁴。曝光前後的係數必須分開使用；SPR-500 i-line 的兩組係數不得互換。

## 來源資料

[[19-optical-lithography--8-04-ch4--ccwms]] 表 4.2 保存曝光前後及 193-nm resist 的係數與適用波長範圍。這些是來源示例，不是通用產品規格。