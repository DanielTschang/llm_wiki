---
type: comparison
title: alternating PSM 與 attenuated PSM 比較
created: 2026-10-03
updated: 2026-10-03
tags: [PSM, phase-errors, comparison]
related: [19-optical-lithography--9-10-ch10--y2hwiy, alternating-psm, attenuated-psm, psm]
sources: ["optical-lithography/10 - ch10.pdf"]
---
# alternating PSM 與 attenuated PSM 比較

| 面向 | alternating PSM | attenuated PSM |
|---|---|---|
| 相位配置 | 相鄰透明空間相差 180° | 弱透射背景與透明區相差 180° |
| 理想機制 | 消除零階，形成對稱一階雙光束 | 調整零階與一階相對振幅，改善邊緣 |
| 版圖限制 | 可能出現 phase conflicts | 沒有 Alt-PSM 的同類相位指派衝突 |
| 光罩誤差 | spacewidth 依賴的相位與強度不平衡 | 仍需控制透射率與相位 |
| 主要誤印問題 | 非預期 phase edges | sidelobes |
| 小相位誤差的來源案例 | 與 defocus 耦合造成線位置偏移 | 主要造成最佳焦點位移 |
| 流程負擔 | 相位配置與製造較複雜，部分方案需雙曝光 | 可透過重新校準 OPC 模型接入既有流程 |

## 解讀限制

[[alternating-psm]] 的理想解析度與 DOF 優勢不能忽略相位衝突及光罩三維效應。[[attenuated-psm]] 的整合優勢也不代表 sidelobes 或局部相位變化可忽略。

EPSM 的 4.93% 最佳相對強度透射率只適用來源的等線空理想雙光束模型。兩種光罩的相位容差應依 CD、位置誤差與共同製程窗口分別評估，不宜套用單一角度規格。

所有比較出自 [[19-optical-lithography--9-10-ch10--y2hwiy]]；採用程度的描述僅代表 2007 年觀點。