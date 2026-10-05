---
type: finding
title: 圖案倒塌先於 CD 規格失效
created: 2026-10-03
updated: 2026-10-03
tags: [圖案倒塌, 製程窗口, 模型]
related: [圖案倒塌, tanaka-t, 焦距與曝光製程窗口]
sources: ["optical-lithography/08 - ch8.pdf"]
source: "[[19-optical-lithography--8-08-ch8--1az2fh0]]"
confidence: medium
replicated: null
---
# 圖案倒塌先於 CD 規格失效

## 直接證據

本章 §8.10 的雙線機械模型示例指出：45-nm 等線／間距、141-nm 光阻膜厚的圖形，可在過曝光造成 CD 誤差達 10% 前倒塌。

這是模型推算示例，不是本章新報告的量產失效率或具名光阻實測。

## 工程推論

[[焦距與曝光製程窗口]]若僅使用 CD、sidewall angle 與 resist loss 邊界，可能高估可製造範圍。機械穩定性亦應作為窗口限制。

## 適用邊界

[[圖案倒塌]]模型假定長線雙線近似最壞情況；短線支撐、黏著、清洗液及材料剛性都可能改變閾值。不得將 45-nm／141-nm 組合視為所有 ArF 光阻的通用失效界線。

來源未提供此示例的獨立重現資料。