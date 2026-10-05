---
type: source
title: 光阻顯影動力學、對比度與顯影路徑
authors: [Chris Mack]
year: 2007
url: ""
venue: "Fundamental Principles of Optical Lithography: The Science of Microfabrication"
created: 2026-10-03
updated: 2026-10-03
tags: [optical-lithography, photoresist, development]
related: [chris-mack, original-mack-model, enhanced-kinetic-model, notch-model, critical-ionization-model, 表面抑制, 光阻理論對比度與量測對比度, 顯影路徑與最短時間原理, lumped-parameter-model, variable-threshold-resist-model, thmr-ip3650, development-rate-monitor, drm-顯影參數萃取, practical-contrast-量測, apex-e-在-026-n-下的選擇性溫度峰值]
sources: ["optical-lithography/07 - ch7.pdf"]
---
# 光阻顯影動力學、對比度與顯影路徑

本來源為 Chris Mack 的 *Fundamental Principles of Optical Lithography: The Science of Microfabrication* 第 7 章〈Photoresist Development〉，書頁 257–296，由 John Wiley & Sons, Ltd. 於 2007 年出版，ISBN: 978-0-470-01893-4。它說明曝光與烘烤形成的化學潛像，如何經由溶解速率及顯影前緣傳播轉換為最終光阻輪廓與 CD。

本章是教材式綜述：包含機制假設、數學推導與引用實驗資料。模型擬合成功不等於微觀機制已獲驗證；作者明確指出，基礎機制實驗仍不足，聚合物鏈長與官能基分布也使宏觀參數只能近似對應微觀事件。

## 動力學與材料系統

顯影包含顯影劑輸送至界面、界面反應及溶解產物移除。本章假定最後一步很快而忽略，並以 [[original-mack-model]] 描述前兩步串聯：

\[
r=r_{\max}\frac{(a+1)(1-m)^n}{a+(1-m)^n}+r_{\min},
\qquad
a=\frac{k_D}{k_RM_0^n}
=\frac{n+1}{n-1}(1-m_{\mathrm{th}})^n.
\]

\(m\) 是剩餘抑制劑的相對濃度；\(n\) 是溶解選擇性；\(m_{\mathrm{th}}\) 是曲線反曲點。加入 \(r_{\min}\) 後，此模型真正的最大速率為 \(r_{\max}+r_{\min}\)。

[[enhanced-kinetic-model]] 同時描述溶解抑制與增強，原式忽略顯影劑質量輸送限制；其 \(r_{\max}\) 直接代表完全反應的最大速率，不能與原模型同名參數直接互換。[[notch-model]] 是補足局部急劇速率下降的半經驗式模型。[[critical-ionization-model]] 則假設足夠比例的受保護基團去保護後，聚合物才變得可溶。

傳統 [[dnq]]／[[novolac]] 系統以 DNQ 抑制溶解，曝光產物增強溶解；[[化學放大型光阻]] 中的受保護 [[polyhydroxystyrene]] 則經 [[聚合物去保護反應]] 產生可溶的 –OH 位點。這些化學分布是顯影模型的輸入，不應與 [[peb-擴散與-dpsf]] 或 [[rdpsf]] 的烘烤反應與擴散模型混同。

作者稱約半數已表徵光阻適用原模型，多數其餘適用 enhanced kinetic model，少數需要 notch model；本章未提供樣本總數或系統性模型比較資料。

## 溫度、濃度與表面效應

[[thmr-ip3650]] 的高劑量速率隨顯影液升溫而增加，低劑量時則可能降低；擬合的 \(r_{\max}\) 與 \(n\) 均隨溫度提高。兩者 Arrhenius 活化能分別為 7.41 與 7.02 Kcal/mole。

[[apex-e]] 不呈現相同的單調趨勢：在表 7.1 的 0.26 N 條件下，35 °C 的 \(n\) 為表中最高值。溫度效應必須保持產品與劑量歸屬。

本章指出 [[tmah]] 的 0.26 N、2.38 % by weight 條件廣泛使用。較高 normality 通常提高溶解速率，但不保證提高選擇性。文中所述約 0.1–0.15 N 以下不再顯影的臨界區間，不是表 7.2 直接量得的結果；該表最低濃度為 0.195 N。

puddle development 的 developer loading 包含溶解物累積及 –OH 消耗，可能降低局部速率；圖案密度差異因此可能造成 CD 不均。double puddle 以更新顯影液減輕此問題。

[[表面抑制]] 可來自氧化、表面溶劑耗失、界面活性劑，或化學放大型光阻的表面酸損失與鹼污染；參見 [[酸損失與曝光後延遲]]。適量抑制可能改善頂部輪廓，過量則可能造成 T-top 與 linewidth 控制劣化。

## 對比度與輪廓

[[光阻理論對比度與量測對比度]] 區分材料速率響應與剩餘膜厚量測：

\[
\gamma_{\mathrm{th}}=\frac{d\ln r}{d\ln E},
\qquad
\partial_x\ln r=\gamma_{\mathrm{th}}\partial_x\ln I.
\]

第二式為 Lithographic Imaging Equation，連結 [[nils-與影像邊緣品質]] 與顯影速率梯度。本章統一使用自然對數。吸收可壓低傳統量測對比度，表面抑制則可抬高它；高量測值不必然意味微影性能改善。

在 log-dose 上的 Gaussian 對比度假設下：

\[
\ln(r_{\max}/r_{\min})
\approx1.06\,\gamma_{\max}\ln(R_{\mathrm{FWHM}}).
\]

因此高對比度還需要足夠的速率動態範圍；這是條件式推導，不是所有光阻配方的普遍門檻。

[[顯影路徑與最短時間原理]] 以到達時間決定輪廓：

\[
t_{\mathrm{dev}}=\int_{\mathrm{path}}\frac{ds}{r(x,y,z)},
\qquad
|\nabla t|^2=\frac{1}{r^2}.
\]

輪廓是等到達時間面，不是逐點套用固定閾值的結果。[[lumped-parameter-model]] 以固定對比度及先垂直後水平的分段路徑降階；[[variable-threshold-resist-model]] 再忽略垂直時間，令閾值隨 image log-slope 改變。

圖 7.20 的 Gaussian 範例使用 \(g=0.0025\ \mathrm{nm}^{-2}\)、\(NILS_1=2.5\)、\(CD_1=100\ \mathrm{nm}\)、\(\gamma=10\)、\(D_{\mathrm{eff}}=200\ \mathrm{nm}\)。近似 LPM 的劑量誤差約 1%；VTR 在 nominal CD 約 2%，CD 偏小 10%、20% 時約 3.5%、6.5%。這些是相對完整 LPM 的模型間差異，不是對實驗真值的誤差。

## 原始表格資料

### 表 7.1：Apex-E 的溫度依賴模型參數，0.26 N 顯影液

| Developer Temperature (°C) | rmax (nm/s) | rmin (nm/s) | mth | n |
|---|---|---|---|---|
| 5 | 53.1 ± 9.0 | 1.18 | 0.571 ± 0.036 | 3.93 ± 0.56 |
| 10 | 68.0 ± 10.9 | 1.283 | 0.578 ± 0.031 | 4.17 ± 0.52 |
| 15 | 91.9 ± 13.8 | 1.35 | 0.586 ± 0.028 | 4.25 ± 0.48 |
| 20 | 115.7 ± 14.1 | 1.48 | 0.682 ± 0.016 | 4.79 ± 0.49 |
| 25 | 146.1 ± 15.2 | 1.55 | 0.637 ± 0.015 | 5.56 ± 0.54 |
| 30 | 177.8 ± 16.0 | 1.62 | 0.646 ± 0.012 | 6.36 ± 0.55 |
| 35 | 199.7 ± 14.6 | 2.56 | 0.693 ± 0.008 | 15.58 ± 2.17 |
| 40 | 196.2 ± 13.1 | 2.14 | 0.694 ± 0.008 | 11.25 ± 1.32 |
| 45 | 209.5 ± 22.5 | 1.83 | 0.665 ± 0.014 | 6.64 ± 0.74 |

### 表 7.2：Apex-E 的溫度與 normality 依賴模型參數

| Developer Temperature (°C) | Developer Normality | rmax (nm/s) | rmin (nm/s) | mth | n |
|---|---|---|---|---|---|
| 20 | 0.195 | 45.9 ± 7.9 | 0.272 | 0.527 ± 0.03 | 5.26 ± 1.00 |
| 20 | 0.24425 | 90.9 ± 11.3 | 0.912 | 0.607 ± 0.018 | 5.41 ± 0.61 |
| 20 | 0.26 | 115.7 ± 14.1 | 1.482 | 0.682 ± 0.016 | 4.79 ± 0.49 |
| 35 | 0.195 | 78.1 ± 12.9 | 0.395 | 0.533 ± 0.01 | 17.33 ± 2.18 |
| 35 | 0.24425 | 151.3 ± 13.1 | 1.191 | 0.667 ± 0.008 | 11.10 ± 1.14 |
| 35 | 0.26 | 199.6 ± 14.6 | 2.562 | 0.693 ± 0.008 | 15.58 ± 2.17 |
| 40 | 0.195 | 118.5 ± 94.6 | 0.395 | 0.511 ± 0.074 | 8.72 ± 1.93 |
| 40 | 0.24425 | 182.9 ± 12.4 | 1.473 | 0.675 ± 0.005 | 16.44 ± 1.62 |
| 40 | 0.26 | 196.2 ± 13.1 | 2.135 | 0.694 ± 0.008 | 11.25 ± 1.32 |

兩表同條件的部分數字不同，依來源保留原值，不自行統一。

### 圖 7.22：未具名 i-line 光阻的擬合標註

```text
RMS Error: 2.7 nm/s
rmax = 100.3 nm/s
rmin = 0.10 nm/s
mth = 0.06
n = 4.74
```

此組資料來自 Poor Man’s DRM，不能歸屬給 THMR-iP3650 或 Apex-E。

## 量測與證據限制

[[development-rate-monitor]] 直接量測顯影中的膜厚；[[drm-顯影參數萃取]] 從膜厚微分取得 \(r(E,z)\)，再結合曝光模型推得 \(r(m,z)\)。因此抑制劑濃度與動力學參數包含模型推論，並非儀器直接讀值。[[practical-contrast-量測]] 可由清除時間估計對比度，但需要速率的劑量與深度依賴可分離。

來源擷取的 Gaussian ILS／NILS 式存在正負號疑點。對 \(I(x)=I_0e^{-x^2/(2\sigma^2)}\)，有號導數為 \(-x/\sigma^2\)；本次相關近似以斜率大小表述，不將擷取的正值逕認為有號導數。

## 主要追溯文獻

- Mack, C.A., 1987, Development of positive photoresist, Journal of the Electrochemical Society, 134, 148–152.
- Mack, C.A., 1992, New kinetic model for resist dissolution, Journal of the Electrochemical Society, 139, L35–L37.
- Arthur, G., Mack, C.A. and Martin, B., 1997, Enhancing the development rate model for optimum simulation capability in the sub-half-micron regime, Proceedings of SPIE: Advances in Resist Technology and Processing XIV, 3049, 189–200.
- Tsiartas, P.C., Flanagin, L.W., Henderson, C.L., Hinsberg, W.D., Sanchez, I.C., Bonnecaze, R.T. and Willson, C.G., 1997, The mechanism of phenolic polymer dissolution: a new perspective, Macromolecules, 30, 4656–4664.
- Mack, C.A., Maslow, M.J., Carpio, R. and Sekiguchi, A., 1998, New model for the effect of developer temperature on photoresist dissolution, Proceedings of SPIE: Advances in Resist Technology and Processing XV, 3333, 1218–1231.
- Maslow, M.J., Mack, C.A. and Byers, J., 1999, Effect of developer temperature and normality on chemically amplified photoresist dissolution, Proceedings of SPIE: Advances in Resist Technology and Processing XVI, 3678, 1001–1011.
- Thornton, S.H. and Mack, C.A., 1996, Lithography model tuning: matching simulation to experiment, Proceedings of SPIE: Optical Microlithography IX, 2726, 223–235.