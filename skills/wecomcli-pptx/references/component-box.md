---
name: component-box
description: Box 容器组件规范 — Flex 布局、盒模型、常用样式属性、限制条款
---

# Box 组件规范

Box 是本 DSL 中最基础的容器组件，承担所有**布局与分组**职责。支持 Flexbox 布局，可任意嵌套，是构建页面结构的基本单元。

## 默认布局

```jsx
<Box>...</Box>
// 等价于
<Box style={{ display: 'flex', flexDirection: 'column' }}>...</Box>
```

- 默认 `display: 'flex'`，`flexDirection: 'column'`（纵向流）
- 未指定 `width` / `height` 时，尺寸由子组件与父容器共同决定

## 盒模型

**遵循 `box-sizing: border-box`**：设置 `width` 时已包含 padding 和 border，内容可用空间 = `width - padding - border`。

例如：`<Box style={{ width: 400, padding: 20, border: '2px solid #333' }}>` 内部子组件可用宽度 = 400 − 20×2 − 2×2 = **356px**。

## 常用样式

| 类别 | 属性 |
| :--- | :--- |
| 布局 | `flexDirection` / `justifyContent` / `alignItems` / `gap` / `flex` / `flexWrap` |
| 尺寸 | `width` / `height` / `minWidth` / `minHeight` / `maxWidth` / `maxHeight` |
| 间距 | `padding` / `margin`（及其方向变体） |
| 视觉 | `background` / `border` / `borderRadius` / `boxShadow` / `opacity` |
| 定位 | `position: 'absolute' / 'relative'` / `top` / `left` / `right` / `bottom` / `zIndex` |

## 硬性限制（必须遵守）

1. **仅支持 `display: 'flex'`**。严禁 `display: 'grid'`、`display: 'table'` 以及任何 grid 相关属性（`gridTemplateColumns` 等）
2. **不支持 CSS `calc()` 函数**。需要动态尺寸时改用 `flex: 1` / 百分比 / 精确计算的固定值
3. **容器只用 `<Box>`，禁止 `<View>`**——`<View>` 是 React Native 组件，本 DSL 不存在；所有布局容器一律 `<Box>`
4. 子组件类型可以是 `Text` / `FAIcon` / `Image` / `Chart` / `Table` / `Box`（嵌套）等任意本 DSL 组件

## 使用示例

### 横向排列（图标 + 文字组合）

```jsx
<Box style={{
    flexDirection: 'row',
    alignItems: 'center',
    gap: 20,
    padding: 20,
    background: '#f0f4f8',
    borderRadius: 8,
}}>
    <FAIcon name='info-circle' style={{ fill: '#3b82f6', width: 24, height: 24 }} />
    <Box>
        <Text style={{ fontSize: 18, fontWeight: 'bold' }}>标题</Text>
        <Text style={{ fontSize: 14, color: '#64748b' }}>描述文本</Text>
    </Box>
</Box>
```

### Flex 纵向均分（内容卡片组）

```jsx
<Box style={{
    display: 'flex',
    flexDirection: 'column',
    gap: 24,
    height: '100%',
    justifyContent: 'space-between',
}}>
    <Box style={{ flex: 1 }}>...卡片 1...</Box>
    <Box style={{ flex: 1 }}>...卡片 2...</Box>
    <Box style={{ flex: 1 }}>...卡片 3...</Box>
</Box>
```

### 绝对定位装饰元素

```jsx
<Box style={{ position: 'relative', width: '100%', height: '100%' }}>
    {/* 背景装饰 */}
    <Box style={{
        position: 'absolute',
        top: 0, right: 0,
        width: 200, height: 200,
        background: 'linear-gradient(135deg, #3b82f6 0%, transparent 100%)',
        opacity: 0.1,
    }} />
    {/* 主内容 */}
    <Box style={{ position: 'relative', zIndex: 1 }}>...</Box>
</Box>
```

## 最佳实践

- 页面内**每个视觉区块**用独立的容器 Box 包裹，必要时加背景（纯色或半透明）增强分区感
- 卡片组用 **Flex + `flex: 1`** 均分空间，而非固定高度堆叠
- 布局、留白与信息密度规范见本次任务的 `DESIGN.md`（Step4 产出的设计契约）

## 扩展能力

- 容器超链接（`<Box href="...">`，含跳页 action）→ [component-hyperlink.md](component-hyperlink.md)
- 入场 / 退出 / 强调动画（`<Animation>` 包裹）→ [component-animation.md](component-animation.md)
