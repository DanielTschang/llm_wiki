---
type: concept
title: 修正 Fujita–Doolittle 模型
created: 2026-10-03
updated: 2026-10-03
tags: [溶劑, 擴散, 自由體積, PAB]
related: [novolac, 光阻烘烤, peb-擴散與-dpsf, 19-optical-lithography--8-05-ch5--x0rq4s]
sources: ["optical-lithography/05 - ch5.pdf"]
---
# 修正 Fujita–Doolittle 模型

修正 Fujita–Doolittle 模型以聚合物自由體積描述溶劑擴散率，並在較高溶劑濃度下納入聚合物自由體積被稀釋的效應。

\[
D=D_0\exp[-B(1/v_f-1/v_g)],
\]
\[
v_f=(1-\phi_s)[v_g+\alpha_2(T-T_g)]+\phi_s\beta.
\]

其中 \(\phi_s\) 為溶劑體積分率，\(v_g\) 為乾燥聚合物在 \(T_g\) 的自由體積分率；\(D_0\) 是無溶劑、\(T=T_g\) 時的參考擴散率。此處 \(B\) 不是 Dill 吸收參數。

## 物理意義

Doolittle 將黏度與自由體積相連；Fujita 等人進一步加入溶劑造成的自由體積。修正式在溶劑增加時，同時降低聚合物部分的貢獻，改善高濃度範圍的描述。

本章估計典型 PAB 過程中擴散率可變動四至六個數量級。膜表面較乾燥，可能使後續 PEB 的分子擴散較弱；化學放大型光阻中的酸擴散應與傳統光阻中的 PAC 擴散分開記錄。

## 模型預測與限制

固定烘烤條件下，相同最終膜厚可對應近乎相同的殘餘溶劑分布；這是模型預測，不是普遍量測定律。

來源採用的玻璃轉移近似在 \(T_g\) 以下固定聚合物自由體積，以上則增加。式 5.56 在 \(T_g\) 連續但斜率改變，因此正文「large discontinuity」尚不能直接解讀為 \(D\) 的數值跳躍。高於 \(T_g\) 的溫度依賴也不一定服從單一 Arrhenius 直線。

典型參數原表保存在[[19-optical-lithography--8-05-ch5--x0rq4s]]，不應當作所有光阻配方的固定值。