---
type: methodology
title: model-based OPC 校準與迭代
created: 2026-10-03
updated: 2026-10-03
tags: [OPC, compact-models, EPE, calibration]
related: [19-optical-lithography--9-10-ch10--y2hwiy, opc, prolith, sum-of-coherent-sources, variable-threshold-resist-model, 關鍵形狀誤差與邊緣位置誤差]
sources: ["optical-lithography/10 - ch10.pdf"]
---
# model-based OPC 校準與迭代

## 方法目的

model-based OPC 以校準的微影模型預測實際設計的 proximity effects，避免為每一種局部幾何建立大量經驗規則。其理由是 rule-based OPC 的規則數會隨精度要求快速增加，而全晶片校正同時需要速度與精度。

## 模型與校準

compact model 通常結合物理空中像模型與簡化經驗光阻模型：

- 使用 [[sum-of-coherent-sources]] 預先計算少數 coherent convolution kernels。
- 以 Gaussian diffusion kernel 近似部分光阻擴散效應。
- 使用 threshold plus bias、[[variable-threshold-resist-model]] 或經驗 kernels 表示光阻響應。
- 視目標加入蝕刻效應，或另用蝕刻模型。

經驗部分必須針對指定光阻製程充分校準；物理成像模型不會自動保證最終輪廓預測準確。

## 迭代程序

1. 將原始設計分割成可獨立移動的邊緣 segments。
2. 模擬光阻形狀。
3. 在量測點比較預測邊緣與目標，計算 EPE。
4. 依 EPE 估計 segment 的移動量。
5. 重新模擬，直到 EPE 小於預設容差。

來源描述典型需三至六次迭代，並非對所有設計的收斂保證。Figure 10.13 使用 PROLITH 展示此流程。

## 複雜度與驗證

較小 segment、較小 jog 及較細 design grid 可提高校正積極程度，但增加 vertices 數與光罩製作成本。

來源的常規流程主要校正 nominal focus／exposure。驗證時必須明確區分名義 EPE 目標與跨 focus／dose 的製程窗口目標，不能由前者收斂推定後者改善。來源见 [[19-optical-lithography--9-10-ch10--y2hwiy]]。