---
type: source
title: 傳統光阻的曝光與烘烤化學
authors: [Chris Mack]
year: 2007
url: ""
venue: "John Wiley & Sons, Ltd"
created: 2026-10-03
updated: 2026-10-03
tags: [光學微影, 光阻, 曝光, 烘烤]
related: [chris-mack, dnq, novolac, kodak-820, dill-abc-參數, 曝光互易性, 修正-fujita-doolittle-模型, peb-擴散與-dpsf, abc-透射率全曲線擬合, 以-a-量測-pac-熱分解, 晶圓烘烤熱歷程建模, 光阻內成像, 光阻烘烤, swing-curves]
sources: ["optical-lithography/05 - ch5.pdf"]
---
# 傳統光阻的曝光與烘烤化學

本來源為 Chris Mack 的 *Fundamental Principles of Optical Lithography: The Science of Microfabrication*（2007）第 5 章，*Conventional Resists: Exposure and Bake Chemistry*，書內頁碼 191–222。ISBN：978-0-470-01893-4。

本章將[[光阻內成像]]連接至化學潛像、烘烤後材料組成及顯影行為。主體為 [[dnq]]/[[novolac]] 傳統正型光阻；引用的實驗、教材模型、模擬與習題必須分開解讀。

## 曝光與潛像

Grotthuss–Draper law 指出只有被吸收的光能造成光化學變化。Lambert 定律描述光強衰減，Beer 定律則在適用濃度範圍內將吸收係數連接至成分濃度。跨成分吸收的線性可加性也需要實驗確認。

[[dill-abc-參數]]以以下模型描述曝光：

\[
\alpha=Am+B,\qquad
\frac{\partial m}{\partial t}=-CIm,\qquad
C=\Phi\sigma_{M-\mathrm{abs}}\frac{\lambda}{hc}.
\]

其中 \(m=M/M_0\) 是相對 PAC 濃度；曝光後的 \(m(x,y,z)\) 是潛像，不是最終顯影輪廓。\(A>0\) 表示漂白，\(A<0\) 表示曝光變暗。漂白光阻在反射基板上通常需要耦合光學與反應的數值計算。

[[曝光互易性]]並不限於非漂白光阻；習題 5.6 要求證明一般一階曝光方程在漂白時仍具有互易性。高強度造成的缺水或曝光加熱，則可能使其失效。來源提及相對濕度低於約 30% 時 DNQ 反應可能無法完成；此數值是該章製程背景，不能當作所有配方的固定門檻。

## PAB、溶劑與熱分解

PAB 移除溶劑，可穩定膜厚、改善附著並降低表面黏性，但 DNQ 在約 70–80 °C 以上開始熱分解。簡化模型為：

\[
m'=\exp(-K_Tt_b),\qquad
K_T=A_r\exp(-E_a/RT),\qquad A=A_{\mathrm{NB}}m'.
\]

水分充足時，分解中間物可形成羧酸，增加未曝光溶解速率 \(r_{\min}\)，而本章模型下不改變完全曝光速率 \(r_{\max}\)。另一條路徑形成與 novolac 結合的酯，降低 \(r_{\max}\)；其完整顯影行為仍不清楚。

[[kodak-820]] 的特定資料支持 \(E_a=30.3\) Kcal/mol、\(\ln(A_r)=35.3\)，其中 \(A_r\) 的時間單位為 1/minute；100 °C、30 分鐘對流烘烤約分解 11% PAC。這些數值不能移植至其他產品或[[化學放大型光阻]]。

[[修正-fujita-doolittle-模型]]以自由體積描述溶劑擴散；典型 PAB 期間擴散率可變動四至六個數量級。「相同最終膜厚對應近乎相同溶劑分布」是固定烘烤條件下的模型預測，不是普遍實驗定律。

圖 5.6 的 248-nm 光阻案例涉及酸擴散，不能歸為 DNQ 的 PAC 擴散。

## PEB 與駐波

[[peb-擴散與-dpsf]]在固定擴散係數、無限域近似下，以 Gaussian 卷積平滑潛像：

\[
m^*=m\otimes DPSF,\qquad \sigma=\sqrt{2Dt}.
\]

擴散長度需接近或超過半個駐波週期，同時遠小於最小特徵尺寸。教材使用的典型尺度為：i-line 半週期約 55 nm、特徵約 300 nm 以上；248 nm 半週期約 35–40 nm，對小於 130 nm 的特徵則可能產生顯著模糊。這些不是今日製程界限。

圖 5.7 使用 PROLITH 比較 20、40、60 nm 擴散長度的 i-line 光阻輪廓。PEB 可平滑駐波，但不處理 [[swing-curves]]；BARC 可降低兩者，見[[peb-擴散與-barc-的駐波抑制比較]]。

## 熱歷程與量測

[[晶圓烘烤熱歷程建模]]區分 hot plate 設定溫度與晶圓實際溫度，並積分溫度相依速率。間隙變動對升溫時間的影響可能大於對平衡溫度的影響。

[[abc-透射率全曲線擬合]]使用透射率—入射劑量曲線，要求已知膜厚與光強、折射率匹配及基板背面抗反射。若 \(A=0\)，透射率無曝光變化，此方法不能取得 \(C\)。

## 結構化資料

以下保留來源擷取文字的欄名、數值與單位。表 5.4 及若干習題的 `mm`／`mm−1` 與正文及圖軸的 `µm`／`1/µm` 不一致；未核對原 PDF 前，不應直接用於計算。

### 表 5.1：溶劑擴散參數

```text
Typical solvent diffusion parameters in photoresist

Parameter       Value
B               0.737
b′              0.351
vg              0.0289
a2 (1/K)        8.7E-04
Tg (°C)         110.5
D0 (nm2/s)      0.0967
```

### 表 5.2：特定厚膜 i-line 光阻的顯影擬合

```text
Development rmax and n values (see Chapter 7) fitted to measured dissolution
rate data as a function of PAB temperature (and thus, solvent content)
for a thick i-line resist18

Temperature (°C)    wt.% Solvent    rmax (nm/s)    Develop n
70                  21.1           104.3          1.71
80                  19.5           90.3           1.77
90                  17.8           78.9           1.96
```

這些資料顯示特定材料的共同變化，不足以單獨證明所有光阻中溶劑含量與顯影速率的通用因果關係。

### 表 5.3：晶圓熱模型參數

```text
Typical values for silicon wafer properties

Parameter                                                        Value    Units
L (wafer thickness), 300-mm-diameter wafer                         0.775    mm
L (wafer thickness), 200-mm-diameter wafer                         0.725    mm
L (wafer thickness), 150-mm-diameter wafer                         0.675    mm
L (wafer thickness), 100-mm-diameter wafer                         0.525    mm
Cp (silicon molar heat capacitance) @ 25 °C                        19.8     J/K-mole
Cp (silicon molar heat capacitance) @ 125 °C                       22.3     J/K-mole
r (silicon density)                                               2.33     g/cm3
Atomic weight of silicon                                         28.09    g/mole
kair (air thermal conductivity) at sea level, 20 °C                0.025    W/m-K
kair (air thermal conductivity) at sea level, 100 °C               0.031    W/m-K
h (convection heat transfer coefficient)                          5–10     W/m2-K
```

表中的 \(C_p\) 為莫耳熱容量；與質量密度搭配時，須利用 silicon 原子量轉換至一致基準。

### 表 5.4：ABC 典型值

```text
Typical values for resist ABC parameters

Resist Type        A (mm−1)    B (mm−1)               C (cm2/mJ)    Refractive Index
g-line (436 nm)    0.6         0.05                   0.015         1.65
i-line (365 nm)    0.9         0.05                   0.018         1.69
248 nm            0.0         0.50                   0.05          1.76
193 nm            0.0         1.20                   0.05          1.71
Dyed              Unchanged   Increased by 0.3–0.5   Unchanged     Approximately unchanged
```

### 習題指定資料與邊界條件

以下是練習設定，不是量測結果；尤其習題 5.10 的導數條件應核對原頁，未在此自行修正。

```text
5.3
A = 0.85 mm−1
B = 0.05 mm−1
C = 0.018 cm2/mJ
Refractive index = 1.72
thickness = 1.1 mm

5.7
95 °C, 300-second bake
100 °C, 60-second bake
105 °C, 17-second bake
activation energy = 31 Kcal/mole
ln(Ar) = 37 (1/minute)

5.9
C(z,0) = 0
C(0,t) = C0
C(∞,t) = 0

5.10
C(z,0) = 0
dC(0,t)/dt = 0
C(∞,t) = 0
∫₀∞ C(z,t) dz = Q0 = constant

5.11
300-mm silicon wafer
hot plate temperature = 100 °C
ambient temperature = 20 °C
proximity gap = 0.2 mm
comparison proximity gap = 0.3 mm

5.12
300-mm silicon wafer
hot plate temperature = 100 °C
proximity gap = 0.2 mm
equilibrium wafer temperature variation = ±0.2 °C

5.14
resist thickness = 0.75 mm
```

## 解讀限制

- Dill 模型依賴 Beer 定律、簡化反應機制與快速激發態穩態近似。
- 「PEB 溫度至少等於 PAB」是配方、溶劑與時間相依的經驗準則。
- Gaussian DPSF 推導假設固定 \(D\) 與無限域；有限膜厚或深度相依擴散率需要另行處理。
- 式 5.56 的自由體積在 \(T_g\) 連續、斜率改變；正文的「large discontinuity」不可直接解讀為擴散率數值跳躍。
- 式 5.81 擷取排版不清，本次不重建其 \(C\) 公式。
- Dill 吸收參數 \(B\) 與自由體積模型的 \(B\) 不同；不同段落的 \(D_0\) 也不可混用。

## 主要引用

- Dill, F.H., Hornberger, W.P., Hauge, P.S. and Shaw, J.M., 1975, Characterization of positive photoresist, IEEE Transactions on Electron Devices, ED-22, 445–452.
- Mack, C.A. and Carback, R.T., 1985, Modeling the effects of prebake on positive resist processing, Proceedings of the Kodak Microelectronics Seminar, 155–158.
- Doolittle, A.K., 1951, Studies in Newtonian flow. II. The dependence of the viscosity of liquids on free-space, Journal of Applied Physics, 22, 1471–1475.
- Fujita, H., Kishimoto, A. and Matsumoto, K., 1960, Concentration and temperature dependence of diffusion coefficients for systems polymethyl acrylate and n-alkyl acetates, Transactions of the Faraday Society, 56, 424–437.
- Mack, C.A., 1998, Modeling solvent effects in optical lithography, PhD Thesis, University of Texas at Austin.
- Walker, E.J., 1975, Reduction of photoresist standing-wave effects by post-exposure bake, IEEE Transactions on Electron Devices, ED-22, 464–466.