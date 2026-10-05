---
type: source
title: 梯度式微影最佳化與 NILS
authors: [Chris Mack]
year: 2007
url: ""
venue: "Fundamental Principles of Optical Lithography: The Science of Microfabrication"
created: 2026-10-03
updated: 2026-10-03
tags: [optical-lithography, NILS, PEB, LER]
related: [chris-mack, nils-與影像邊緣品質, 潛像梯度與-chemical-contrast, 線邊緣粗糙度, nils-defocus-最佳化與曝光裕度校準, 微影資訊傳遞與-cd-誤差傳播, rdpsf]
sources: ["optical-lithography/09 - ch9.pdf"]
---
# 梯度式微影最佳化與 NILS

本來源為 Chris Mack 的 *Fundamental Principles of Optical Lithography: The Science of Microfabrication*（2007）第 9 章，原章名為 *Gradient-Based Lithographic Optimization: Using the Normalized Image Log-Slope*，書內頁碼 369–409。章節以局部梯度與一階誤差傳播，連結光學影像、曝光、PEB、顯影、CD 控制與 LER。

證據主要來自解析推導、近似模型與模擬；不應將本章的示例數值當作獨立實驗驗證或通用製程規格。

## 邊緣品質與曝光裕度

CD 由圖案邊緣附近的強度轉換決定，而非亮、暗區中心的整體對比度。章節採用：

\[
ILS=\frac{d\ln I}{dx},\qquad NILS=w\frac{d\ln I}{dx},
\]

其中 \(w\) 為名義線寬，導數取於名義邊緣。比較邊緣品質時須固定座標方向，或比較梯度大小。

以 CD 對劑量的相對敏感度大小定義：

\[
S_E=\left|\frac{\partial\ln CD}{\partial\ln E}\right|.
\]

理想無限對比度光阻給出 \(S_E=2/|NILS|\)。若 CD 容許範圍為 ±10%，且該區間內 NILS 近似固定，則完整劑量容許範圍為：

\[
\%EL\approx\frac{20}{S_E}\approx10|NILS|.
\]

非理想光阻可用製程相依的經驗關係 \(\%EL\approx\alpha(NILS-\beta)\) 校準。圖 9.6 的模擬條件為 248 nm、NA = 0.6、\(\sigma=0.5\)、500 nm UV6／ARC／silicon、250 nm 線與空間，得到：

\[
\%EL=8.9(NILS-0.5).
\]

此案例的 10% 曝光裕度對應 NILS 約 1.7；它不是所有 UV6 製程或所有光阻的門檻。詳見 [[nils-defocus-最佳化與曝光裕度校準]] 與既有 [[nils-與影像邊緣品質]]。

## 曝光與潛像梯度

對一階曝光動力學且光學性質不隨曝光改變的光阻：

\[
m=e^{-CI_rt},\qquad
\frac{\partial m}{\partial x}
=m\ln(m)\frac{\partial\ln I_r}{\partial x}.
\]

\(-m\ln m\) 在 \(m=e^{-1}\approx0.37\) 最大。曝光因此不只是尺寸調整工具，也會改變潛像品質；此最適值僅適用於上述模型，不能直接視為完整化學放大型光阻的製程最適濃度。

漂白可提高膜內梯度，但章節相關推導排除駐波及膜厚方向離焦，且部分解析式限定 \(B=0\)。膜內 NILS 必須指定計算深度，通常取光阻底部，亦可依用途取中部。

## PEB 的反應與擴散權衡

對週期 \(p\) 的 Fourier 分量，令：

\[
q=2(\pi n\sigma_D/p)^2.
\]

純擴散 DPSF 與無酸損失 RDPSF 的振幅衰減分別為：

\[
\frac{a_n^*}{a_n}=e^{-q},\qquad
\frac{a_n^*}{a_n}=\frac{1-e^{-q}}{q}.
\]

RDPSF 反映去保護反應所累積的酸分布，而非僅使用 PEB 結束時的酸分布。相同擴散長度下，其高頻衰減較小；詳見 [[無酸損失-rdpsf-保留較多-fourier-振幅]]。

反應—擴散競爭由下式表徵：

\[
\eta=\frac{\pi^2D}{L^2K_{\mathrm{amp}}},\qquad
\alpha_f=K_{\mathrm{amp}}t_{\mathrm{PEB}}.
\]

短 PEB 受放大不足限制，長 PEB 受梯度平滑限制，因此簡化模型存在有限的梯度最佳 PEB。由於 \(\eta\propto L^{-2}\)，特徵縮小時須改善 \(D/K_{\mathrm{amp}}\)，才能維持相同競爭程度。章節對 \(\eta\approx0.1\text{–}0.2\) 提出的 \(\alpha_f\approx3\)、\(m^*\approx0.45\)、LIG/ILS 約 0.25，均屬模型結果，未提供獨立產品驗證。

含 quencher 的解析近似另要求酸—鹼反應遠快於擴散，且酸與 quencher 的擴散係數完全相同。可先對帶正負號的淨酸濃度作卷積，再於放大計算前將負有效酸濃度截為零；不應將此方法推廣到不等擴散係數情況。

## 顯影與 CD 誤差傳播

局部光阻對比度為：

\[
\gamma=\frac{\partial\ln r}{\partial\ln E},
\qquad
\frac{\partial\ln r}{\partial x}
=\gamma\frac{\partial\ln I}{\partial x}.
\]

此 \(\gamma\) 包含曝光、PEB 與顯影的整體化學響應，且通常隨劑量而變；它不應與其他量測對比度定義直接混用。

Lump鄰ed Parameter Model 的分段顯影路徑與 Gaussian 影像近似給出，採章節的正向座標慣例：

\[
\frac{\partial\ln CD}{\partial\ln E}
\approx
\frac{2}{NILS}
+\frac{2\gamma D_{\mathrm{eff}}}{CD}
\left[\frac{I(CD/2)}{I(0)}\right]^\gamma.
\]

第一項為理想光學極限，第二項為顯影路徑修正。此式要求近似固定對比度及特定影像與路徑假設，不是任意三維輪廓的精確關係。相關背景見 [[lumped-parameter-model]]、[[original-mack-model]] 與 [[光阻理論對比度與量測對比度]]。

## LER 與最佳化目標的差異

無限對比度閾值顯影與一階展開給出：

\[
LER\propto\frac{\sigma_{m^*}}{|dm^*/dx|}.
\]

酸擴散可降低濃度波動，也會降低潛像梯度，因此最小 LER 不必發生於最大梯度的製程條件。圖 9.28 是 45 nm 特徵與不同 reaction capture range 的相對趨勢預測，縱軸為任意單位；其簡化過程忽略 quencher，部分步驟亦忽略 photon shot noise。詳見 [[線邊緣粗糙度]]。

## 原始結構化資料

以下保留 Table 9.1 的原文欄位與內容，僅合併跨行文字。

| Process Step | Information | Error Sources | Information Metric |
|---|---|---|---|
| Design | Polygons, binary | (usually assumed perfect) | |
| Mask | Amplitude transmittance, tm(x,y) | CD and registration errors, corner rounding, phase and transmittance | |
| Aerial Image | I(x,y) | Diffraction limitation, aberrations, defocus, flare, polarization | NILS |
| Image in Resist | I(x,y,z) | Substrate reflections/thin film effects, polarization effects, defocus through the resist | NILS |
| Exposure | Latent Image m(x,y,z) or h(x,y,z) (before PEB) | Exposure dose errors | Latent image gradient |
| Post-exposure Bake | Latent Image m*(x,y,z) (after PEB) | Thermal dose errors, diffusion | Latent image gradient |
| Development | Development Rate r(x,y,z) + Resist Profile (CD, sidewall angle, resist loss) | Finite contrast, rmax/rmin | Development rate log-slope, gamma + exposure latitude, CD error |

## 限制與資料品質

- 梯度法不能捕捉 isofocal bias 對焦深的影響，仍須以完整 focus–exposure matrix 驗證。
- OCR 的部分長度單位出現 mm，與圖軸 microns 或圖說 nm 不一致；未核對原 PDF 前，不採錄那些數值作製程設定。
- 本頁以 \(\eta\) 表示反應—擴散競爭參數，避免與酸濃度 \(h\) 混淆。
- 依 \(m^*=e^{-\alpha_fh_{\mathrm{eff}}}\)，\(m^*\) 為剩餘 blocked polymer 濃度；不沿用部分圖說的 deblocked concentration 標籤。
- 章節的 LER 常見值與節點預測屬 2007 年背景，不列為今日規格。