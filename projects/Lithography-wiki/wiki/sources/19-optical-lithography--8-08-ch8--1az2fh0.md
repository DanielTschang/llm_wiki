---
type: source
title: 半導體製造中的微影控制
authors: [Chris Mack]
year: 2007
url: ""
venue: "John Wiley & Sons, Ltd."
created: 2026-10-03
updated: 2026-10-03
tags: [微影, 製程控制, 量測]
related: [chris-mack, 焦距與曝光製程窗口, mask-error-enhancement-factor, overlay-控制, 線端縮短, 關鍵形狀誤差與邊緣位置誤差, 圖案倒塌, composite-cd-空間分解, overlay-fingerprint-表徵, 雙標靶劑量與焦距監控, semi-p35-1106]
sources: ["optical-lithography/08 - ch8.pdf"]
---
# 半導體製造中的微影控制

本來源為 Chris Mack 的 *Fundamental Principles of Optical Lithography: The Science of Microfabrication* 第 8 章，章名為 *Lithographic Control in Semiconductor Manufacturing*，涵蓋書籍頁碼 297–368。內容屬教科書綜述，結合理論、工程方法、歷史估計及模擬，並非單一實驗研究。

## 核心框架

微影品質包括光阻輪廓控制、overlay、下游相容性及可製造性。單一 CD 合格不代表形狀、位置、機械穩定性或最終元件性能合格。

CD 控制須同時管理均值與分布。CMOS gate 示例中，大 CD 尾端限制速度，小 CD 尾端增加漏電；across chip linewidth variation（ACLV）也影響路徑時序。縮小分布可在不超過漏電限制下減少平均 gate CD，但本章使用歷史性簡化模型，不能直接外推為現代元件的定量規則。

## 控制與最佳化

一階誤差傳播可寫為：

\[
\Delta CD \approx \sum_i \frac{\partial CD}{\partial v_i}\Delta v_i.
\]

- 製程控制降低輸入誤差 \(\Delta v_i\)。
- 製程最佳化降低敏感度 \(\partial CD/\partial v_i\)。

焦距與曝光具有非線性及交互作用，作者將一階式主要視為理解工具，而非完整計算模型。

CD 本身亦依賴 feature model 與擬合準則。即使輪廓已知，梯形底寬、指定高度的寬度及側壁直線擬合仍可能給出不同結果；離焦示例中的差異可超過 5%。相關术語引用 [[semi-p35-1106]]。

## 空間誤差與 overlay

[[composite-cd-空間分解]]利用相同位置跨晶圓平均估計系統性誤差。step-and-scan 可平均部分 scan 方向光學誤差，但 slit 方向特徵及 reticle CD 誤差仍保留。reticle 誤差須配合圖形特定的 [[mask-error-enhancement-factor]] 傳播至晶圓。

[[overlay-控制]]須區分相對既有層的 overlay、相對絕對網格的 registration，以及圖形依賴的 pattern placement errors（PPE）。量測偏差、可修正低階項、lens fingerprint、reticle fingerprint 與 scanner stage signature 是不同來源，不宜互相歸因。

box-in-box 可抵消對稱尺寸變化，但不能抵消不對稱。作者定義：

\[
TIS=Overlay(0^\circ)+Overlay(180^\circ).
\]

固定 TIS 可校正；TIS 變動增加不確定度。WIS 則源自晶圓標靶不對稱，通常不容易以固定偏移移除。[[aim]] 以多條帶及影像處理改善量測精密度與 CMP 後的標靶穩健性。

線性 overlay 模型有 10 個係數；加入兩個隨 scan direction 改號的 backlash 項後為 12 個。[[overlay-fingerprint-表徵]]補充高階工具差異。批次放行宜估計 die 層級 overlay-limited yield，而不只檢查稀疏量測點的最大誤差。

## 條件化製程窗口

[[焦距與曝光製程窗口]]是 CD、sidewall angle、resist loss 等規格同時滿足的區域。DOF 必須指定曝光範圍、圖形與輪廓規格；不是固定光學常數。

- 矩形表示固定範圍誤差；橢圓表示假定獨立 Gaussian 誤差的等密度輪廓。
- isofocal bias 使窗口彎曲，降低可用容忍度。
- 不同圖形及場位置的窗口須交疊，才能得到可同時使用的窗口與 UDOF。
- 單一 CD 劑量回饋可能掩蓋焦距漂移；[[雙標靶劑量與焦距監控]]可改善辨識，但近似二次焦距響應仍無法判別方向。

Figure 8.22 的 EL = 10% 示例，抽取文字記載 DOF 為矩形 `0.40 mm`、橢圓 `0.52 mm`，圖軸則標示 µm。數值不應在單位未核對前用於計算。

## MEEF、形狀及倒塌

MEEF 定義為晶圓光阻 CD 對已換算至晶圓尺度之 mask CD 的導數。只有 zero-bias 條件下，才等同對數導數。image MEEF 與實際有限對比度光阻 MEEF 不可混用。

特定 coherent 三光束等線／間距模型有 image MEEF = 0.5，並可寫成 image MEEF = 4/NILS。小 contact hole、等向性 mask 誤差近似下，有效劑量約與 mask CD 四次方成正比。[[低-k1-光阻對比度影響-meef]]記錄本章的虛擬光阻模擬，不能視為商用光阻實測。

[[線端縮短]]源自繞射、reticle 圓角及 PEB 擴散。本章模擬中，擴散造成的 LES 增量約隨 diffusion length 平方增加，補充了 [[peb-擴散與-dpsf]] 與 [[rdpsf]] 的形狀代價。

[[關鍵形狀誤差與邊緣位置誤差]]以逐點誤差描述複雜圖形。[[圖案倒塌]]則是乾燥毛細力與機械恢復力的競爭；[[圖案倒塌先於-cd-規格失效]]記錄 CD 尚未超規便可能失效的模型示例。

## 原始結構化資料

以下保留抽取資料的欄位名稱、數值與單位寫法。`mm`、`s` 疑為 µm、σ 的抽取問題；年份後的 2、3 是引用標記，未逕行修正。

### Table 8.1：各世代隨機焦距誤差估計

原始表題：*Examples of random focus errors (mm, 6s) for different lithographic generations*

| Error Source | 1991² i-line 0.50 mm | 1995³ i-line 0.35 mm | 1995³ KrF stepper 0.35 mm | 2001 KrF scanner 0.18 mm | 2005 ArF scanner 0.09 mm |
|---|---:|---:|---:|---:|---:|
| Lens Heating (Compensated) | 0.10 | 0.10 | 0.00 | 0.00 | 0.00 |
| Environmental (Compensated) | 0.20 | 0.20 | 0.10 | 0.10 | 0.05 |
| Mask Tilt (actual/16) | 0.05 | 0.05 | 0.10 | 0.05 | 0.05 |
| Mask Flatness (actual/16) | 0.12 | 0.12 | 0.12 | 0.12 | 0.07 |
| Wafer Flatness (over one field) | 0.30 | 0.33 | 0.33 | 0.15 | 0.07 |
| Chuck Flatness (over one field) | 0.14 | 0.03 | 0.03 | 0.03 | 0.03 |
| Laser Bandwidth | 0.0 | 0.0 | 0.20 | 0.1 | 0.04 |
| Autofocus Repeatability | 0.20 | 0.08 | 0.10 | 0.07 | 0.04 |
| Best Focus Determination | 0.30 | 0.15 | 0.10 | 0.10 | 0.05 |
| Vibration | 0.10 | 0.10 | 0.05 | 0.05 | 0.03 |
| Total RSS Random Focus Errors | 0.60 | 0.50 | 0.45 | 0.28 | 0.15 |

### Table 8.2：系統性與隨機焦距誤差的合併

原始表題：*Examples of systematic (mm, total range) and random focus error estimates combined to determine the Built-in Focus Errors (BIFE) of a process*

| Error Source | 1991² i-line 0.50 mm | 1995³ i-line 0.35 mm | 1995³ KrF stepper 0.35 mm | 2001 KrF scanner 0.18 mm | 2005 ArF scanner 0.09 mm |
|---|---:|---:|---:|---:|---:|
| Topography | 0.5 | 0.3 | 0.3 | 0.10 | 0.05 |
| Field Curvature and Astigmatism | 0.4 | 0.4 | 0.3 | 0.08 | 0.05 |
| Resist Thickness | 0.2 | 0.2 | 0.2 | 0.10 | 0.05 |
| Total Systematic Errors (range) | 1.1 | 0.9 | 0.8 | 0.28 | 0.15 |
| Total Random Errors (6s) | 0.60 | 0.50 | 0.45 | 0.28 | 0.15 |
| Range/s | 11 | 10.8 | 10.7 | 6 | 6 |
| Total BIFE (6s equivalent) | 1.5 | 1.2 | 1.1 | 0.47 | 0.25 |

這些是 1991–2005 製程的典型估計，不是完整量產資料集；未包含 backside particle hot spots 與 edge die 問題。BIFE 的合併使用均勻系統性分布與 Gaussian 隨機分布的卷積，不是直接將兩個總範圍相加。

### Figure 8.14：場內模型項目

| Error Term | Coefficients |
|---|---|
| Translation | ∆x, ∆y |
| Rotation | φx, φy |
| Magnification | ∆mx, ∆my |
| Trapezoid (Keystone) | t1, t2 |
| Lens Distortion | d3, d5 |

### Figure 8.43：形狀誤差示例

```text
CSEavg = 13.5 nm
CSE80 = 23 nm
CSE90 = 28 nm
CSE95 = 32 nm
CSE99.7 = 46 nm
```

上述數值是單一示例分布摘要，不是通用合格門檻。

## 解讀限制與待核對事項

1. 原文稱 contact resistance「proportional to the area」，又稱 CD 減少會增加電阻，兩者不一致。固定其他條件的反比面積模型下，直徑減少 10% 對應約 23.5% 電阻增加；本章的「roughly 20%」只能視為粗略估計。
2. 漏電對 gate length 的對數敏感度約 −10，與「10% 縮短造成 100% 增加」屬一階敘述，不能作精確有限變化計算。
3. 二維標準化半徑為 3 的 Gaussian 橢圓，內含機率約 98.9%，不是一維 ±3σ 的 99.7%；見 [[製程窗口橢圓應採何種聯合覆蓋率]]。
4. 擬合殘差可能仍含未建模的系統性誤差、量測偏差或取樣不足。
5. 光阻厚度造成焦距響應不對稱的解釋，須以相應空中像對稱假設為前提。
6. OCR 已損及若干根號、分母、希臘字母與單位；未核對的高階公式不宜直接實作。

## 既有知識連結

本章將 [[離焦與焦深]]延伸至製造規格與誤差預算，將 [[鏡頭像差與-zernike-polynomial]]連結到 H–V bias、overlay 與 PPE 的不同機制，並為 [[nils-與影像邊緣品質]]、[[光阻理論對比度與量測對比度]]、[[original-mack-model]]提供條件化 MEEF 脈絡。良率與批次處置則延伸 [[adi-與重工]]。