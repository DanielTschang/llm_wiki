---
type: concept
title: Mask Error Enhancement Factor (MEEF)
created: 2026-10-03
updated: 2026-10-03
tags: [微影, MEEF, 誤差傳播]
related: [wilhelm-maurer, nils-與影像邊緣品質, original-mack-model, 低-k1-光阻對比度影響-meef, composite-cd-空間分解, 19-optical-lithography--8-08-ch8--1az2fh0]
sources: ["optical-lithography/08 - ch8.pdf"]
---
# Mask Error Enhancement Factor (MEEF)

Mask Error Enhancement Factor（MEEF）是晶圓光阻 CD 對 mask CD 的局部敏感度；mask CD 必須先換算為晶圓尺度：

\[
MEEF=\frac{\partial CD_{\mathrm{resist}}}{\partial CD_{\mathrm{mask}}}.
\]

zero-bias，即晶圓 CD 與換算後 mask CD 相等時，才可直接寫為對數導數。一般情況下：

\[
\frac{\partial\ln CD_{\mathrm{resist}}}{\partial\ln CD_{\mathrm{mask}}}
=\frac{CD_{\mathrm{mask}}}{CD_{\mathrm{resist}}}MEEF.
\]

## 必須保留的主體區分

- **image MEEF**：以固定強度閾值的空中像寬度估計。
- **光阻 MEEF**：包含有限對比度、曝光與顯影等實際光阻響應。
- **圖形特定 MEEF**：隨尺寸、pitch、duty cycle、照明及離焦改變。

因此單一 image MEEF 不應作為所有光阻與圖形的 mask 誤差放大率。

## 條件化結果

特定 coherent 三光束等線／間距模型，僅包含 0、±1 繞射階時，最佳焦距的 image MEEF 為 0.5；包含離焦可寫為 \(4/NILS\)。這不是一般光阻 MEEF 的固定值，亦不能無條件延伸至 isolated line。

小 contact hole 的近似假定 mask 誤差等向性，且像形主要由 PSF 決定。此時有效劑量約與 mask CD 四次方成正比；在本章使用的 zero-bias 對數形式下：

\[
MEEF\approx4\frac{\partial\ln CD_{\mathrm{wafer}}}{\partial\ln E}.
\]

非 zero-bias 時須恢復 CD 尺度比值。

## 製程用途

reticle CD map 配合圖形特定 MEEF 可預測晶圓誤差圖，支援 [[composite-cd-空間分解]]。OPC 可改善線性解析度，但本章指出一般不因此改善 MEEF。

[[低-k1-光阻對比度影響-meef]]顯示虛擬光阻對比度在低 \(k_1\) 時的重要性。

來源：[[19-optical-lithography--8-08-ch8--1az2fh0]]，§8.7。