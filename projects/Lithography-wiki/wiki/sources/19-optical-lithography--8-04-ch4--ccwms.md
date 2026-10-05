---
type: source
title: 光阻內成像：駐波與 swing curves
authors: [Chris Mack]
year: 2007
url: ""
venue: John Wiley & Sons, Ltd.
created: 2026-10-03
updated: 2026-10-03
tags: [光學微影, 薄膜干涉, 光阻]
related: [chris-mack, 駐波與-barc, swing-curves, tarc, 光阻內成像, 有效吸收係數, cauchy-equation, barc-穩健最佳化, barc-與-tarc-比較, swing-ratio-極值厚度差是否為半週期]
sources: ["optical-lithography/04 - ch4.pdf"]
---
# 光阻內成像：駐波與 swing curves

本來源為 [[chris-mack]] 所著 *Fundamental Principles of Optical Lithography: The Science of Microfabrication* 第 4 章，原章名為 *Imaging in Resist: Standing Waves and Swing Curves*。ISBN：`978-0-470-01893-4`。本章將空中像理論延伸至吸收性光阻與多層膜，說明反射、折射、偏振及光阻厚度如何影響曝光與線寬。

## 核心結論

[[駐波與-barc]] 描述固定膜厚下的深度曝光振盪；[[swing-curves]] 則描述膜厚改變時，CD、dose-to-clear 與晶圓反射率的變化。兩者同源於薄膜干涉，但不是同一現象。正入射週期為 \(\lambda/(2n_2)\)，斜入射為 \(\lambda/(2n_2\cos\theta_2)\)。

多層底膜可整合為複數有效反射係數。其幅值控制干涉幅度，相位控制振盪位置與光阻底部曝光；因此，僅追蹤強度反射率不足以預測 footing、undercut 或 CD 變動。本章 nitride／oxide／silicon 案例中，約 ±20 nm 的 nitride 厚度變化已造成顯著輪廓差異，但不是通用臨界值。

BARC 降低有效基板反射，可改善駐波、swing curves 與 reflective notching。[[tarc]] 降低上表面反射，主要抑制 swing curves，不能據此認定光阻內的基板反射駐波已消失。增加光阻吸收亦能降低干涉，但會加大深度曝光梯度。

## 公式與適用條件

本章採用 \(e^{+ikz}\) 傳播慣例，吸收材料的複數折射率寫為 \(\tilde n=n+i\kappa\)，其中：

\[
\alpha=\frac{4\pi\kappa}{\lambda},\qquad
\tau_D=e^{i2\pi\tilde n_2D/\lambda}.
\]

對正入射、均勻單層光阻，總電場為：

\[
E_T(z)=E_I
\frac{\tau_{12}\left(e^{ik_2z}+\rho_{23}\tau_D^2e^{-ik_2z}\right)}
{1+\rho_{12}\rho_{23}\tau_D^2}.
\]

多層膜需以有效複數反射係數取代 \(\rho_{23}\)。上述均勻膜解不宜直接套用至任意非均勻光阻；若僅吸收隨深度小幅變動，可用 \(\int_0^z\alpha(z')\,dz'\) 作近似修正。

本章定義的 swing amplitude 為：

\[
A_{\mathrm{swing}}=4|\rho_{12}\rho_{23}|e^{-\alpha D}.
\]

注意 \(\rho\) 是電場反射係數，強度反射率為 \(R=|\rho|^2\)。斜入射干涉相位涉及 \(D\cos\theta_2\)，吸收則涉及 \(D/\cos\theta_2\)，不可將所有厚度項作同一替換。

## BARC 設計與劑量誤差

單層 BARC 的正入射零反射條件為：

\[
\rho_{12}+\rho_{23}\tau_D^2=0.
\]

此處層編號依 BARC 模型定義：第 1 層為光阻、第 2 層為 BARC、第 3 層為基板，與前述光阻模型不同。複數條件提供兩項實數約束，而 \(n,\kappa,D\) 提供三個自由度，因此理想設計可形成一族解。

圖 4.16 的 193-nm 光阻／silicon 案例，在允許強度反射率 0.1% 時：

- 20-nm BARC 約可容忍 1 nm 厚度誤差，或 0.053 折射率誤差。
- 40-nm BARC 約可容忍 2 nm 厚度誤差，或 0.04 折射率誤差。

這些是其他參數無誤差時的個別容差，不是聯合容差。高角度非偏振光同時要求 s／p 複數零反射，共四項約束，單層 BARC 一般無法完全滿足。透明底膜厚度變動大時，應採 [[barc-穩健最佳化]]，降低預期厚度範圍內的最大反射率。

图 4.23 的 100-nm 線、280-nm pitch、環形照明模型，即使基板反射率為 0.07%，仍有可見 CD swing；這不是跨製程反射率規格。

在 swing minimum 附近，對小厚度增加 \(\Delta\) 及良好 BARC：

\[
\frac{\Delta E}{E}\approx
\alpha\Delta+
4|\rho_{12}\rho_{23}|e^{-\alpha D}
\left(\frac{2\pi n_2\Delta}{\lambda}\right)^2.
\]

193-nm 示例採 \(\alpha=1.2\,\mu\mathrm{m}^{-1}\)、\(D=200\) nm；厚度變動涵蓋約 56 nm 週期且 \(R=1\%\) 時，吸收與 swing 分別增加約 6.7% 與 8.3% 有效劑量誤差。

## 高 NA 與曝光強度

[[光阻內成像]] 必須逐繞射階次納入角度相關透射、吸收、多重反射及深度離焦。折射降低光阻內角度，改善 TM 電場方向重疊。理想雙光束模型中：

\[
C_{\mathrm{TM,resist}}=
1-\frac{2\sin^2\theta_{\mathrm{air}}}{n^2}.
\]

當 \(n=1.7\)、空氣入射角為 30°，TM 對比由約 0.5 增至約 0.83。非偏振、NA = 0.9 示例由約 0.19 增至約 0.72，但後者忽略 TE／TM 透射差異，偏樂觀。

光阻折射亦產生系統性球面像差；\(n_1=1,n_2=1.7\)、NA = 0.9 的估算約為每一波長光阻厚度 10 milliwaves。這不是鏡頭本身像差的量測。

曝光動力學所需的強度是傳播方向垂直截面的能流，而非介面投影功率：

\[
J=I\cos\theta,\qquad
\frac{I_t}{I_i}=T\frac{\cos\theta_1}{\cos\theta_2},\qquad
I(z)\propto e^{-\alpha z/\cos\theta_2}.
\]

單位體積吸收能量率與 \(\alpha I\) 成正比。此區別補充 [[radiometric-correction]]。

## 原始結構化資料

以下保留表 4.1–4.3 的欄位、材料名稱與數值；不得視為任意現行產品的通用規格。

### 表 4.1：光源波長與未縮窄頻寬

| Source | Center Wavelength (nm) | Typical Unnarrowed Bandwidth (nm) |
|---|---:|---:|
| Mercury Arc Lamp g-line | 435.8 | 5 |
| Mercury Arc Lamp h-line | 404.7 | 5 |
| Mercury Arc Lamp i-line | 365.0 | 6 |
| KrF excimer laser | 248.35 | 0.30 |
| ArF excimer laser | 193.3 | 0.45 |
| F2 excimer laser | 157.63 | 0.002 |

### 表 4.2：光阻 Cauchy 係數

來源所列網址：`http://www.microe.rit.edu/research/lithography/`

| Photoresist | C1 | C2 (nm2) | C3 (nm4) | Wavelength Range (nm) |
|---|---:|---:|---:|---|
| SPR-500 i-line (unexposed) | 1.6133 | 5481.6 | 1.4077 × 10⁹ | 300–800 |
| SPR-500 i-line (fully exposed) | 1.5954 | 9291.9 | 4.2559 × 10⁸ | 300–800 |
| 193-nm resist | 1.5246 | 3484 | 1.49 × 10⁸ | 190–800 |

### 表 4.3：材料複數折射率

| Material | n at 436 nm | n at 365 nm | n at 248 nm | n at 193 nm |
|---|---|---|---|---|
| Photoresist | 1.65 + i0.022 | 1.69 + i0.027 | 1.76 + i0.010 | 1.71 + i0.018 |
| Silicon | 4.84 + i0.178 | 6.50 + i2.61 | 1.57 + i3.57 | 0.883 + i2.78 |
| Amorphous Silicon | 4.45 + i1.73 | 3.90 + i2.66 | 1.69 + i2.76 | 1.13 + i2.10 |
| Silicon Dioxide | 1.470 + i0.0 | 1.474 + i0.0 | 1.51 + i0.0 | 1.56 + i0.0 |
| Silicon Nitride | 2.06 + i0.0 | 2.09 + i0.0 | 2.28 + i0.0 | 2.66 + i0.240 |
| Aluminum | 0.580 + i5.30 | 0.408 + i4.43 | 0.190 + i2.94 | 0.113 + i2.20 |
| Copper | 1.17 + i2.33 | 1.27 + i1.95 | 1.47 + i1.78 | 0.970 + i1.40 |
| Chrome | 1.79 + i4.05 | 1.39 + i3.24 | 0.85 + i2.01 | 0.84 + i1.65 |
| Gallium Arsenide | 5.07 + i1.25 | 3.60 + i2.08 | 2.27 + i4.08 | 1.36 + i2.02 |
| Indium Phosphide | 4.18 + i0.856 | 3.19 + i1.95 | 2.13 + i3.50 | 1.49 + i2.01 |
| Germanium | 4.03 + i2.16 | 4.07 + i2.58 | 1.39 + i3.20 | 1.13 + i2.09 |

## 證據限制與待核對事項

本章主要提供解析推導與數值示例，不是完整的統計實驗報告。圖 4.26 的 AZ Photoresist 影像支持 BARC 可抑制 reflective notching，但未提供可重建的量化實驗條件；不可與 AZ 7500 混同。

- 第 23 頁對相鄰極值厚度差的敘述疑似混用半週期與完整週期，見 [[swing-ratio-極值厚度差是否為半週期]]。
- 第 4.142 式反射乘積的符號／共軛，以及第 4.143 式非偏振加權方式，須核對原版；暫不作為計算依據。
- OCR 中 `mm`／`mm−1` 有微米符號遺失風險；上述微米尺度依物理脈絡判讀，定量重用前應核對原版。
- 同調長度 \(L\approx\lambda^2/(2\Delta\lambda)\) 是簡化抵消估算，不是通用精確截止。
- 參考文獻 6、10 的 Brunner 1991 引用似為同篇論文，不算兩份獨立佐證。
- CEL 使用情況、材料可得性及典型規格均屬 2007 年背景。

## 與既有理論的銜接

本章延伸 [[19-optical-lithography--8-02-ch2--k73j1g]] 與 [[19-optical-lithography--8-03-ch3--s477p8]] 的空中像理論，並補充 [[向量成像與偏振]]、[[離焦與焦深]]、[[鏡頭像差與-zernike-polynomial]] 與 [[浸潤微影]]。目前只有既有頁面索引，尚不能確認頁面全文是否存在直接矛盾。