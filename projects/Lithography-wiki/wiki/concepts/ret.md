---
type: concept
title: RET
created: 2026-10-03
updated: 2026-10-03
tags: [optical-lithography, RET, process-window]
related: [19-optical-lithography--9-10-ch10--y2hwiy, opc, oai, psm, sraf, 自然解析度, 數值孔徑與解析度, 焦距與曝光製程窗口]
sources: ["optical-lithography/10 - ch10.pdf"]
---
# RET

Resolution Enhancement Technologies（RET）是透過光罩形狀、照明及透射相位的調整，改善特定微影圖案解析度與製程窗口的技術集合，核心包括 [[opc]]、[[oai]] 與 [[psm]]。

## 波前操控與圖案選擇性

wavefront engineering 的基本思路是：晶片圖案類型與方向有限，因此可針對這些圖案設計特殊成像系統。操控方式包括修改光罩、修改照明及使用 pupil filter；來源指出投影鏡頭的 pupil 不易直接存取，因此光罩與照明修改更具實用性。

此收益具有圖案選擇性，不存在對所有圖案都最佳的照明方向或光罩處理。

## 評估準則

- 分別記錄特徵解析度及節距解析度，不能把固定節距下的線寬縮小稱為節距改善。
- 以指定曝光裕度、DOF 與光阻輪廓規格判定可製造解析度。
- 比較多種圖案的共同製程窗口，而非只比較單一圖案的最佳焦點。
- 同時檢查主特徵、SRAF 誤印、sidelobes 及位置誤差。

理想雙光束成像可使最小節距達到 \(\lambda/(2NA)\)，但實際製造仍受光阻與製程規格限制。參見 [[焦距與曝光製程窗口]] 與 [[自然解析度]]。

## 來源界線

本章的產業採用敘述以 2007 年為時間界線；其理想解析模型也不構成現代製程的直接效能基準。來源見 [[19-optical-lithography--9-10-ch10--y2hwiy]]。