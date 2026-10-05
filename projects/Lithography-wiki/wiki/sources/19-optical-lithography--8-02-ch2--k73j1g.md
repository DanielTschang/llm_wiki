---
type: source
title: 空中像形成的基礎
authors: [Chris Mack]
year: 2007
url: ""
venue: "John Wiley & Sons, Ltd."
created: 2026-10-03
updated: 2026-10-03
tags: [光學微影, 成像理論, 部分同調]
related: [chris-mack, 光學微影, fourier-optics, 空間同調與部分同調, 數值孔徑與解析度, 點擴散函數, radiometric-correction, abbe-成像計算法, hopkins-成像計算法, sum-of-coherent-sources, kintner-成像計算法, köhler-illumination, prolith, 19-optical-lithography--8-01-ch1--cmoy1o]
sources: ["optical-lithography/02 - ch2.pdf"]
---
# 空中像形成的基礎

本來源為 Chris Mack 的 *Fundamental Principles of Optical Lithography: The Science of Microfabrication* 第 2 章，*Aerial Image Formation – The Basics*，書頁 29–74。它以電磁場、繞射與 Fourier optics 建立理想投影成像模型，再加入部分同調照明及其計算方法。這是理論教科書章節，不是製程實驗或軟體效能基準。

前章 [[19-optical-lithography--8-01-ch1--cmoy1o]] 著重製程整合；本章則說明 [[光學微影]] 可傳遞至晶圓的光學資訊及其限制。

## 核心模型與適用條件

在標量、理想鏡頭及適當遮罩近似下：

\[
T_m=\mathcal F\{E_i t_m\},\qquad
E=\mathcal F^{-1}\{T_mP\},\qquad
I=|E|^2.
\]

遮罩產生繞射頻譜，有限瞳孔只保留部分頻譜，鏡頭再重建光場。繞射極限表示理想鏡頭仍受有限孔徑限制，不代表影像完美。詳見 [[fourier-optics]]。

空中像是晶圓平面在空氣中的光強分布，不是光阻薄膜內的完整光場，也不是顯影或蝕刻後圖案。因此不能直接將它當作 [[圖案轉移]]、[[蝕刻選擇比與光阻輪廓]] 或 [[化學放大型光阻]] 動力學的證據。

Kirchhoff boundary condition 以材料局部透射率近似遮罩，忽略立體結構繞射。本章提出遮罩特徵大於約 \(2\lambda\)、起伏小於約 \(\lambda/2\) 的經驗適用條件；這不是普遍誤差保證。高 NA、非平行偏振場及真實遮罩結構可能需要向量處理。

## 解析度必須附帶條件

對理想等寬線／空間圖案：

- 正入射同調照明的最低節距為 \(\lambda/NA\)，半節距為 \(0.5\lambda/NA\)。
- 極端斜入射使零級與一個一級繞射位於瞳孔相對邊緣時，半節距可達 \(0.25\lambda/NA\)。
- 至少兩個繞射級次才能產生空間調變。
- \(R=k_1\lambda/NA\) 是尺度關係，不能作為所有圖案的統一硬截止。

傳統非相移 chrome-on-glass 接觸孔以單位峰值強度約 0.25–0.3 的寬度判準，得到約 \(0.66–0.70\lambda/NA\)。這是具遮罩與強度判準條件的 PSF 結果，不是量產製程的絕對極限。詳見 [[數值孔徑與解析度]]、[[點擴散函數]]。

## 同調性與計算法

同調成像對遮罩電場透射率線性；非同調成像對遮罩強度透射率線性；部分同調一般不能用其中任一單一線性關係描述。

模型假定每個光源點內的繞射波可以干涉，不同光源點互不相干，因此跨光源點加總的是強度：

- [[abbe-成像計算法]]：逐光源點計算同調影像，再積分強度。
- [[hopkins-成像計算法]]：先積分光源，建立不依賴遮罩的 TCC。
- [[sum-of-coherent-sources]]：分解光學核，以有限個同調分量近似影像。
- [[kintner-成像計算法]]：對小節距線／空間圖案，以幾何重疊計算一、二、三束成像權重。

本章稱適當 SOCS 分解可能減少約一個數量級的項數，但未提供硬體、執行時間、測試集或誤差容忍值，不能寫成普遍加速保證。

[[köhler-illumination]] 將光源成像於物鏡入口瞳孔，使不同視場位置具有一致的繞射頻譜位置。離軸照明則減少近軸來源分量，以環形、四極或雙極來源改變級次收集；改善與否仍依賴圖案及光學配置。

## 吸收、能流與縮小投影

本章採用 \(e^{-i\omega t}\) 時間因子及 \(\tilde n=n+i\kappa\)，均勻介質中的吸收為：

\[
I(z)=I_0e^{-\alpha z},\qquad
\alpha=\frac{4\pi\kappa}{\lambda}.
\]

光強源自時間平均 Poynting vector；不能無條件等同於電場平方。上述吸收關係可銜接 [[駐波與-barc]]，但本章尚未推導多層薄膜駐波或 BARC 效果。

縮小比 \(R\) 滿足 Abbe sine condition：

\[
n_w\sin\theta_w=R\,n_m\sin\theta_m.
\]

無損鏡頭的角度相關電場修正為
\(\sqrt{\cos\theta_m/\cos\theta_w}\)。[[radiometric-correction]] 可改變高頻權重、影像形狀及正規化，並非只有尺寸換算。

## 結構化資料

以下保留表格的原始欄名與項目；數學排版依可辨讀轉錄整理，並非已逐式核對 PDF 的版本。原表的圖形欄無法由文字重建。

### Table 2.1：Fourier transform 對應

| g(x) | Graph of g(x) | G(fx) |
|---|---|---|
| `rect(x) = 1, |x| < 0.5; 0, |x| > 0.5` | 原表圖形未轉錄 | \(\sin(\pi f_x)/(\pi f_x)\) |
| `step(x) = 1, x > 0; 0, x < 0` | 原表圖形未轉錄 | \(\frac12\delta(f_x)-\frac{i}{2\pi f_x}\) |
| `Delta function δ(x)` | 原表圖形未轉錄 | \(1\) |
| \(\operatorname{comb}(x)=\sum_{j=-\infty}^{\infty}\delta(x-j)\) | 原表圖形未轉錄 | \(\sum_{j=-\infty}^{\infty}\delta(f_x-j)\) |
| \(\cos(\pi x)\) | 原表圖形未轉錄 | \(\frac12\delta(f_x+\frac12)+\frac12\delta(f_x-\frac12)\) |
| \(\sin(\pi x)\) | 原表圖形未轉錄 | \(\frac{i}{2}\delta(f_x+\frac12)-\frac{i}{2}\delta(f_x-\frac12)\) |
| `Gaussian` \(e^{-\pi x^2}\) | 原表圖形未轉錄 | \(e^{-\pi f_x^2}\) |
| `circ(r) = 1, r < 1; 0, r > 1`，\(r=\sqrt{x^2+y^2}\) | 原表圖形未轉錄 | 待核對：分析整理為 \(J_1(2\pi\rho)/\rho\)，提供的 OCR 則顯示分母 \(\pi\rho\)；\(\rho=\sqrt{f_x^2+f_y^2}\) |

`circ` 列實際是二維徑向函數，雖然原表標題稱為 1D。其係數差異必須回查 PDF，不能將此列直接用作數值實作。

### Table 2.2：傳統照明的同調類型

| Illumination Type | Partial Coherence Factor | Source Shape |
|---|---|---|
| Coherent | σ = 0 | Point source |
| Incoherent | σ = ∞ | Infinite size source |
| Partially Coherent | 0 < σ < ∞ (but generally 0 < σ < 1) | Circular disk-shaped source |

### Table 2.3：傳統來源的 Kintner 權重

| Regime | Description | s1 | s2 | s3 |
|---|---|---:|---:|---:|
| \(\lambda/(pNA)\le1-\sigma\) | All three-beam imaging | 0 | 0 | 1 |
| \(1-\sigma\le\lambda/(pNA)\le\sqrt{1-\sigma^2}\) | Combination of two- and three-beam imaging | 0 | \(2\gamma\) | \(1-2\gamma\) |
| \(\sqrt{1-\sigma^2}\le\lambda/(pNA)\le1\) | Combination of one-, two- and three-beam imaging | \(\eta+2\gamma-1\) | \(2(1-\gamma-\eta)\) | \(\eta\) |
| \(1\le\lambda/(pNA)\le1+\sigma\) | Combination of one- and two-beam imaging | \(2\gamma-1\) | \(2(1-\gamma)\) | 0 |
| \(1+\sigma\le\lambda/(pNA)\) | Only the zero order is captured, all one-beam | 1 | 0 | 0 |

\(\gamma\)、\(\eta\) 依式 (2.91)–(2.93) 定義；反三角函數與根號轉錄不可靠，未在此重建。

### Table 2.4：薄環形來源的 Kintner 權重

| Regime | Description | s1 | s2 | s3 |
|---|---|---:|---:|---:|
| \(\lambda/(pNA)\le1-\sigma\) | All three-beam imaging | 0 | 0 | 1 |
| \(1-\sigma\le\lambda/(pNA)\le\sqrt{1-\sigma^2}\) | Combination of two- and three-beam imaging | 0 | \(2(1-\gamma)\) | \(2\gamma-1\) |
| \(\sqrt{1-\sigma^2}\le\lambda/(pNA)\le1+\sigma\) | Combination of one- and two-beam imaging | \(1-2\gamma\) | \(2\gamma\) | 0 |
| \(1+\sigma\le\lambda/(pNA)\) | Only the zero order is captured, all one-beam | 1 | 0 | 0 |

此表的 \(\gamma\) 由式 (2.107) 定義，不能與 Table 2.3 的 \(\gamma\) 混用。

## 圖示與證據界線

圖 2.24 使用 [[prolith]] 計算二維空中像，未提供軟體版本或獨立驗證資料。圖 2.19 展示實測環形來源並非理想 top-hat 分布，但不構成整章解析度推導的製程驗證。

4×–10× 縮小倍率及劑量感測器位置等描述屬於 2007 年出版背景，不應當作當代設備的完整概況。

## 待核對事項

1. 式 (2.50) 後將 \(g(ax,by)\) 中 \(a,b<1\) 描述為圖案縮小，與通常的縮放意義不一致，疑似原文筆誤。
2. 式 (2.46) 的 evanescent wave 閾值轉錄為 \(1/\lambda\)，但前述 \(f_x=n\alpha/\lambda\) 定義一般導向 \(n/\lambda\)；需確認是否限定空氣或轉錄有誤。
3. 式 (2.73)–(2.74)、Table 2.1 及圖示的整體係數與峰值正規化須分開核對。
4. OTF 一般可含相位，MTF 僅為其幅值；理想圓形瞳孔結果不能外推至任意像差系統。
5. 傳統來源 \(\sigma>2\) 的近似非同調敘述是成像實務近似，不是所有角度皆被包含的物理定義。

## 章末參考文獻

- Goodman, J.W., 1968, Introduction to Fourier Optics, McGraw-Hill (New York, NY).
- Wong, A.K., 2005, Optical Imaging in Projection Microlithography, SPIE Press (Bellingham, WA), pp. 3–8.
- Born, M. and Wolf, E., 1980, Principles of Optics, sixth edition, Pergamon Press, (Oxford, UK), pp. 113–117.
- Kintner, E.C., 1978, Method for the calculation of partially coherent imagery, Applied Optics, 17, 2747–2753.
- Hopkins, H.H., 1951, The concept of partial coherence in optics, Proceedings of the Royal Society of London, Series A, 208, pp. 263–277.
- Hopkins, H.H., 1953, On the diffraction theory of optical images, Proceedings of the Royal Society of London, Series A, 217, 408–432.
- Pati, Y.C. and Kailath, T., 1994, Phase-shifting masks for microlithography: automated design and mask requirements, Journal of the Optical Society of America A, 11, 2438–2452.
- Straaijer, A., 2005, Formulas for lithographic parameters when printing isolated and dense lines, Journal of Microlithography, Microfabrication, and Microsystems, 4, 043005–1.