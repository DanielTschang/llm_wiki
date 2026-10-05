---
type: concept
title: 潛像梯度與 chemical contrast
created: 2026-10-03
updated: 2026-10-03
tags: [optical-lithography, latent-image, PEB]
related: [19-optical-lithography--8-09-ch9--1xlwz30, nils-與影像邊緣品質, rdpsf, base-quencher, 光阻理論對比度與量測對比度]
sources: ["optical-lithography/09 - ch9.pdf"]
---
# 潛像梯度與 chemical contrast

潛像梯度（LIG）是光阻內化學物種濃度在圖案邊緣附近的空間導數；chemical contrast 則表徵光學對數梯度轉換為化學潛像梯度的效率。兩者描述曝光與 PEB 後的化學影像，不等同於顯影速率的光阻對比度。

## 曝光的轉換效率

對一階曝光且不漂白的模型：

\[
m=e^{-CI_rt},\qquad
\frac{\partial m}{\partial x}=m\ln m\;ILS.
\]

\(m\) 為剩餘光敏材料相對濃度。梯度大小的轉換係數 \(-m\ln m\) 在 \(m=e^{-1}\) 最大；零曝光與完全曝光皆形成均勻濃度，梯度趨近零。

這說明提高 [[nils-與影像邊緣品質|NILS]] 有利於潛像，但相同光學影像在不同劑量下仍可能有不同化學品質。

## 化學放大後的定義

章節定義：

\[
\text{chemical contrast}
=\frac{\partial m^*/\partial x}{\partial\ln I/\partial x}
=\frac{\partial m^*}{\partial\ln E}.
\]

此為帶符號導數；比較品質時可使用其大小。\(m^*\) 為 PEB 後剩餘 blocked polymer 相對濃度，而非已去保護部分。

忽略酸損失時：

\[
m^*=e^{-\alpha_fh_{\mathrm{eff}}},
\qquad \alpha_f=K_{\mathrm{amp}}t_{\mathrm{PEB}}.
\]

\(h_{\mathrm{eff}}\) 由 [[rdpsf]] 給出時間平均有效酸分布。反應—擴散竞争參數為：

\[
\eta=\frac{\pi^2D}{L^2K_{\mathrm{amp}}}.
\]

同一材料的 \(D/K_{\mathrm{amp}}\) 不變時，縮小特徵尺度 \(L\) 會增大 \(\eta\)，使擴散更易破壞梯度。

## 與顯影的連結

\[
\frac{\partial\ln r}{\partial x}
=\frac{\partial\ln r}{\partial m^*}
\frac{\partial m^*}{\partial x}.
\]

因此最大 LIG 不一定等於最大顯影速率對數梯度：還須讓名義邊緣濃度落在顯影函數最有辨識力的區域。最大梯度也不保證最小 LER，因為隨機濃度波動同樣影響邊緣位置。

以上關係的推導與限制見 [[19-optical-lithography--8-09-ch9--1xlwz30]]。