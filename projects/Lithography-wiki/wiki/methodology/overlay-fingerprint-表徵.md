---
type: methodology
title: overlay fingerprint 表徵
created: 2026-10-03
updated: 2026-10-03
tags: [overlay, 量測, 工具匹配]
related: [overlay-控制, 鏡頭像差與-zernike-polynomial, 19-optical-lithography--8-08-ch8--1az2fh0]
sources: ["optical-lithography/08 - ch8.pdf"]
---
# overlay fingerprint 表徵

overlay fingerprint 表徵密集量測相對穩定的工具空間位置誤差，以免低階模型將固定高階差異誤判為可修正項或隨機殘差。

## 相對 lens fingerprint

選定 reference tool，製作第一層標記已蝕刻的 artifact wafer。以密集場內標靶在各工具印製第二層，移除低階 correctables，並平均多個場。

所得 fingerprint 是相對 reference tool 的差異，不是絕對 registration。若資料庫含 D−A 與 G−A，D 印第一層、G 印第二層的相對差異為：

\[
(G-A)-(D-A)=G-D.
\]

可用於模型前扣除或工具配對排序。

## 來源混淆與動態分離

不同 reticle 的固定 registration 差異可能混入 lens fingerprint。來源建議以同一 reticle 上相鄰的兩層標靶降低此問題。

靜態曝光表徵 lens；沿 scan 方向平均形成 slit signature。完整掃描曝光得到 lens／scan 合成特徵，再扣除 slit signature 以估計 stage scan signature。

lens、reticle 與 stage 不能視為同一 fingerprint；stage 的時間變化通常較快，表徵更新頻率亦應不同。

來源：[[19-optical-lithography--8-08-ch8--1az2fh0]]，§8.4.3。