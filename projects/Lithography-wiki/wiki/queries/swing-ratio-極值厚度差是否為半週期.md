---
type: query
title: swing ratio 極值厚度差是否為半週期？
created: 2026-10-03
updated: 2026-10-03
tags: [公式核對, swing-ratio, 薄膜干涉]
related: [19-optical-lithography--8-04-ch4--ccwms, swing-curves]
sources: ["optical-lithography/04 - ch4.pdf"]
---
# swing ratio 極值厚度差是否為半週期？

## 問題

來源 PDF 第 23 頁、印刷頁 151，在第 4.58–4.59 式之間將 \(D_{\max}=D+\Delta D/2\)、\(D_{\min}=D-\Delta D/2\)，並稱 \(\Delta D\) 為 swing 完整週期 \(\lambda/(2n_2)\)。但相鄰極大與極小，對理想單一餘弦應相隔半週期。

## 已知與尚未確定

已知正入射干涉項為 \(\cos(4\pi n_2D/\lambda+\phi)\)，完整週期是 \(\lambda/(2n_2)\)，其相鄰極大與極小的間距是 \(\lambda/(4n_2)\)。

尚未確定原文是否有排版錯誤、是否採特殊極值配對，或後續吸收修正的使用方式亦須調整。真實吸收斜率可能移動極值位置，也應與理想餘弦情形分開檢查。

## 核對方式

- 比對原版 PDF 字形與其他版本或作者勘誤。
- 從第 4.51 式重新求極值，而非只複製第 4.62 式。
- 分別測試無吸收與弱吸收案例，檢查相鄰極值配對及 \(\Delta D\) 符號。
- 釐清第 4.95 式的完整厚度變動範圍是否沿用相同定義。

在確認前，[[swing-curves]] 保留 SR 的直接數值定義，不將疑似厚度差敘述當作已確認公式。來源：[[19-optical-lithography--8-04-ch4--ccwms]]。