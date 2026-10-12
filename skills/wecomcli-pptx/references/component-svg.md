---
name: component-svg
description: SVG component - inline vector graphics, viewBox, path, shapes
---

# SVG 组件规范

SVG 组件允许直接使用 `<svg>` 标签渲染矢量图形，适合自定义图标、装饰形状或复杂的矢量图。

## 核心属性

- **width / height**: 必须指定。
- **viewBox**: 定义画布的坐标系。
- **children**: 内联 SVG 元素（如 `<rect>`, `<circle>`, `<path>`, `<g>` 等）。

## 布局与样式

- **默认布局**: `display: 'block'`。
- **支持属性**: `fill`, `stroke`, `strokeWidth`, `opacity` 等标准 SVG 属性。
- **注意事项**:
    - 建议使用 `viewBox` 以确保图形在不同尺寸下正确缩放。

## 示例代码

### 1. 基础用法 (简单形状)

```jsx
<svg width={100} height={100} viewBox='0 0 100 100'>
    <circle cx='50' cy='50' r='40' fill='#3b82f6' />
</svg>
```

### 2. 综合示例 (复杂路径)

```jsx
<svg width={24} height={24} viewBox='0 0 24 24'>
    <path d='M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5' fill='none' stroke='#64748b' strokeWidth='2' />
</svg>
```

## `<svg>` 内禁止 `&`（硬规则，一个字符毁整张图）

导出时 `<svg>` 里的 `&amp;` 会被反转义成裸 `&`，svg 不再是合法 XML，**整张图**空白——不是少一行字，是全图没了。这种页连 PNG 兜底都不会生成，导出后也无法修补。

- **正确写法**：`<svg>` 内不要出现 and 符号，改成「与」/「and」/「·」。
- **绕法全部无效**：`&amp;` / `&#38;` / `&#x26;` / `{'\u0026'}` 导出后都是同一个裸 `&`。
- **`<svg>` 外不受影响**：`<Text>` 组件里的 `&` 正常显示，不用改。
- **插入接口不会报错**（返回 `error_msg:""`），别把插入成功当成图能显示。

```jsx
{/* ✗ 整张图变空白 */}
<text x='40' y='180' fontSize='18' fill='#1E4FA8'>Lyapunov &amp; Kaplan-Yorke</text>

{/* ✓ */}
<text x='40' y='180' fontSize='18' fill='#1E4FA8'>Lyapunov 与 Kaplan-Yorke</text>
```

## 文字必须留在 viewBox 内（硬规则）

svg 内的文字超出 viewBox 会被**硬裁**（`16-core CPU / 40-core GPU` 只剩 `16-core CP`），而页面布局的溢出校验**不看 svg 内部**，不会报错。只能在写坐标时自己算。

**宽度估算**：`宽 ≈ 字数 × fontSize × 系数`，中日韩字符系数 **1.0**，英文数字标点 **0.55**。

```
x + 估算宽 ≤ viewBox 宽         textAnchor 默认 start
x ± 估算宽/2 落在 [0, viewBox宽] textAnchor='middle'
x − 估算宽 ≥ 0                  textAnchor='end'
```

例：`<text x='470' fontSize='11'>16-core CPU / 40-core GPU</text>` 在 `viewBox='0 0 520 260'` 里 —— 25 字 × 11 × 0.55 ≈ 137，470+137 = 607 > 520，**溢出 87，会被裁**。

画图表时：

- 柱状图标签放在柱子右端 +8 处，先按**最长的那条标签**反推柱子最大长度，别让柱子占满 viewBox。
- 右侧统一预留 `最长标签字数 × fontSize × 0.55` 的空白带。
- 缩放柱子要**改整体刻度**（每单位多少像素），**不能只缩最长那根** —— 那会让柱长不再等比于数值，图表就开始说谎（40 核的柱子比 20 核短）。
- 改完核对：同组柱子的 `长度 ÷ 数值` 必须全部相等。

## 企微导出注意（硬规则）

企微把在线 PPT 导出为本地 `.pptx` 时，每个 `<svg>` 会同时存一份矢量和一张栅格化的 PNG 兜底；PowerPoint 2016+ 读矢量，Keynote / Quick Look / 预览类查看器读 PNG，而栅格化**不保留透明**——透明区域会变成白色方块。所以：

> **前置条件**：PNG 兜底只在 svg 是合法 XML 时才生成。svg 内含 `&` 时兜底根本不会产出，本节规则全部无从生效 —— 先按上面清掉 `&`。

- `<svg>` 的第一个绘制子元素（`<defs>` 之后）必须是覆盖整个 viewBox 的实底矩形，填它所在区域的底色：`<rect x='0' y='0' width='W' height='H' fill='#底色' />`。
- 整个 SVG 要落在一块纯色区里，不跨渐变、不跨两种底色；整页纹理直接让 SVG 自带页底色。
- 不在 `<svg>` 的 `style` 上写 `opacity`，淡色按比例预混进色值；不写 `rgba()`，用 hex 加 `fillOpacity` / `strokeOpacity`。`rgba()` 不会让整图空白，风险是兜底位图丢色，严重度低于 `&`。
- `build_page.py contrast`（`buildall` 已含）会拦截：缺实底矩形、svg 内的 `&`、svg 文字超出 viewBox。

```jsx
<svg width={360} height={330} viewBox='0 0 360 330' style={{ position: 'absolute', left: 870, top: 310 }}>
    <rect x='0' y='0' width='360' height='330' fill='#E63F26' />
    <circle cx='40' cy='60' r='4' fill='#FFC000' opacity='0.9' />
</svg>
```

## 最佳实践

- **保持简洁**: 优先使用 FAIcon，仅在需要特殊自定义图形时使用 SVG。
- **坐标对齐**: 确保 `viewBox` 与 `width/height` 比例一致，避免图形变形。
