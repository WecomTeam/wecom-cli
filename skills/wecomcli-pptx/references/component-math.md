---
name: component-math
description: Math component - LaTeX rendering via MathJax, display mode, common syntax
---

# Math 组件规范

用于展示数学公式，使用 MathJax 将 LaTeX 转换为 SVG。

## 核心属性

- **latex**: LaTeX 公式字符串（推荐）
- **width**: 公式宽度（推荐指定，避免过大或过小）
- **color**: 公式颜色，默认 #000（黑色）
- **display**: 是否为块级公式，默认 false

## 常用 LaTeX 语法

**基础**:

- 分数: `\frac{a}{b}`
- 根号: `\sqrt{x}` 或 `\sqrt[n]{x}`
- 上下标: `x^2`, `x_i`, `x^{10}`, `x_{ij}`

**符号**:

- 希腊字母: `\alpha, \beta, \gamma, \delta, \pi, \theta, \lambda, \mu, \sigma`
- 运算: `\pm, \times, \div, \cdot, \leq, \geq, \neq, \approx`
- 求和积分: `\sum, \int, \prod, \lim`
- 集合: `\in, \subset, \cup, \cap, \forall, \exists, \emptyset, \infty`

**高级**:

- 矩阵: `\begin{pmatrix} a & b \\ c & d \end{pmatrix}`
- 方程组: `\begin{cases} x + y = 5 \\ 2x - y = 1 \end{cases}`

## 示例

### 基础公式

```jsx
<Math latex="E = mc^2" width={200} color="#000" />
<Math latex="a^2 + b^2 = c^2" width={180} color="#000" />
<Math latex="e^{i\pi} + 1 = 0" width={180} color="#000" />
```

### 复杂公式（display 模式）

```jsx
<Math latex="\frac{-b \pm \sqrt{b^2-4ac}}{2a}" width={280} display={true} color="#000" />
<Math latex="\int_0^\infty e^{-x^2} dx = \frac{\sqrt{\pi}}{2}" width={300} display={true} color="#000" />
```

### 组合布局

```jsx
<Box style={{ display: 'flex', alignItems: 'center', gap: '15px' }}>
    <Text style={{ fontSize: '18px' }}>勾股定理：</Text>
    <Math latex='a^2 + b^2 = c^2' width={200} color='#000' />
</Box>
```

## 注意事项

- **Math 不是 inline 元素，不能放在 Text 内部**
    - ❌ 错误: `<Text>参数 <Math latex="\theta" width={10} /> 说明</Text>`
    - ✅ 正确: `<Box style={{ display: 'flex' }}><Text>参数</Text><Math latex="\theta" width={10} color="#000" /><Text>说明</Text></Box>`
- LaTeX 反斜杠需转义：`\\frac` 而非 `\frac`
- 建议指定 width 避免尺寸问题
- 复杂公式使用 `display={true}` 以块级模式渲染
