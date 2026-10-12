---
name: component-text
description: Text 文本组件规范 — 富文本内联、换行、渐变文字、map 映射
---

# Text 组件规范

Text 是本 DSL 中承载所有**文字内容**的组件。支持基础样式、富文本内联（加粗/变色/下划线）、渐变文字、阴影等高级样式。

## 基础用法

```jsx
<Text style={{ fontSize: 20, color: '#0F172A', lineHeight: 1.6 }}>
    这是一段标准文本
</Text>
```

## 富文本内联

Text 内部允许使用 `<span>` 对部分字符做局部样式覆盖（加粗、变色、下划线等）。

```jsx
<Text style={{ fontSize: 20, color: '#0F172A', lineHeight: 1.6 }}>
    这是 <span style={{ fontWeight: 'bold' }}>加粗</span> 和
    <span style={{ color: '#2563EB' }}>蓝色强调</span>
    <br />
    第二行内容
</Text>
```

**关键规则**：
- **换行**必须用 `<br />`（小写、自闭合）。不要用 `\n`
- **局部样式**用 `<span style={{...}}>...</span>` 包裹
- **不支持** `<strong>` / `<em>` / `<u>` 等 HTML 语义标签，一律用 `<span>` + inline style

## 渐变文字

通过 `backgroundClip: 'text'` + `backgroundImage` 实现。

```jsx
<Text style={{
    fontSize: 48,
    fontWeight: 'bold',
    backgroundImage: 'linear-gradient(135deg, #3b82f6 0%, #8b5cf6 100%)',
    backgroundClip: 'text',
    color: 'transparent',
}}>
    渐变标题
</Text>
```

## 文字阴影

```jsx
<Text style={{
    fontSize: 32,
    color: '#ffffff',
    textShadow: '2px 2px 8px rgba(0, 0, 0, 0.3)',
}}>
    带阴影的白字
</Text>
```

## 在 map 映射中使用富文本

当通过数组 `.map()` 渲染多条 Text 时，若条目内含富文本，必须用 Fragment `<>...</>` 包裹（因为 `<span>` 与外层 Text 的结构需要显式分组）：

```jsx
<Box style={{ flexDirection: 'column', gap: 12 }}>
    {items.map((item, idx) => (
        <Text key={idx} style={{ fontSize: 16 }}>
            <>
                <span style={{ fontWeight: 'bold' }}>{item.label}</span>：
                {item.desc}
            </>
        </Text>
    ))}
</Box>
```

## 常用样式

| 类别 | 属性 |
| :--- | :--- |
| 字号字重 | `fontSize` / `fontWeight`（`'normal' / 'bold' / 100-900`） |
| 颜色 | `color` / `opacity` |
| 间距 | `lineHeight`（推荐 1.5-1.6）/ `letterSpacing` |
| 对齐 | `textAlign`（`'left' / 'center' / 'right'`） |
| 装饰 | `textDecoration`（`'underline' / 'line-through'`） / `textShadow` |
| 溢出 | `whiteSpace` / `overflow` / `textOverflow` |

## 最佳实践

- **仅在需要视觉层次时**切换字号 / 字重，同一层级保持统一
- 文字颜色优先使用项目 `DESIGN.md` 定义的文本色、主色与辅助色，避免自创新色
- 字号、行长、段落长度与文字密度规则见本次任务的 `DESIGN.md`（Step4 产出的设计契约）

## 扩展能力

- 超链接（整段 `<Text href="...">`，段内 span 不支持）→ [component-hyperlink.md](component-hyperlink.md)
- 入场 / 退出 / 强调动画（`<Animation>` 包裹）→ [component-animation.md](component-animation.md)
