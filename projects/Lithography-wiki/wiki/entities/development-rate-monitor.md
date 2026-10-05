---
type: entity
title: development rate monitor
created: 2026-10-03
updated: 2026-10-03
tags: [metrology, development, photoresist]
related: [19-optical-lithography--8-07-ch7--dxj4cs, drm-顯影參數萃取, practical-contrast-量測]
sources: ["optical-lithography/07 - ch7.pdf"]
---
# development rate monitor

development rate monitor（DRM）是用於即時、原位量測光阻顯影期間膜厚變化的量測工具類別，而非本章指定的品牌或產品。

對多個曝光劑量取得膜厚–時間曲線後，可藉由微分計算不同深度的溶解速率。[[drm-顯影參數萃取]] 再結合曝光模型，將速率資料轉為抑制劑濃度相關的動力學參數。

## 量測邊界

膜厚是直接觀測量；速率是經微分處理的量；抑制劑濃度與模型參數則另依賴曝光與顯影模型。DRM 因而不能單獨證明某個微觀溶解機制。

來源將 DRM 視為萃取新參數的最佳方法，並介紹以多組離線膜厚曲線替代即時量測的 Poor Man’s DRM。參見 [[19-optical-lithography--8-07-ch7--dxj4cs]] §7.4。