---
type: source
title: 空中像形成的細節與非理想成像
authors: [Chris Mack]
year: 2007
url: ""
venue: John Wiley & Sons, Ltd.
created: 2026-10-03
updated: 2026-10-03
tags: [光學微影, 空中像, 成像模型]
related: [chris-mack, 19-optical-lithography--8-02-ch2--k73j1g, 鏡頭像差與-zernike-polynomial, 離焦與焦深, flare, 向量成像與偏振, jones-pupil, 浸潤微影, nils-與影像邊緣品質, 含像差與離焦的瞳孔建模, 大型島形圖案-flare-量測, stepper-與-step-and-scan-成像比較, lord-rayleigh]
sources: ["optical-lithography/03 - ch3.pdf"]
---
# 空中像形成的細節與非理想成像

本頁整理 [[chris-mack]] 的 *Fundamental Principles of Optical Lithography: The Science of Microfabrication* 第 3 章 *Aerial Image Formation – The Details*，書頁 75–128，ISBN `978-0-470-01893-4`。本章延伸 [[19-optical-lithography--8-02-ch2--k73j1g]] 的理想空中像模型，加入像差、離焦、瞳孔透射、flare、掃描與偏振效應，並說明浸潤微影及影像品質指標。

## 證據範圍

本章是解析推導、模擬與引用量測的教材整合，不是單一實驗研究。設備性能、光源頻寬及浸潤技術展望均屬 **2007 年時點**，不可直接視為現況。教材引用的量測亦不等於獨立重現。

## 非理想成像的主要機制

### 圖案依賴的像差

[[鏡頭像差與-zernike-polynomial]] 將波前 OPD 展開為瞳孔上的多項式。不同節距與方向的繞射級次取樣不同瞳孔位置，因此鏡頭整體 RMS 波前誤差或 Strehl ratio 不能單獨決定所有圖案的品質。

- 純 x-tilt 對 y 向線／空圖案造成位置誤差 \(-Z_1\lambda/NA\)。
- 球面像差造成節距相關最佳焦點；像散造成方向相關最佳焦點及焦距相關 H–V bias。
- 彗差造成圖案相關位置偏移、左右 CD 不對稱及離焦時的單側光阻輪廓退化。圖 3.10 是模擬，不是設備量測。

同調線／空图案的三階 x-coma 位置誤差為
\[
-\frac{Z_6\lambda}{NA}\left[3\left(\frac{\lambda}{pNA}\right)^2-2\right],
\]
僅適用於 \(\lambda/NA<p<3\lambda/NA\) 的指定模型。

Strehl ratio 是最佳焦點實際 [[點擴散函數]] 峰值相對理想峰值；小像差下可用
\[
S\approx e^{-(2\pi W_{\mathrm{RMS}})^2}
\]
估計。它是全域光學指標，不是特定圖案的 CD 保證。

### 色差、瞳孔透射與 flare

色差效應由鏡頭波長響應和光源光譜共同決定。圖 3.13a 的特定 NA = 0.6 鏡頭呈近似線性的波長—最佳焦點關係；不同波長形成的影像以強度相加，造成焦向平均。圖 3.13b 的 180-nm 線、500-nm 節距案例不可泛化為所有鏡頭的頻寬容忍度。

瞳孔濾波可改變振幅與相位；高空間頻率透射降低稱為 apodization。[[flare]] 則是非預期反射與散射到達晶圓的光，依賴局部圖案、總透光量、場位置與晶圓反射率。簡化模型為
\[
I=SF\cdot EF+(1-SF)I_0,
\]
並可用透光面積比例 \(CF\) 近似 \(EF\)。短程散射另以 \(PSF_{\mathrm{scat}}*I_0\) 描述。

圖 3.17 的線性區擬合資料為：

| 橫軸 | 縱軸 | 擬合 |
|---|---|---|
| Clear Die Area (mm²) | Flare (%) | `y = 0.02x + 1.1` |

這是特定引用量測，不是通用設備定律；圖說作者與參考文獻編號的對應仍須核對。

### 離焦、照明與掃描

[[離焦與焦深]] 的幾何模型為
\[
OPD=n\delta(1-\cos\theta),\qquad \Delta\Phi=\frac{2\pi}{\lambda}OPD.
\]
此處假設像距遠大於離焦距離。近軸近似在空氣中 NA = 0.6 的瞳孔邊緣約有 10% 誤差，NA = 0.93 約有 32% 誤差。

等半徑雙光束具有相同離焦相位，理想同調模型的影像可不隨離焦改變；有限面積 dipole 光源仍會劣化。單側偏軸照明可能產生焦距相關位移，對稱照明可消除該位移。

小離焦下，focus averaging 可寫為
\[
I(x,\delta)\approx I(x,0)-f(x)(\delta^2+\sigma_F^2).
\]
image isofocal point 是影像強度對焦距不敏感的位置，部分模型僅近似成立，且不應等同實際光阻 CD 的 isofocal 值。

[[stepper-與-step-and-scan-成像比較]] 顯示掃描可平均掃描方向的場依賴像差及 flare，但同步誤差造成橫向模糊，焦距動態誤差造成 focus averaging。8-mm 狹縫、2-kHz 雷射、50 pulses 的例子對應晶圓速度 0.32 m/s；4× 系統的光罩速度是其 4 倍。

### 向量成像與浸潤

[[向量成像與偏振]] 中，指定線／空三光束模型的 TM 零級／一級干涉乘上 \(\cos\theta\)，正負一級干涉乘上 \(\cos2\theta\)。進入高折射率光阻後，角度減小，TM 重疊改善；空中像不能直接代表光阻內的偏振損失。

[[Jones pupil]] 用複數 2×2 矩陣描述偏振相關傳輸，但不能直接描述 depolarization；本章建議與 flare 模型結合。

[[浸潤微影]] 解除低折射率間隙對傳播角度的限制，需配合更高 NA 鏡頭才能提高解析度。固定繞射級次與倍率時，換用浸潤介質不會自動增加光阻內角度。固定圖案與 NA 下的 DOF 改善，不代表提高 NA 並縮小圖案後焦深仍增加。

### 邊緣品質與光阻模型

[[NILS 與影像邊緣品質]] 使用
\[
ILS=\frac{d\ln I}{dx},\qquad NILS=w\frac{d\ln I}{dx}
\]
在名義邊緣量化局部光學品質。特定同調、最佳焦點、等線／空三光束 TE 模型得到 NILS = 8，不是所有 TE 成像的通用值。

閾值模型以 \(I_{\mathrm{th}}=E_{\mathrm{th}}/E\) 擷取 image CD；resist bias 可改善經驗對應，但閾值與邊緣偏差透過影像斜率相互關聯。這些近似不能取代光阻化學及三維輪廓模擬。

## 來源結構化資料

以下保留表 3.1、3.2 的編號、公式與技術名稱；數學排版恢復上下標與希臘字母。表 3.1 原標題稱「first 36 terms」，實際列出 Z0–Z36，共 37 項。不同 Zernike 編號慣例不可混用。

### 表 3.1：Fringe Zernike polynomial

| Term | Fringe Zernike Formula, \(F_i\) | Common Name |
|---|---|---|
| Z0 | \(1\) | Piston |
| Z1 | \(R\cos\phi\) | x-Tilt |
| Z2 | \(R\sin\phi\) | y-Tilt |
| Z3 | \(2R^2-1\) | Power (paraxial focus) |
| Z4 | \(R^2\cos2\phi\) | 3rd Order Astigmatism |
| Z5 | \(R^2\sin2\phi\) | 3rd Order 45° Astigmatism |
| Z6 | \((3R^2-2)R\cos\phi\) | 3rd Order x-Coma |
| Z7 | \((3R^2-2)R\sin\phi\) | 3rd Order y-Coma |
| Z8 | \(6R^4-6R^2+1\) | 3rd Order Spherical |
| Z9 | \(R^3\cos3\phi\) | Trefoil (3rd Order 3-Point) |
| Z10 | \(R^3\sin3\phi\) | 45° Trefoil |
| Z11 | \((4R^2-3)R^2\cos2\phi\) | 5th Order Astigmatism |
| Z12 | \((4R^2-3)R^2\sin2\phi\) | 5th Order 45° Astigmatism |
| Z13 | \((10R^4-12R^2+3)R\cos\phi\) | 5th Order x-Coma |
| Z14 | \((10R^4-12R^2+3)R\sin\phi\) | 5th Order y-Coma |
| Z15 | \(20R^6-30R^4+12R^2-1\) | 5th Order Spherical |
| Z16 | \(R^4\cos4\phi\) | Quadrafoil (3rd Order 4-Point) |
| Z17 | \(R^4\sin4\phi\) | 45° Quadrafoil |
| Z18 | \((5R^2-4)R^3\cos3\phi\) | 5th Order Trefoil (5th Order 3-Point) |
| Z19 | \((5R^2-4)R^3\sin3\phi\) | 5th Order 45° Trefoil |
| Z20 | \((15R^4-20R^2+6)R^2\cos2\phi\) | 7th Order Astigmatism |
| Z21 | \((15R^4-20R^2+6)R^2\sin2\phi\) | 7th Order 45° Astigmatism |
| Z22 | \((35R^6-60R^4+30R^2-4)R\cos\phi\) | 7th Order x-Coma |
| Z23 | \((35R^6-60R^4+30R^2-4)R\sin\phi\) | 7th Order y-Coma |
| Z24 | \(70R^8-140R^6+90R^4-20R^2+1\) | 7th Order Spherical |
| Z25 | \(R^5\cos5\phi\) | Pentafoil (3rd Order 5-Point) |
| Z26 | \(R^5\sin5\phi\) | 45° Pentafoil |
| Z27 | \((6R^2-5)R^4\cos4\phi\) | 5th Order Quadrafoil (5th Order 4-Point) |
| Z28 | \((6R^2-5)R^4\sin4\phi\) | 5th Order 45° Quadrafoil |
| Z29 | \((21R^4-30R^2+10)R^3\cos3\phi\) | 7th Order Trefoil (7th Order 3-Point) |
| Z30 | \((21R^4-30R^2+10)R^3\sin3\phi\) | 7th Order 45° Trefoil |
| Z31 | \((56R^6-105R^4+60R^2-10)R^2\cos2\phi\) | 9th Order Astigmatism |
| Z32 | \((56R^6-105R^4+60R^2-10)R^2\sin2\phi\) | 9th Order 45° Astigmatism |
| Z33 | \((126R^8-280R^6+210R^4-60R^2+5)R\cos\phi\) | 9th Order x-Coma |
| Z34 | \((126R^8-280R^6+210R^4-60R^2+5)R\sin\phi\) | 9th Order y-Coma |
| Z35 | \(252R^{10}-630R^8+560R^6-210R^4+30R^2-1\) | 9th Order Spherical |
| Z36 | \(924R^{12}-2772R^{10}+3150R^8-1680R^6+420R^4-42R^2+1\) | 11th Order Spherical |

### 表 3.2：等線／空影像的 NILS

| Image Type | Equation | \(\beta_0\) | \(\beta_1\) | \(\beta_2\) | NILS |
|---|---|---|---|---|---|
| Coherent, in focus, TE | (3.69) | \(\frac14+\frac{2}{\pi^2}\) | \(\frac2\pi\) | \(\frac{2}{\pi^2}\) | \(8\) |
| Coherent, in focus, TM | (3.71) | \(\frac14+\frac{2}{\pi^2}\) | \(\frac2\pi\cos\theta\) | \(\frac{2}{\pi^2}\cos(2\theta)\) | \(\frac{8\cos\theta}{1+\frac{8}{\pi^2}\sin^2\theta}\) |
| Coherent, out of focus, TE | (3.27) | \(\frac14+\frac{2}{\pi^2}\) | \(\frac2\pi\cos(\Delta\Phi)\) | \(\frac{2}{\pi^2}\) | \(8\cos(\Delta\Phi)\) |
| Partially coherent, out of focus, TE | (3.29) | \(\frac14+\frac{2}{\pi^2}\) | \(\frac2\pi\cos(\Delta\Phi)\frac{2J_1(a)}a\) | \(\frac{2}{\pi^2}\frac{J_1(2a)}a\) | \(\frac{8\cos(\Delta\Phi)\frac{2J_1(a)}a}{1+\frac8{\pi^2}\left(1-\frac{J_1(2a)}a\right)}\) |
| Incoherent, in focus, TE | See Chapter 2 | \(\frac12\) | \(\frac2\pi MTF(1/p)\) | \(0\) | \(4MTF(1/p)\) |

部分同調列使用近軸離焦、圓形光源及未被瞳孔裁切的指定三光束條件；\(a=2\pi\delta\sigma NA/p\)。慣例通常調整 NILS 符號為正值。

## 校對與解讀界線

- 書頁 88 的擷取文字 `0.255mm/pm` 與圖軸及推導不一致，應視覺核對 µm/pm；本頁不採用未確認的毫米單位。
- 書頁 115 的 `J11 = J12` 純量極限敘述與矩陣及式 (3.74) 不一致，詳見 [[Jones pupil]]。
- 圓偏振除奇數倍 \(\pi/2\) 相位差外，亦需正交分量振幅相等；不能將來源的簡略條件當作充分條件。
- 零級與一級相差 90° 時，式 (3.27) 的主要諧波消失，但二次諧波仍可存在；「沒有圖案」的敘述過強。
- 脈衝平均改善因子中的根號、大型島尺寸及散射寬度中的 µm 符號可能在擷取時遺失。
- 未取得既有頁面全文，不能據此宣稱跨來源矛盾。