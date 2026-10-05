---
type: concept
title: SRAF
created: 2026-10-03
updated: 2026-10-03
tags: [optical-lithography, SRAF, process-window]
related: [19-optical-lithography--9-10-ch10--y2hwiy, opc, oai, sraf-提高示例重疊-dof, 焦距與曝光製程窗口]
sources: ["optical-lithography/10 - ch10.pdf"]
---
# SRAF

Subresolution Assist Features（SRAFs）是配置在主特徵附近、不應列印的窄線或透明槽，用來改變主特徵的成像及製程響應。輔助線亦稱 scattering bars；暗場圖案可使用透明 assist slots。

## 改善機制

孤立線與密集線即使經 [[opc]] 校正至相同名義 CD，其 Bossung 曲線仍可能不同。SRAF 讓孤立主特徵在光學上更接近密集圖案，因此可取得針對密集節距最佳化的 [[oai]] 所提供的部分 DOF 收益。

來源 Figure 10.15 中，50 nm 輔助線將重疊 DOF 從 300 nm 增至 400 nm，詳見 [[sraf-提高示例重疊-dof]]。

## 設計權衡

輔助特徵通常越大越有效，但必須在整個要求的製程窗口內不列印。位置與尺寸應以共同製程窗口改善為目標，而非只以最佳焦點 CD 為目標。

double scattering bars 可進一步提高有效密集程度，但需要更多空間。intermediate pitches 可能沒有足夠空間插入 SRAF，留下低裕度區間。二維轉角與線端配置也比理想孤立線更複雜。

EPSM 的 assist slots 還可藉相消干涉抑制 sidelobes；不能將所有輔助槽作用都描述為增加局部亮度。來源見 [[19-optical-lithography--9-10-ch10--y2hwiy]]。