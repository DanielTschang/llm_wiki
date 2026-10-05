---
type: concept
title: Köhler illumination
created: 2026-10-03
updated: 2026-10-03
tags: [照明系統, 光學微影]
related: [august-köhler, 空間同調與部分同調, fourier-optics, 19-optical-lithography--8-02-ch2--k73j1g]
sources: ["optical-lithography/02 - ch2.pdf"]
---
# Köhler illumination

Köhler illumination 是將光源成像於物鏡入口瞳孔，並將遮罩置於聚光鏡出口瞳孔的照明配置，以 [[august-köhler]] 命名。

## 配置的理由

不同遮罩位置需要適當的入射傾斜，使相同圖案的繞射頻譜落在物鏡瞳孔相同位置。匯聚至入口瞳孔的球面波提供這種視場相關方向，改善全視場成像一致性。

每個遮罩位置接收來自各光源點的光，因此遮罩照明均勻性不直接取決於光源本身的強度均勻性；這不是任意真實系統皆完全均勻的保證。

## 局部平面波近似

球面波曲率半徑相對圖案足夠大時，局部繞射仍可使用平面波近似。[[19-optical-lithography--8-02-ch2--k73j1g]] 說明此配置讓入口瞳孔的繞射能以 [[fourier-optics]] 表述，並與來源形狀控制相容。