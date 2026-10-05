---
type: concept
title: overlay 控制
created: 2026-10-03
updated: 2026-10-03
tags: [微影, overlay, 量測]
related: [aim, overlay-fingerprint-表徵, overlay-與-pattern-placement-error-比較, adi-與重工, 19-optical-lithography--8-08-ch8--1az2fh0]
sources: ["optical-lithography/08 - ch8.pdf"]
---
# overlay 控制

overlay 控制管理新微影層相對晶圓既有圖形的位置誤差。registration 則衡量圖形相對絕對座標網格的位置，兩者不可混用。

## 量測偏差

box-in-box 以兩側間隙差的一半估計位置差，可抵消對稱 CD 變化，不能抵消不對稱。[[aim]]利用多條帶改善精密度與製程穩健性。

本章採用 \(TIS=Overlay(0^\circ)+Overlay(180^\circ)\) 的定義。固定 TIS 可校正，其變動才增加不確定度。WIS 是標靶不對稱造成的晶圓誘發位移，且可能與量測焦距交互作用，不保證可用固定偏移修正。

## 模型及可識別性

基本線性模型包含晶圓與場內各兩個旋轉、倍率項，加上共用的兩個平移項，共 10 個係數。平移無法單從此模型分離為晶圓與 reticle 來源。

backlash 加入兩個隨掃描方向改號的平移項。高階畸變、reticle registration 及 stage signature 則需額外表徵；見 [[overlay-fingerprint-表徵]]。殘差不能未经檢查便視為純隨機。

## 批次處置

overlay 資料可用於批次放行、曝光工具設定修正及量測診斷。稀疏量測點的最大值未必代表未取樣區域；較有意義的做法是结合模型與誤差估計，預測每個 die 的 overlay-limited yield，再比較良率損失與重工成本。

可修正項是對已處理批次的回溯估計；用於下一批次的 APC 仍依賴誤差的時間穩定性。與圖形特定位置誤差的區分見 [[overlay-與-pattern-placement-error-比較]]。

來源：[[19-optical-lithography--8-08-ch8--1az2fh0]]，§8.4。