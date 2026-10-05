---
type: concept
title: base quencher
created: 2026-10-03
updated: 2026-10-03
tags: [中和, 光阻化學, 隨機變異]
related: [photoacid-generator, 化學放大型光阻, 光阻化學隨機建模, 19-optical-lithography--8-06-ch6--1wif8]
sources: ["optical-lithography/06 - ch6.pdf"]
---
# base quencher

base quencher 是刻意加入化學放大型光阻的鹼，用來中和低濃度酸，形成放大反應閾值並限制酸向名義未曝光區域擴散的影響。

## 快速中和近似

2007 年來源所述 quencher loading 約為初始 PAG loading 的 5–20%。若中和遠快於放大且忽略擴散，令 \(h\) 為中和前酸濃度、\(q_0\) 為初始正規化鹼濃度：

\[
m=
\begin{cases}
e^{-K_{\mathrm{amp}}t_{\mathrm{PEB}}(h-q_0)}, & h>q_0,\\
1, & h\leq q_0.
\end{cases}
\]

此時主要效果是劑量偏移，而不是提高相同劑量下的敏感度。

## 邊緣與隨機限制

Figure 6.5 對含與不含 quencher 的光阻各自調整至 dose-to-size，顯示邊緣附近潛像斜率可改善。這不是直接量測 LER 改善的證據。

酸與鹼各自擴散會改變中和邊界；其移動方向取決於擴散係數與潛像形狀。鹼分子數較少也可能帶來更大的計數變異，中和的 annihilation 性質可能引入酸／鹼聚集與低頻 LER。本章未建立此隨機模型。

來源：[[19-optical-lithography--8-06-ch6--1wif8]]，§6.2.4、§6.4.8。