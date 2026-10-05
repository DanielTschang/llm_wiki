---
type: concept
title: swing curves
created: 2026-10-03
updated: 2026-10-03
tags: [光阻, 薄膜干涉, 線寬控制]
related: [19-optical-lithography--8-04-ch4--ccwms, 駐波與-barc, tarc, barc-穩健最佳化, swing-ratio-極值厚度差是否為半週期]
sources: ["optical-lithography/04 - ch4.pdf"]
---
# swing curves

swing curves 是微影指標隨光阻厚度呈近似週期性變化的曲線，常見指標包括 CD、dose-to-clear \(E_0\) 與塗有光阻的晶圓反射率。其起因是上表面直接反射與經光阻往返的反射場干涉。

## 與駐波的區別

[[駐波與-barc]] 著重固定厚度下的深度強度分布；swing curves 著重改變厚度後的能量耦合。對均勻膜、單色平面波：

\[
P=\frac{\lambda}{2n_2\cos\theta_2}.
\]

正入射時 \(\cos\theta_2=1\)。反射幅值與相位、入射角、偏振及吸收皆會影響曲線。

## 指標與抑制方式

來源定義：

\[
A_{\mathrm{swing}}=4|\rho_{12}\rho_{23}|e^{-\alpha D},\qquad
SR=\frac{E_{0,\max}-E_{0,\min}}
{(E_{0,\max}+E_{0,\min})/2}.
\]

BARC 降低底部反射；[[tarc]] 降低頂部有效反射。增加吸收可降低干涉，但加大曲線的整體斜率。相鄰極值的吸收斜率可能抵消振盪，令某一對極值的 SR 接近零；這不代表整條曲線平坦或反射干涉消失。

原文 SR 推導的厚度差定義有疑點，吸收修正公式應待 [[swing-ratio-極值厚度差是否為半週期]] 核對後再使用。

## 角度與圖案依賴

部分同調照明的角度分布會平均不同週期與相位的響應。第 4.2.4 節的近似模型在 193 nm、NA = 0.93、\(\sigma=0.7\)、300-nm 光阻下，預測 SR 降低約 23%；這是模型結果，不是實測普遍比例。

開放場 \(E_0\) 曲線主要反映零階光，不能直接代表所有圖案的 CD 曲線；不同繞射階次可能處於相反的 swing 相位。

## 來源

見 [[19-optical-lithography--8-04-ch4--ccwms]] 第 4.2 節。反射率規格應由圖案、厚度變動及劑量誤差預算決定，而非只設定固定低反射率門檻。