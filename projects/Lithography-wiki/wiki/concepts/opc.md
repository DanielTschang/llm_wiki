---
type: concept
title: OPC
created: 2026-10-03
updated: 2026-10-03
tags: [optical-lithography, OPC, EPE]
related: [19-optical-lithography--9-10-ch10--y2hwiy, ret, sraf, model-based-opc-校準與迭代, rule-based-與-model-based-opc-比較, 線端縮短, 光阻理論對比度與量測對比度]
sources: ["optical-lithography/10 - ch10.pdf"]
---
# OPC

Optical Proximity Correction（OPC）是修改光罩特徵的形狀與邊緣位置，使預測或量測的晶圓圖案接近設計目標的技術。

## 校正對象

proximity effects 指特徵 CD 或形狀隨鄰近圖案改變，包括 iso-dense print bias、轉角圓化與 [[線端縮短]]。雖稱 optical proximity effects，其來源也可能包含 PEB diffusion 或顯影效應。

iso-dense print bias 是孤立線與密集線的列印尺寸差，但整段 CD-through-pitch 曲線的尺寸變動可能比此差值更大。曲線及偏差正負號依賴照明、像差、特徵尺寸與光阻。

## 實作方式

- **rule-based OPC**：使用量測建立的規則表，依局部線寬與鄰近距離移動邊緣。
- **model-based OPC**：使用校準的微影模型及 EPE 迭代決定邊緣移動。

參見 [[rule-based-與-model-based-opc-比較]] 及 [[model-based-opc-校準與迭代]]。

## 限制

名義焦距與曝光下的尺寸或 EPE 正確，不保證 through-focus 響應一致，也不保證共同製程窗口增加。[[sraf]] 可補足單純 bias OPC 未處理的孤立／密集圖案焦距響應差異。

來源 Figure 10.8 的較高光阻對比度案例有較大的 iso-dense bias，不能把提高 [[光阻理論對比度與量測對比度|光阻對比度]] 當作降低所有 proximity effects 的通則。來源見 [[19-optical-lithography--9-10-ch10--y2hwiy]]。