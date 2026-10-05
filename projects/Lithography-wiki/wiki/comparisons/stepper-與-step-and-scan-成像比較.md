---
type: comparison
title: stepper 與 step-and-scan 成像比較
created: 2026-10-03
updated: 2026-10-03
tags: [掃描曝光, CD, 動態誤差]
related: [19-optical-lithography--8-03-ch3--s477p8, 離焦與焦深, flare, 鏡頭像差與-zernike-polynomial]
sources: ["optical-lithography/03 - ch3.pdf"]
---
# stepper 與 step-and-scan 成像比較

此比較依據 [[19-optical-lithography--8-03-ch3--s477p8]] 第 3.5 節的機制分析，不是跨設備的受控性能試驗。

| 面向 | stepper | step-and-scan |
|---|---|---|
| 曝光方式 | 光罩與晶圓靜止曝光整個場，再步進 | 光罩與晶圓同步掃過狹縫，再步進 |
| 鏡頭設計 | 需涵蓋整個瞬時曝光場 | 較小狹縫降低鏡頭場設計負擔 |
| 場依賴誤差 | 保留各場位置的局部成像差異 | 平均掃描方向的像差及 flare 差異 |
| 平均限制 | 無掃描平均 | 狹縫長軸方向不具有同樣平均 |
| 動態誤差 | 曝光期間振動等 | 另需控制同步與掃描焦距誤差 |

## 動態模型

隨機同步誤差可以 moving standard deviation（MSD）表示；在 Gaussian 誤差假設下，平均影像是靜態影像與相應 Gaussian 的卷積。掃描焦距誤差則產生 focus averaging。

掃描速度為
\[
V=\frac{W_sf}{n},
\]
其中 \(W_s\) 是狹縫寬度、\(f\) 是雷射重複頻率、\(n\) 是曝光所需脈衝數。本章 8 mm、2 kHz、50 pulses 的例子得到 0.32 m/s。

## 結論邊界

掃描平均可改善 CD 與位置均勻性，但動態誤差可能抵銷其收益；不能一概宣稱所有 scanner 優於所有 stepper。

來源擷取稱劑量重複性改善因子為 `n`。独立隨機脈衝的標準差通常依 \(1/\sqrt n\) 降低，此處可能遺失根號，須核對原頁。