---
type: concept
title: PEB 擴散與 DPSF
created: 2026-10-03
updated: 2026-10-03
tags: [PEB, 擴散, 駐波]
related: [點擴散函數, 駐波與-barc, swing-curves, 修正-fujita-doolittle-模型, peb-擴散與-barc-的駐波抑制比較, 19-optical-lithography--8-05-ch5--x0rq4s]
sources: ["optical-lithography/05 - ch5.pdf"]
---
# PEB 擴散與 DPSF

PEB 擴散是曝光後烘烤期間分子濃度重新分布的過程。DPSF 是擴散的脈衝響應；在固定擴散係數與無限域條件下為 Gaussian 核。

\[
\frac{\partial C}{\partial t}=\nabla\cdot(D\nabla C),\qquad
\sigma=\sqrt{2Dt}.
\]

三維固定 \(D\) 時：

\[
DPSF(\mathbf r)=\frac{1}{(2\pi\sigma^2)^{3/2}}
\exp\left(-\frac{|\mathbf r|^2}{2\sigma^2}\right),\qquad
m^*=m\otimes DPSF.
\]

其數學形式類似[[點擴散函數]]，但物理機制是分子移動，不是光學成像。

## 平滑與解析度的權衡

傳統光阻中的 PAC 擴散可平滑垂直駐波，卻也模糊橫向潛像。擴散長度應接近或超過半個駐波週期，同時遠小於最小特徵尺寸。

來源以 i-line 半週期約 55 nm、248 nm 約 35–40 nm 說明此權衡；這些是教材時期的典型尺度。PROLITH 的 i-line 模擬使用 20、40、60 nm 擴散長度展示輪廓變化。

## 適用邊界

- 有限膜厚、阻擋表面及深度相依 \(D\) 可能需要不同解法。
- 殘餘溶劑與溫度改變擴散率，固定 \(D\) 只是近似。
- 化學放大型光阻的酸擴散不是 DNQ 的 PAC 擴散。
- PEB 平滑不處理 [[swing-curves]]；比較見[[peb-擴散與-barc-的駐波抑制比較]]。

## 來源

[[19-optical-lithography--8-05-ch5--x0rq4s]]，第 5.3 節。