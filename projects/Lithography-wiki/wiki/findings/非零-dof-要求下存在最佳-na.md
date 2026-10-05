---
type: finding
title: 非零 DOF 要求下存在最佳 NA
source: "[[19-optical-lithography--9-10-ch10--y2hwiy]]"
confidence: medium
replicated: null
created: 2026-10-03
updated: 2026-10-03
tags: [NA, DOF, resolution]
related: [19-optical-lithography--9-10-ch10--y2hwiy, 數值孔徑與解析度, 離焦與焦深, ret]
sources: ["optical-lithography/10 - ch10.pdf"]
---
# 非零 DOF 要求下存在最佳 NA

## 直接證據

來源 Figure 10.4 顯示等線空圖案在 \(\lambda=193\) nm、\(\sigma=0.7\) 下，不同最低 DOF 要求對應不同的解析度–NA 曲線。

要求 DOF = 200 nm 時，曲線最低點約為 NA = 0.84，對應特徵尺寸約 105 nm。繼續提高 NA，最小符合規格的特徵尺寸反而增加。

來源所用輪廓規格為 CD ±10%、側壁角 >80°、光阻損失 <10%，曝光裕度為 6%；各點調整 nominal focus／exposure 以取得最佳製程窗口，未納入 mask linearity 約束。

## 推論與適用範圍

此圖例支持：提高 NA 可改善光學節距截止，但不一定改善非零 DOF 要求下的特徵解析度。不能將 NA = 0.84 當作普遍最佳值。

比較解析度時，必須同時記錄圖案種類、DOF、曝光裕度與輪廓規格。參見 [[數值孔徑與解析度]]。

## 重現狀態

未提供獨立重現資料，故 `replicated` 為 null。來源圖例支持趨勢，但不足以建立跨製程的數值最佳值。