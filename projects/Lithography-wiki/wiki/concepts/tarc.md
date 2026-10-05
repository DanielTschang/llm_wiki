---
type: concept
title: TARC
created: 2026-10-03
updated: 2026-10-03
tags: [抗反射塗層, 光阻, 薄膜干涉]
related: [19-optical-lithography--8-04-ch4--ccwms, swing-curves, 駐波與-barc, barc-與-tarc-比較]
sources: ["optical-lithography/04 - ch4.pdf"]
---
# TARC

TARC 是置於光阻上方的頂部抗反射塗層，透過降低上表面的有效反射，減少 [[swing-curves]] 並提高光線耦合進光阻的效率。

## 理想設計

對正入射、透明介質及非吸收塗層，四分之一波設計為：

\[
n_{\mathrm{TARC}}=\sqrt{n_{\mathrm{upper}}n_{\mathrm{resist}}},
\qquad
D_{\mathrm{TARC}}=\frac{\lambda}{4n_{\mathrm{TARC}}}.
\]

空氣上方介質、光阻折射率 1.7、波長 193 nm 的理想值約為折射率 1.30、厚度 37 nm。

## 限制

真實光阻有吸收，理想完全零反射解不再嚴格成立。來源指出弱吸收的影響通常較小；更重要的是材料折射率限制及角度／偏振分布。來源所述多數可用材料折射率大於 1.4，屬 2007 年背景，不能當作現行材料清單。

TARC 降低頂部反射，不會自動消除底部反射場形成的駐波，也不具 BARC 對 reflective notching 的相同作用。兩者見 [[barc-與-tarc-比較]]。

## 來源

[[19-optical-lithography--8-04-ch4--ccwms]] 第 4.4 節。