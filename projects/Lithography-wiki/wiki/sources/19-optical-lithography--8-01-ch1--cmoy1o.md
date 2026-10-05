---
type: source
title: 光學微影基礎與半導體製程整合
authors: [Chris Mack]
year: 2007
url: ""
venue: "John Wiley & Sons, Ltd."
created: 2026-10-03
updated: 2026-10-03
tags: [光學微影, 半導體製程, 製造經濟]
related: [chris-mack, 光學微影, 圖案轉移, 蝕刻選擇比與光阻輪廓, 離子植入遮罩, moores-law, 光阻烘烤, 旋轉塗佈, 駐波與-barc, 化學放大型光阻, adi-與重工]
sources: ["optical-lithography/01 - ch1.pdf"]
---
# 光學微影基礎與半導體製程整合

本頁摘要 [[chris-mack]] 所著 *Fundamental Principles of Optical Lithography: The Science of Microfabrication* 第 1 章 *Introduction to Semiconductor Lithography*，涵蓋第 1–28 頁。章節將微影視為成像、光阻化學、熱處理、顯影、量測與圖案轉移共同構成的製造流程，而非單一曝光步驟。

## 核心框架

[[光學微影]] 形成三維光阻圖案，供蝕刻、選擇性沉積或離子植入使用。正型光阻的曝光區在顯影後移除；負型光阻的曝光區保留。此分類不可與加法／減法圖案轉移或圖案正負關係混為一談。

本章以約 2006–2007 年的產業狀態指出，微影與蝕刻通常重複 25–40 次，微影約占晶片製造成本 30%，約三分之一微影層屬於關鍵層。這些是教材式歷史概括，不是現代所有製程的固定比例。

## 圖案轉移與材料限制

### 蝕刻輪廓

第 3–4 頁說明 [[蝕刻選擇比與光阻輪廓]]：光阻線寬正確仍不足以保證最終蝕刻圖案正確。較高選擇比、較低水平侵蝕及接近垂直的側壁，均能減少 CD 變化。

式（1.1）的縮減速率量值為：

\[
2(R_H+R_V\cot\theta)
\]

若以剩餘線寬的時間導數表示，縮小應寫成：

\[
\frac{dCD}{dt}=-2(R_H+R_V\cot\theta)
\]

此處明確區分來源的「縮減速率」用語與帶方向的導數；模型假設直線側壁，不能涵蓋全部實際輪廓及蝕刻效應。

### 離子植入

第 5–6 頁以 Gaussian 深度分布近似 [[離子植入遮罩]] 的厚度需求：

\[
t_{\mathrm{resist}}\ge R_p+m\Delta R_p
\]

若光阻底部濃度不得超過峰值的 \(10^{-4}\)，來源建議 \(m=4.3\)。此濃度比門檻不是穿透離子的總積分比例。

圖 1.4 的量測材料是 [[az-7500]]；數據引用 Glawischnig and Parks（1996）。表 1.1 的標題與結構資料保留如下：

> Empirical model of ion implanted projected range (Rp, in nm) into photoresist versus ion energy (E, in keV) as Rp = aEb

| Dopant | Coefficient a | Power b |
|---|---:|---:|
| Boron | 26.9 | 0.63 |
| Phosphorous | 5.8 | 0.80 |
| Arsenic | 0.49 | 1.11 |

\[
R_p=aE^b,\qquad \Delta R_p=4.8E^{0.5}
\]

\(E\) 以 keV 計，\(R_p\) 與 \(\Delta R_p\) 以 nm 計。來源指出，1 MeV 以上的高原子序摻雜離子會產生較大 straggle。不得把上述材料專屬擬合直接套用至其他光阻；本章未提供原始數據、完整測試条件或誤差估計。

## 產業歷史與製造經濟

第 8–12 頁將 [[moores-law]] 解讀為具經濟條件的整合趨勢。[[gordon-moore]] 於 1965 年討論的元件包含電阻、電容及電晶體，整合規模指向最低每元件成本，而非可塞入的最大元件數。

1975 年的成長分解為密度、晶片面積與設計改進：

\[
(1.25)(1.20)(1.33)\approx2
\]

一年、兩年及約 18 個月的倍增描述對應不同時段與計數方式，不能互換。本章認為 minimum half-pitch 比任意單一特徵尺寸更適合代表電路密度。

示意經濟模型假設成本與面積／良率成正比，售價與最小特徵尺寸成反比。圖 1.7 使用 \(w_0=65\) nm、\(\sigma=10\) nm，最低成本約在 87 nm、良率約 90%；圖 1.8 的最高利潤約在 80 nm、良率約 65%。這是教學模型，不是晶圓廠實證結果。式（1.3）的文字擷取損壞，未在此重建完整公式。

[[semiconductor-industry-association]]、[[ntrs]] 與 [[itrs]] 代表將趨勢制度化為產業路線圖的歷史背景；其中對 2010 年的預測應視為當時預測，不可當成已驗證成果。

## 製程鏈與控制

典型流程為基板準備、旋轉塗佈、PAB、對準與曝光、PEB、顯影、postbake、量測檢查、圖案轉移及光阻剝除。

- [[hmds]] 用於改善基板附著；清潔、脫水與表面反應不能互相替代。
- [[旋轉塗佈]] 由快速徑向流動轉為溶劑蒸發；式（1.4）為 \(t\propto\nu^{0.4}/\omega^{0.5}\)，正文另給黏度指數約 0.4–0.6。
- [[光阻烘烤]] 必須涵蓋冷卻在內的完整熱史。PAB 後的殘留溶劑會影響 PEB 擴散與反應。
- [[化學放大型光阻]] 在曝光時產酸，在 PEB 中由酸催化改變樹脂溶解性。
- [[駐波與-barc]] 說明反射干涉如何造成側壁起伏及厚度相關線寬變化。
- [[tmah]] 是本章所述常用水性鹼顯影劑，濃度為 0.2–0.26 N。顯影劑耗竭可用 double-puddle 更新局部顯影能力。
- [[adi-與重工]] 在不可逆圖案轉移前提供重新加工機會；FI 後失敗通常不能以剝除光阻重做方式挽救。

## 曝光設備的歷史比較

第 20–22 頁將掃描投影、stepper 與 step-and-scan 的發展分別連結至 [[perkin-elmer]]、[[gca-corp]] 及 [[svg-lithography]]。step-and-scan 以較小瞬時成像場降低鏡頭設計難度，但增加光罩與晶圓載台複雜度。

表 1.2 結構資料如下：

|  | First Stepper (1978) | Immersion Scanner (2006) |
|---|---|---|
| Wavelength | 436 nm | 193 nm |
| Numerical Aperture | 0.28 | 1.2 |
| Field Size | 10 × 10 mm | 26 × 33 mm |
| Reduction Ratio | 10 | 4 |
| Wafer Size | 4″ (100 mm) | 300 mm |
| Throughput | 20 wafers per hour (0.44 cm²/s) | 120 wafers per hour (24 cm²/s) |

Rayleigh 準則為 \(R\propto\lambda/NA\)。浸潤介質可使 NA 大於 1，但解析度不能取代疊對、對焦、成像場、像差或光阻解析能力的控制。

## 資料品質與使用邊界

- 文字擷取中的部分 `mm` 疑為 `μm`，包括 proximity gap、練習題光阻厚度及早期解析度描述；未核對 PDF 前不採為定量事實。
- 正文的首台 stepper 成像場為直徑 14 mm，表 1.2 為 10 × 10 mm；來源未說明差異。
- 正文提及最高浸潤 NA 1.35，表格為 1.2；不得合併成同一設備規格。
- 正文晶片面積年增約 20%，圖 1.9 約 15%；不應移除各自歷史脈絡後合併。
- 來源使用 `hexamethyl disilizane` 及 `silonal` 拼寫；材料頁保留 HMDS 識別符，不把疑似錯字當成標準名稱。
- 光學微影相對 electron-beam 與 x-ray lithography 的成本優勢是當時教材論述，未提供完整技術成本比較。

## 重要引用

- Glawischnig, H. and Parks, C.C., 1996, SIMS and modeling of ion implants into photoresist, Proceedings of the 11th International Conference on Ion Implantation Technology, 579–582.
- Moore, G.E., 1965, Cramming more components onto integrated circuits, Electronics, 38, 114–117.
- Moore, G.E., 1975, Progress in digital integrated electronics, IEDM Technical Digest, 21, 11–13.
- Moore, G.E., 1995, Lithography and the future of Moore’s law, Proceedings of SPIE: Optical/Laser Microlithography VIII, 2440, 2–17.