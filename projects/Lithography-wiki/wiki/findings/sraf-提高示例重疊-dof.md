---
type: finding
title: SRAF 提高示例重疊 DOF
source: "[[19-optical-lithography--9-10-ch10--y2hwiy]]"
confidence: medium
replicated: null
created: 2026-10-03
updated: 2026-10-03
tags: [SRAF, DOF, simulation]
related: [19-optical-lithography--9-10-ch10--y2hwiy, sraf, oai, 焦距與曝光製程窗口]
sources: ["optical-lithography/10 - ch10.pdf"]
---
# SRAF 提高示例重疊 DOF

## 直接證據

來源 Figures 10.14–10.15 比較密集與孤立 130-nm 特徵。條件為 \(\lambda=248\) nm、NA = 0.85，quadrupole illumination 針對 260-nm 節距最佳化。

孤立線先經 bias OPC，使其在密集線的最佳焦距及曝光下具有正確 CD；僅使用此校正時，重疊 DOF 為 300 nm。加入晶圓尺度 50 nm 的 assist bars（\(k_1=0.17\)）後，重疊 DOF 增至 400 nm。

## 推論與適用範圍

依來源數值計算，增加幅度約為 33%。圖例支持 [[sraf]] 可改善孤立／密集圖案的共同焦距裕度，而非只校正名義 CD。

此结果不保證其他 source shape、光阻、節距或二維幾何也有相同改善。SRAF 必須在要求的製程窗口內不列印。

## 重現狀態

提供內容沒有獨立重現資料，故 `replicated` 為 null。此為來源圖例結果，不標示為已重現實驗。