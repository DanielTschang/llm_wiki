---
type: concept
title: alternating PSM
created: 2026-10-03
updated: 2026-10-03
tags: [optical-lithography, PSM, phase-conflicts]
related: [19-optical-lithography--9-10-ch10--y2hwiy, psm, marc-levenson, attenuated-psm, 自然解析度, 關鍵形狀誤差與邊緣位置誤差]
sources: ["optical-lithography/10 - ch10.pdf"]
---
# alternating PSM

alternating PSM（Alt-PSM，亦稱 Levenson PSM）是讓相鄰透明空間具有 180° 相位差的 phase-shifting mask。

## 理想成像優勢

對來源的週期線空模型，交替相位消除零階與其他偶數階，留下位於 \(\pm1/(2p)\) 的一階光束。理想同調 TE 雙光束成像可達到最小節距 \(\lambda/(2NA)\)。

兩光束振幅相等且對稱於鏡頭中心，因此具有良好的影像斜率與離焦表現；此為理想模型結果，不表示所有實際光罩均符合。

## phase conflicts

相位指派可能出現兩種衝突：

1. 關鍵特徵兩側需要相位差，卻因相位配置衝突而沒有。
2. 不需要暗線的位置出現 0–180° 相位邊界，例如線端的 termination problem。

相位邊界本身會形成窄暗線，參見 [[自然解析度]]。雙曝光可處理部分衝突，但增加成本、降低 throughput，並可能限制圖案密度。來源對 phase-friendly layout 的未解描述屬於 2007 年觀點。

## 實際光罩與誤差

石英蝕刻區的繞射造成 spacewidth 依賴的相位及強度不平衡。dual trench、undercut 與 biased space 可減輕問題，但來源未將任何方法視為全尺寸與全節距的完美修正。

相位誤差引入原本消失的零階。與 defocus 耦合後，相鄰透明空間亮度不對稱，可能使線寬近似不變而線位置偏移。因此評估必須包含 [[關鍵形狀誤差與邊緣位置誤差]]，不能只看 CD。

來源見 [[19-optical-lithography--9-10-ch10--y2hwiy]]。