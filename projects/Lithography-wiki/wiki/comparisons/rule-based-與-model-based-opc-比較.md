---
type: comparison
title: rule-based 與 model-based OPC 比較
created: 2026-10-03
updated: 2026-10-03
tags: [OPC, calibration, comparison]
related: [19-optical-lithography--9-10-ch10--y2hwiy, opc, model-based-opc-校準與迭代]
sources: ["optical-lithography/10 - ch10.pdf"]
---
# rule-based 與 model-based OPC 比較

兩種方法都修改光罩邊緣以補償 proximity effects，差別在於如何預測與決定修正量。

| 面向 | rule-based OPC | model-based OPC |
|---|---|---|
| 預測依據 | 經驗規則及參數表 | 校準的微影模型 |
| 主要資料需求 | CD-through-pitch、線端及 serif 量測 | 模型校準資料與實際設計模擬 |
| 修正方式 | 套用局部幾何規則 | 依 EPE 迭代移動 segments |
| 二維幾何 | 規則容易繁複 | 可針對實際形狀預測，但受模型精度限制 |
| 擴展瓶頸 | 精度提高導致規則數快速增加 | 全晶片速度、校準品質與模型誤差 |
| 複雜度控制 | 規則與 design grid | segment size、jog size 與 design grid |

## 比較界線

來源描述 130-nm 世代開始轉向 model-based OPC、90-nm 世代需要此方法，屬於 2007 年的歷史敘述。

兩種方法若只以名義焦距及曝光下的尺寸為目標，都不能自動保證共同製程窗口改善。model-based 不等於 process-window-aware；必須記錄真正的最佳化條件。

流程詳見 [[model-based-opc-校準與迭代]]；來源見 [[19-optical-lithography--9-10-ch10--y2hwiy]]。