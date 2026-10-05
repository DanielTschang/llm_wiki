---
type: methodology
title: Hopkins 成像計算法
created: 2026-10-03
updated: 2026-10-03
tags: [成像計算, TCC]
related: [h-h-hopkins, abbe-成像計算法, sum-of-coherent-sources, 19-optical-lithography--8-02-ch2--k73j1g]
sources: ["optical-lithography/02 - ch2.pdf"]
---
# Hopkins 成像計算法

Hopkins 成像計算法先對光源積分，將來源與瞳孔組合為 transmission cross-coefficient（TCC），再與遮罩頻譜組合求影像。

## 為何重排積分

TCC 不依賴遮罩，固定光源及瞳孔時可以重用。因此它適合大量不同遮罩的反覆成像計算，例如 OPC，而不必每次重新逐來源點積分。

以一維及正規化來源 \(\tilde S\) 表示：

\[
TCC(f,g)=\int P(f+q)P^*(g+q)\tilde S(q)\,dq.
\]

\[
I(x)=\iint TCC(f,g)T_m(f)T_m^*(g)
e^{i2\pi(f-g)x}\,df\,dg.
\]

## 方法邊界

在相同模型及充分數值精度下，此表述與 [[abbe-成像計算法]] 等價。改變來源或瞳孔後須重新計算 TCC。

[[19-optical-lithography--8-02-ch2--k73j1g]] 支持核重用的數學理由，但未提供普遍的執行時間改善倍率。[[sum-of-coherent-sources]] 進一步使用核分解與截斷。