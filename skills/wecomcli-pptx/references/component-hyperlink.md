---
name: component-hyperlink
description: 超链接 href prop 规范 — Text / Picture / Box 整段挂链接，写入 OOXML hlinkClick
---

# 超链接（href）规范

`href` 是**跨组件的 prop**，不是独立组件。挂在 Text / Picture / Box 上，导出的 pptx 在 PowerPoint / 腾讯文档里点击可跳转。

## 支持的 URI

| 形式 | 示例 |
| :--- | :--- |
| 外链 | `https://docs.qq.com` / `http://...` |
| 邮件 | `mailto:foo@bar.com` |
| 跳页 action | `ppaction://hlinkshowjump?jump=nextslide` |
|            | `ppaction://hlinkshowjump?jump=previousslide` |
|            | `ppaction://hlinkshowjump?jump=firstslide` |
|            | `ppaction://hlinkshowjump?jump=lastslide` |

## 用法

### 整段 Text（写入 `rPr.hlinkClick`）

```jsx
<Text href="https://docs.qq.com" style={{ color: '#2563EB', textDecoration: 'underline' }}>
    docs.qq.com
</Text>

<Text href="mailto:slide@docs.qq.com" style={{ color: '#2563EB', textDecoration: 'underline' }}>
    slide@docs.qq.com
</Text>
```

`<Text href>` 给该 Text 内所有文字加链接。整段 Text 就是一个链接，无法只给其中一部分文字挂链接。

### Picture / Box 级（写入 `nvPr.hlinkClick`）

```jsx
<Picture src="hero_1" href="https://example.com" style={{ width: 400 }} />

<Box href="ppaction://hlinkshowjump?jump=nextslide" style={{ padding: 12, background: '#f1f5f9' }}>
    <Text>下一页</Text>
</Box>
```

⚠️ **`src` 的取值规则与 `<Image>` 相同**：写上传口返回的 `url`（见 [component-image.md](component-image.md)）。材料图可附加超链接。

## 硬性限制

- **段内文字链接不支持**。`<Text>` 内嵌 `<span href="...">` 目前不会写入 hlinkClick，编译时被忽略。需要链接就用整段 `<Text href>`；如果一句话里只有一部分是链接，把那一部分拆成独立的 `<Text href>`，与其他文字用 flex 排在一起
- `href` 值必须为字符串；空字符串等同未设置
- 一期不支持：`hrefTooltip`（悬浮提示）、`hrefSlide`（跳转到指定页）、hover link、SmartArt / Diagram 节点级 link
- 跨页跳转仅支持上表 4 个 `ppaction://hlinkshowjump?jump=...` 字面量，不支持指定第 N 页

## 使用建议

- 目录页 / 章节页 用 `ppaction://hlinkshowjump?jump=...` 做手动导航按钮
- 引用来源、参考资料用整段 `<Text href>`，方便观众追溯
- 装饰性元素**不加** `href`，避免误点
