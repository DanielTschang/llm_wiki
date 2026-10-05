---
type: concept
title: attenuated PSM
created: 2026-10-03
updated: 2026-10-03
tags: [optical-lithography, PSM, EPSM, sidelobes]
related: [19-optical-lithography--9-10-ch10--y2hwiy, psm, alternating-psm, opc, oai, sraf, 自然解析度]
sources: ["optical-lithography/10 - ch10.pdf"]
---
# attenuated PSM

attenuated PSM 是让原本遮光區透射少量、且相對透明區具有約 180° 相位差的 phase-shifting mask，亦稱 embedded PSM（EPSM）或 half-tone PSM。

## 成像機制與透射率

相移背景降低零階並提高一階的相對貢獻，改善邊緣影像。來源的等線空、理想同調 TE 雙光束模型中，兩繞射階振幅相等時 NILS 最大，對應相對強度透射率約 4.93%。此數值不是所有圖案及照明的通用最佳值。

模型中的透射率以透明基板為基準。絕對強度透射率為 6% 的商用 blank，在來源所列基板透射率下，相對強度透射率約為：

| 波長 | 基板強度透射率 | EPSM 相對強度透射率 |
|---|---:|---:|
| 248 nm | 92% | 6.5% |
| 193 nm | 91% | 6.6% |

電場振幅透射率與強度透射率也必須分開記錄。

## sidelobes 與輔助槽

Figure 10.33 的 6% EPSM 孤立空間案例產生約 17% sidelobe 強度，不能僅以背景材料的 6% 透射率估計誤印風險。

適當配置的非列印 assist slots 可藉相消干涉降低 sidelobe 強度。參見 [[sraf]]。

## 相位誤差與流程整合

來源的週期等線空分析中，小相位誤差主要改變最佳焦點。6% EPSM 的 10° 相位誤差在部分同調條件下，可能使最佳焦點移動約 DOF 的 10%，並造成約 20% 的潛在 DOF 損失；此為條件依賴估計。

來源的 EPSM 案例沒有 [[alternating-psm]] 式 through-focus 位置變化，不能外推至任意圖案。跨 reticle 的相位變動尤其值得注意，因為平均焦點校正不能消除所有局部焦點差異。

EPSM 可藉重新校準 OPC 模型接入既有流程；本章對其採用程度的描述以 2007 年為界。來源見 [[19-optical-lithography--9-10-ch10--y2hwiy]]。