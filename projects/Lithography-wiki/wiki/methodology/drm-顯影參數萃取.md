---
type: methodology
title: DRM 顯影參數萃取
created: 2026-10-03
updated: 2026-10-03
tags: [metrology, development, model-calibration]
related: [19-optical-lithography--8-07-ch7--dxj4cs, development-rate-monitor, dill-abc-參數, original-mack-model, 表面抑制]
sources: ["optical-lithography/07 - ch7.pdf"]
---
# DRM 顯影參數萃取

本方法利用 [[development-rate-monitor]] 的膜厚–時間資料，分離劑量與深度對速率的影響，再校準顯影模型。其目的不是只取得清除劑量，而是建立可預測輪廓的速率函數。

## 流程與推論層次

1. 在多個入射曝光劑量下，即時量測膜厚隨顯影時間的變化。
2. 將膜厚資料微分，取得 \(r(E,z)\)；這一步需控制雜訊與微分處理誤差。
3. 以曝光模型計算 \(m(E,z)\)，傳統光阻可使用 [[dill-abc-參數]] 所描述的曝光模型。
4. 組合成 \(r(m,z)\)，擬合 [[original-mack-model]] 或其他適合模型，必要時加入 [[表面抑制]]。

直接證據是膜厚變化；速率是處理結果；\(m\) 與最終模型參數是模型依賴推論。不同模型的良好擬合不能自行證明唯一微觀機制。

## Poor Man’s DRM

沒有即時設備時，可在多個顯影時間下量測 open-frame 剩餘膜厚–劑量曲線，再轉成 \(r(E,z)\)，其後沿用相同分析。此方法提供近似替代資料，但重建速率並不簡單。

來源圖 7.22 的未具名 i-line 光阻擬合值保存在 [[19-optical-lithography--8-07-ch7--dxj4cs]]，不應歸屬給本章其他具名產品。

來源定位：§7.4。