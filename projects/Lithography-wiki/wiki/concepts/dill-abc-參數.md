---
type: concept
title: Dill ABC 參數
created: 2026-10-03
updated: 2026-10-03
tags: [曝光, 光阻, 吸收模型]
related: [dnq, 曝光互易性, abc-透射率全曲線擬合, 有效吸收係數, 光阻內成像, 19-optical-lithography--8-05-ch5--x0rq4s]
sources: ["optical-lithography/05 - ch5.pdf"]
---
# Dill ABC 參數

Dill ABC 參數以吸收與一階反應動力學，描述光阻曝光時感光成分消耗及光學性質變化。

\[
\alpha=Am+B,\qquad m=M/M_0,\qquad
\frac{\partial m}{\partial t}=-CIm.
\]

- \(A\)：可漂白吸收係數，單位為反長度。
- \(B\)：完全曝光後的不可漂白吸收係數，單位為反長度。
- \(C\)：曝光反應速率參數；當光強與劑量使用相應單位時，可使用 cm²/mJ。

\[
A=\alpha_{\mathrm{unexposed}}-\alpha_{\mathrm{exposed}},\qquad
B=\alpha_{\mathrm{exposed}},\qquad
C=\Phi\sigma_{M-\mathrm{abs}}\frac{\lambda}{hc}.
\]

\(A>0\) 表示漂白，\(A<0\) 表示變暗。量子產率 \(\Phi\) 是被吸收光子造成化學變化的比例，不能把所有吸收都視為有效反應。

## 耦合求解與潛像

若局部光強固定，初始 \(m=1\) 時有 \(m=\exp(-CIt)\)。漂白時，組成改變吸收，吸收又改變局部光強，因此需耦合求解。反射基板造成的駐波使問題更複雜，通常需要數值方法。

輸出 \(m(x,y,z)\) 是化學潛像，而不是顯影輪廓。這將[[光阻內成像]]與化學反應連接起來。

## 假設與量測限制

模型依賴 Beer 定律、吸收跨成分可加性、簡化一階反應及快速激發態穩態近似。漂白本身不代表[[曝光互易性]]失效。

[[abc-透射率全曲線擬合]]可取得參數，但 \(A=0\) 時此方法無法辨識 \(C\)。來源表 5.4 的吸收單位存在擷取疑點，應先核對原 PDF。

## 來源

[[19-optical-lithography--8-05-ch5--x0rq4s]]，第 5.1、5.5 節。