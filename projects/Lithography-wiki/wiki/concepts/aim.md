---
type: concept
title: Advanced Imaging Metrology (AIM)
created: 2026-10-03
updated: 2026-10-03
tags: [量測, overlay, 標靶]
related: [overlay-控制, 19-optical-lithography--8-08-ch8--1az2fh0]
sources: ["optical-lithography/08 - ch8.pdf"]
---
# Advanced Imaging Metrology (AIM)

Advanced Imaging Metrology（AIM）是本章介紹的多條帶 overlay 標靶與影像量測方法；不同微影層分別印製內側與外側條帶，再估計其相對位置。

## 設計理由

相較 box-in-box，多條帶對 CMP dishing 及其他可能損傷標靶的製程效應較不敏感。多條帶也能在同次量測中提供平均效果，提高精密度。

影像處理估計各條帶的對稱中心，再轉換為 x、y overlay。這不表示所有晶圓誘發不對稱都已消除；仍須進行 [[overlay-控制]]中的標靶診斷。

## 證據界線

本章提供設計機制與較佳量測結果的敘述，未提供可跨產品直接套用的精密度數值。圖片提供者為 KLA-Tencor Corp.，不能將圖片署名視為完整產品規格。