---
type: methodology
title: Abbe 成像計算法
created: 2026-10-03
updated: 2026-10-03
tags: [成像計算, 部分同調]
related: [空間同調與部分同調, hopkins-成像計算法, sum-of-coherent-sources, 19-optical-lithography--8-02-ch2--k73j1g]
sources: ["optical-lithography/02 - ch2.pdf"]
---
# Abbe 成像計算法

Abbe 成像計算法將擴展光源拆成互不相干的來源點，逐點計算同調影像，再以来源強度加權積分。

## 物理理由

單一来源點所產生的繞射波可以干涉，故先加總電場；不同來源點沒有固定相位關係，故只能加總強度。

## 計算步驟

1. 指定遮罩電場透射率、瞳孔與來源強度分布 \(S\)。
2. 對各來源點，依入射方向平移遮罩繞射頻譜。
3. 經瞳孔濾波及反變換得到 \(E_q\)。
4. 計算正規化強度：
   \[
   I=\frac{\int S(q)|E_q|^2\,dq}{\int S(q)\,dq}.
   \]

數值實作須檢查來源取樣與頻率離散化的收斂。[[19-optical-lithography--8-02-ch2--k73j1g]] 未指定通用取樣數或誤差標準。

## 使用條件

此方法以來源點互不相干及本章標量遮罩模型為前提。[[hopkins-成像計算法]] 是同一模型的積分重排，而非不同光學定律。