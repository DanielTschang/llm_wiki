---
type: concept
title: radiometric correction
created: 2026-10-03
updated: 2026-10-03
tags: [縮小投影, 能量守恆]
related: [fourier-optics, 19-optical-lithography--8-02-ch2--k73j1g]
sources: ["optical-lithography/02 - ch2.pdf"]
---
# radiometric correction

radiometric correction 是縮小或放大投影中，為符合能量守恆而加入的角度相關瞳孔電場振幅修正。

## 縮小投影關係

縮小比 \(R\) 的 Abbe sine condition 為：

\[
n_w\sin\theta_w=R\,n_m\sin\theta_m.
\]

橫向尺寸在晶圓側縮小為遮罩側的 \(1/R\)，晶圓側空間頻率則放大 \(R\) 倍。無損鏡頭模型的角度相關振幅因子為：

\[
\sqrt{\frac{\cos\theta_m}{\cos\theta_w}}.
\]

## 影像與正規化

修正會提高部分高頻分量的相對權重，某些遮罩及照明配置可能得到改善的影像形狀；這不是普遍製程改善保證。

遮罩側與晶圓側 open-field 正規化可能不同，應交代所用基準。[[19-optical-lithography--8-02-ch2--k73j1g]] 對劑量感測器位置及常見縮小倍率的敘述屬於出版時背景。