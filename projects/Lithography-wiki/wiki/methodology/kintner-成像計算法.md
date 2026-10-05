---
type: methodology
title: Kintner 成像計算法
created: 2026-10-03
updated: 2026-10-03
tags: [部分同調, 週期圖案, 幾何權重]
related: [e-c-kintner, abbe-成像計算法, 19-optical-lithography--8-02-ch2--k73j1g]
sources: ["optical-lithography/02 - ch2.pdf"]
---
# Kintner 成像計算法

Kintner 成像計算法對只涉及零級與正負一級繞射的小節距線／空間图案，將來源積分化為一、二、三束成像的權重問題。

## 幾何化的理由

當来源均勻且同一區域內收集到相同繞射級次時，各來源點產生相同類型的影像。因此可用來源與移動瞳孔的重疊面積，取代逐點積分：

\[
I=s_1I_{\text{1-beam}}+s_2I_{\text{2-beam}}+
s_3I_{\text{3-beam}},\qquad s_1+s_2+s_3=1.
\]

## 適用範圍

[[19-optical-lithography--8-02-ch2--k73j1g]] 的 Table 2.3 保存傳統圓盤來源權重，Table 2.4 保存薄環形來源近似權重。

兩表的 \(\gamma\) 具有不同定義，不能混用。權重公式依賴來源形狀、節距及繞射級次限制，不能直接套到任意二維圖案或非均勻來源。式 (2.91)–(2.93) 與 (2.107) 的轉錄仍需回查 PDF 才能可靠實作。