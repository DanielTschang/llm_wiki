---
type: source
title: 解析度增強技術與可製造解析度
authors: [Chris Mack]
year: 2007
url: ""
venue: "John Wiley & Sons, Ltd."
created: 2026-10-03
updated: 2026-10-03
tags: [optical-lithography, RET, OPC, OAI, PSM]
related: [chris-mack, prolith, marc-levenson, ret, opc, oai, sraf, alternating-psm, attenuated-psm, 自然解析度, model-based-opc-校準與迭代, 數值孔徑與解析度, 焦距與曝光製程窗口]
sources: ["optical-lithography/10 - ch10.pdf"]
---
# 解析度增強技術與可製造解析度

本來源為 Chris Mack 的 *Fundamental Principles of Optical Lithography: The Science of Microfabrication*（2007）第 10 章〈Resolution Enhancement Technologies〉，書頁 411–456。章節以解析推導、微影模擬圖例與製造流程說明，探討如何針對有限類型的晶片圖案，結合 OPC、OAI 與 PSM 改善影像及共同製程窗口。

## 解析度的三種意義

- **特徵解析度**：指定圖案在要求的 DOF、曝光裕度與光阻輪廓規格下，可列印的最小尺寸。主要限制是 CD 與輪廓控制逐漸惡化，而非單一硬截止。
- **節距解析度**：指定 duty cycle 與 DOF 下，可列印的最小節距。本章將 duty cycle 定義為 spacewidth／linewidth。
- **[[自然解析度]]**：特殊光罩圖案趨近極限時，由成像系統及強度閾值決定的影像尺度；不是無條件的最小可製造尺寸。

在非零 DOF 要求下，提高 NA 不一定改善可製造解析度。Figure 10.4 的等線空示例要求 DOF = 200 nm 時，最佳 NA 約為 0.84，對應特徵尺寸約 105 nm；此數值只適用該示例條件。參見 [[非零-dof-要求下存在最佳-na]]。

以下方程式為依來源內容整理的可讀形式，並非 OCR 文字的逐字引文：

\[
p_{\min,\text{正常入射同調成像}}=\frac{\lambda}{NA},
\qquad
p_{\min,\text{理想對稱雙光束}}=\frac{\lambda}{2NA}.
\]

正常入射週期圖案若僅剩零階通過鏡頭，就不再形成週期影像。最佳化 OAI 或 Alt-PSM 可形成雙光束影像，達到理想 \(k_{\text{pitch}}=0.5\)；此光學極限仍不保證符合製程窗口及光阻輪廓規格。固定節距下縮小線寬並擴大空間，也不等於改善節距解析度。

## OPC：形狀正確不等於製程穩健

[[opc]] 透過修改光罩邊緣補償 proximity effects，包括 iso-dense print bias、轉角圓化與線端縮短。

CD-through-pitch 曲線依賴照明、像差與光阻；照明變化甚至可反轉 iso-dense bias 的正負號。Figure 10.8 的光阻比較中，較高對比度案例的 iso-dense bias 反而更大，不能將「更高光阻對比度」等同於「更小 proximity effect」。該圖的 dissolution selectivity parameter 分別為 4、5.5、7、10、16。

rule-based OPC 依經驗量測建立規則表；model-based OPC 使用校準模型，將設計分割成可移動邊緣，以 EPE 引導迭代。來源描述典型需三至六次迭代。compact models 結合物理空中像模型及簡化經驗光阻模型，可使用 [[sum-of-coherent-sources]]、Gaussian diffusion kernel 與 [[variable-threshold-resist-model]]。

本章的常規 model-based OPC 主要以 nominal focus／exposure 為目標，EPE 收斂不能直接視為共同製程窗口改善。Figure 10.13 的 OPC 與模擬使用 PROLITH。參見 [[model-based-opc-校準與迭代]] 及 [[rule-based-與-model-based-opc-比較]]。

## SRAF 與 OAI 的互補及節距限制

[[sraf]] 是不應列印的輔助線或槽，讓孤立主特徵具有較密集的成像與焦距響應。單純 bias OPC 可使孤立線與密集線在最佳條件下尺寸一致，卻不能使其 Bossung 曲線一致。

Figure 10.15 中，加入 50 nm SRAF 後，重疊 DOF 由 300 nm 增至 400 nm，約增加 33%。這是來源圖例結果，而非已獨立重現的實驗，詳見 [[sraf-提高示例重疊-dof]]。SRAF 越大通常越有效，但必須在整個要求的製程窗口內不列印。

[[oai]] 減少近軸照明，移動光罩繞射階，使通過鏡頭的光束對稱配置。理想 dipole 的最佳偏移量為：

\[
\sigma_{\mathrm{opt}}=\frac{\lambda}{2pNA}.
\]

此式限於來源討論的節距與繞射幾何。dipole 具方向選擇性；quadrupole 可同時照顧水平與垂直線，但不保證其他方向最佳。annular illumination 涵蓋較廣泛方向。

在來源的三光束轉雙光束情境中，OAI 降低最佳焦點的 NILS，卻提高 through-focus 穩定性；不可擴張為所有 OAI 配置都降低 in-focus NILS。

OAI 對接近設計節距的圖案最有效，SRAF 可補足較孤立圖案。兩者之間可能留下無足夠 OAI 收益、又無空間容納 SRAF 的 intermediate-pitch 區間。本章給出的 forbidden pitches 約為最小節距的 1.3–1.7 倍，是條件依賴的經驗範圍，不是絕對物理禁區。

## PSM：成像收益及不同誤差機制

[[psm]] 同時調整光罩透射振幅與相位。[[alternating-psm]] 使相鄰透明區具有 180° 相位差，理想情况下消除零階，形成等振幅、對稱的一階雙光束影像。其限制包括 phase conflicts、光罩製作複雜度，以及 spacewidth 依賴的 phase／intensity imbalance。

[[attenuated-psm]] 讓原本遮光區透射少量、相差 180° 的光。等線空、理想雙光束成像下，最大 NILS 對應相對強度透射率約 4.93%，並非所有 EPSM 圖案的通用最佳值。blank 的絕對 6% 透射率也不等於相對基板透射率。

| 圖案或光罩 | 小相位誤差的主要效應 | 必須保留的限制 |
|---|---|---|
| isolated phase edge | 近似加入背景 flare；與 defocus 耦合造成位置偏移 | flare 等效不代表位置誤差可忽略 |
| Alt-PSM | 引入零階；離焦時相鄰空間亮度不對稱，造成線位置偏移 | 線寬近似不變不等於圖案正確 |
| EPSM | 主要造成最佳焦點位移 | 來源案例未見 Alt-PSM 式位置變化，不能推廣至任意圖案 |

來源估計 6% EPSM 的 10° 相位誤差，在部分同調條件下可能使最佳焦點移動約 DOF 的 10%，並潛在造成約 20% DOF 損失。Figure 10.36 的特定同調案例則為約 14 nm 焦點位移。两者具有不同條件，不應混作統一相位容差。

Figure 10.33 的 6% EPSM 孤立空間案例出現約 17% sidelobe 強度。適當配置的非列印 assist slots 可藉干涉抑制 sidelobes，而非單純增加局部亮度。

## 數值與設定保留

下列區塊保留分析所擷取的來源數值與設定，供後續核對；並非完整原始圖表資料。

```text
DOF profile specifications:
CD ±10 %
sidewall angle >80 °
resist loss <10 %
exposure latitude specification of 6 %

Figure 10.1:
l = 193nm, NA = 0.9, s = 0.7
typical resist on a nonreflective substrate

Figure 10.14:
130-nm features
l = 248nm, NA = 0.85
quadrupole illumination optimized for a 260-nm pitch

Figure 10.15:
assist bars: 50 nm (k1 = 0.17), wafer dimensions
bias OPC: overlapping DOF = 300nm
scattering bars: overlapping DOF = 400nm

Figures 10.21–10.22:
NA = 0.85, l = 193nm, 100-nm line
chrome-on-glass mask
quadrupole settings of 0.8/0.2
```

| 圖案／模型 | 閾值或條件 | 來源尺度 |
|---|---|---|
| COG contact PSF | intensity threshold = 0.25 | 約 \(0.705\lambda/NA\) |
| 6% EPSM contact PSF | intensity threshold = 0.25 | 約 \(0.595\lambda/NA\)，較 COG 窄 16% |
| coherent LSF | intensity threshold = 0.25 | \(0.603\lambda/NA\) |
| isolated 180° phase edge | intensity level = 0.25–0.3 | \(0.26–0.29\lambda/NA\) |

## 書目與歷史界線

Marc Levenson 被來源稱為 wavefront engineering 的早期先驅；Alt-PSM 亦稱 Levenson PSM。關鍵引用包括：

- Levenson, M.D., 1993, Wavefront engineering for photolithography, Physics Today, 46, 28–36.
- Levenson, M.D., Viswanathan, N.S. and Simpson, R.A., 1982, Improving resolution in photolithography with a phase-shifting mask, IEEE Transactions on Electron Devices, ED-29, 1828–1836.
- Chen, J.F., Laidig, T., Wampler, K. and Caldwell, R., 1997, Optical proximity correction for intermediate-pitch features using sub-resolution scattering bars, Journal of Vacuum Science and Technology B, 15, 2426–2433.
- Socha, R., Dusa, M., Capodieci, L., Finders, J., Fung Chen, J., Flagello, D. and Cummings, K., 2000, Forbidden pitches for 130-nm lithography and below, Proceedings of SPIE: Optical Microlithography XIII, 4000, 1140–1155.

130／90-nm 世代的 OPC 轉型、PSM 採用程度及 phase-friendly layout 的未解狀態，均為 2007 年出版時的敘述，不能當作現況。

## 證據限制與既有研究連結

本章未提供完整 OPC 模型誤差分布或運算效能基準。公式 OCR 多處受損，Figure 10.37 caption 的 “PSM” 與正文 PSF 用語亦不一致；正式逐字引用或採用複雜方程式前應核對 PDF 原頁。

本章延伸 [[數值孔徑與解析度]]、[[離焦與焦深]]、[[焦距與曝光製程窗口]] 及 [[nils-與影像邊緣品質]]，並將相位與離焦造成的位置誤差連接至 [[關鍵形狀誤差與邊緣位置誤差]]。其共同意義是：RET 的評估必須同時記錄適用圖案、成像假設、製程規格及比較基準。