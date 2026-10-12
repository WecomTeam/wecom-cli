---
name: component-qrcode
description: QRCode component - text must be real accessible URL, size and color config
---

# QRCode 组件规范

用于将文本转换为 QR Code SVG，text 必须是真实可访问链接（例如 https://docs.qq.com）。

1. 原则：生成前需验证链接真实有效且与当前内容主题一致。
2. 原则：二维码旁须配套说明文案，明确扫码后的用途与价值。
3. 原则：如需叠加品牌元素或背景纹理，先规划视觉层级与容错策略。
4. 原则：建立二维码链接与使用场景的记录，便于追踪与复用。

## 用法示例

```jsx
<QRCode
    text='https://docs.qq.com/sheet/HIJKLMN' // 必填真实链接；此处仅为示例，需替换为当前素材的真实地址
    width={240} // 二维码边长；height 自动等于 width，保持 150-300 区间
    darkColor='#000' // 模块前景色，确保与背景形成高对比
    lightColor='#fff' // 背景色，应避免透明或花纹
    errorCorrectionLevel='Q' // L/M/Q/H 纠错等级，默认 M，可按场景提升
/>
```

注意：text 永远不要使用不可访问的链接或无效占位符。
