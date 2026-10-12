---
name: component-chart
description: Chart component - types, data format, colors, barDirection, grouping
---

# Chart 组件规范

Chart 组件用于在 PPT 中嵌入数据图表，适合展示：趋势（折线/面积）、对比（柱/条）、占比（饼/圆环）。

> 继承关系：`Chart` 继承自 `Box`，完整兼容 `Box` 的 `style` 属性，可直接设置宽高、背景、边距、阴影等样式。

## 1) 快速使用（推荐写法）

> 推荐：让 `Chart` 直接参与父级布局，宽度可依赖父容器（默认 `width: '100%'`），只需通过 `style.height` 或 `style.flex`（如 `flex: 1`, `minHeight`）保证垂直空间，否则无法撑满。

```jsx
<Chart
    // 只指定高度，宽度默认占满父容器
    style={{ height: 450 }}
    chartType='barChart'
    title='季度销售对比'
    data={[
        ['季度', '2023', '2024'],
        ['Q1', 120, 145],
        ['Q2', 156, 178],
        ['Q3', 198, 210],
        ['Q4', 230, 256],
    ]}
/>
```

## 2) 支持的图表类型

- 柱状/条形：`barChart`, `bar3dChart`
    - `barDirection: 'bar' | 'column'`（默认 `column`）
    - **注意**: `'bar'` 表示横向条形图，`'column'` 表示纵向柱形图（不要使用 `'horizontal'` 或 `'vertical'`）
- 折线/面积：`lineChart`, `areaChart`
    - 支持 `grouping='stacked'`（面积/柱状常用）
- 饼图/圆环：`pieChart`, `doughnutChart`
    - **仅支持单系列数据**（两列：类别 + 数值）

## 3) 数据格式（必须遵守）

- `data` 为二维数组
    - 第 1 行：表头（第一列是分类名，其余列为系列名）
    - 后续行：数据行（第一列为分类值，其余列为数值）
- **数值必须是 `number`，不要用字符串**（例如 `120` 而不是 `'120'`）

基础格式：

```jsx
[
    ['类别名', '系列1', '系列2'],
    ['分类1', 10, 20],
    ['分类2', 15, 18],
];
```

饼图/圆环（单系列）格式：

```jsx
[
    ['类别', '占比'],
    ['产品A', 35],
    ['产品B', 25],
    ['产品C', 20],
    ['产品D', 15],
    ['其他', 5],
];
```

## 4) 颜色与背景（纯色 / 渐变）

- `colors: string[]`：系列配色（可纯色、可渐变）
- `background: string`：图表背景（可纯色、可渐变）
- `titleColor / legendColor / axisColor / dataLabelColor`：**文本颜色只支持纯色**

支持的渐变语法（CSS 字符串）：

```txt
linear-gradient(90deg, #起始颜色 0%, #结束颜色 100%)
radial-gradient(circle, #起始颜色 0%, #结束颜色 100%)
```

混合纯色与渐变示例：

```jsx
<Chart
    chartType='areaChart'
    grouping='stacked'
    title='用户增长趋势'
    colors={['#4472C4', 'linear-gradient(90deg, #ea580c 0%, #fdba74 100%)']}
    background='linear-gradient(180deg, #f8fafc 0%, #f1f5f9 100%)'
    data={[
        ['月份', '活跃用户', '新增用户'],
        ['1月', 1200, 350],
        ['2月', 1450, 420],
        ['3月', 1780, 480],
        ['4月', 2100, 520],
    ]}
/>
```

## 5) 属性说明

| 属性             | 类型       | 必需 | 说明                                                                                                                                 |
| ---------------- | ---------- | ---- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `chartType`      | `string`   | ✅   | 图表类型（小驼峰字符串，如 `barChart`）                                                                                              |
| `data`           | `any[]`    | ✅   | 二维数组数据（数值必须为 `number`）                                                                                                  |
| `style`          | `object`   | ✅   | 必填。可继承 `Box` 的所有样式；宽度可依赖父容器（默认 `100%`），但必须提供高度（`height`）或 `flex`/`minHeight` 等设置来撑满垂直空间 |
| `title`          | `string`   | ❌   | 标题                                                                                                                                 |
| `colors`         | `string[]` | ❌   | 系列配色（纯色或渐变）                                                                                                               |
| `background`     | `string`   | ❌   | 背景（纯色或渐变）                                                                                                                   |
| `showLegend`     | `boolean`  | ❌   | 是否显示图例（默认 `true`）                                                                                                          |
| `showDataLabels` | `boolean`  | ❌   | 是否显示数据标签（默认 `false`）                                                                                                     |
| `titleColor`     | `string`   | ❌   | 标题文本颜色（纯色）                                                                                                                 |
| `legendColor`    | `string`   | ❌   | 图例文本颜色（纯色）                                                                                                                 |
| `axisColor`      | `string`   | ❌   | 坐标轴文本颜色（纯色）                                                                                                               |
| `dataLabelColor` | `string`   | ❌   | 数据标签颜色（纯色）                                                                                                                 |
| `grouping`       | `string`   | ❌   | 分组方式：`clustered \| stacked \| percentStacked \| standard`                                                                       |
| `barDirection`   | `string`   | ❌   | 柱状方向：`'bar'`（横向） \| `'column'`（纵向，默认）。**不要使用** `'horizontal'` 或 `'vertical'`                                   |

`grouping` 取值说明：

```js
'clustered';        // 簇状图：多系列并排显示，柱子之间有间隙
'stacked';          // 堆积图：多系列堆叠显示，显示绝对值总和
'percentStacked';   // 百分比堆积图：多系列堆叠显示，显示百分比（总和100%）
'standard';         // 标准图：用于折线图等
```

**注意拼写**：是 `percentStacked` (有字母 r)，不是 `precentStacked`

`barDirection` 取值示例：

```js
'bar';     // 横向条形图
'column';  // 纵向柱形图（默认）
// ❌ 不要使用: 'horizontal', 'vertical'
```

## 6) 常见场景怎么选

| 场景          | 推荐图表                              | 说明                             |
| ------------- | ------------------------------------- | -------------------------------- |
| 数据对比      | 柱状/条形（`barChart`）               | 适合多类别横向比较               |
| 趋势展示      | 折线（`lineChart`）                   | 时间序列/趋势最清晰              |
| 数量变化/构成 | 面积（`areaChart`）                   | 强调累积与变化幅度；可 `stacked` |
| 占比分析      | 饼/圆环（`pieChart`/`doughnutChart`） | 单系列、占比表达直观             |
| 精确数值清单  | Table（不建议用图表）                 | 静态明细更适合表格               |

## 7) 代表性示例

### A. 折线趋势

```jsx
<Chart
    style={{ width: 900, height: 450 }}
    chartType='lineChart'
    title='月度用户增长趋势'
    titleColor='#11182A'
    showLegend={true}
    showDataLabels={false}
    legendColor='#9ca3af'
    axisColor='#9ca3af'
    colors={['#4472C4', '#ED7D31']}
    background='#FFFFFF'
    data={[
        ['月份', '活跃用户', '新增用户'],
        ['1月', 1200, 350],
        ['2月', 1450, 420],
        ['3月', 1780, 480],
        ['4月', 2100, 520],
        ['5月', 2450, 580],
        ['6月', 2800, 620],
    ]}
/>
```

### B. 堆积面积图（产品构成）

```jsx
<Chart
    style={{ width: 800, height: 400 }}
    chartType='areaChart'
    grouping='stacked'
    title='产品线收入构成'
    titleColor='#1e293b'
    showLegend={true}
    showDataLabels={false}
    legendColor='#94a3b8'
    axisColor='#94a3b8'
    colors={[
        'linear-gradient(90deg, #5b21b6 0%, #a78bfa 100%)',
        'linear-gradient(90deg, #dc2626 0%, #fca5a5 100%)',
        'linear-gradient(90deg, #05966A 0%, #6ee7b7 100%)',
    ]}
    background='linear-gradient(180deg, #fefce8 0%, #fef3c7 100%)'
    data={[
        ['季度', '产品A', '产品B', '产品C'],
        ['Q1', 50, 30, 20],
        ['Q2', 65, 35, 25],
        ['Q3', 80, 40, 30],
        ['Q4', 95, 45, 35],
    ]}
/>
```

### C. 横向条形对比（`barDirection='bar'`）

```jsx
<Chart
    style={{ width: 700, height: 500 }}
    chartType='barChart'
    barDirection='bar'
    grouping='clustered'
    title='部门业绩对比'
    titleColor='#0f172a'
    showLegend={true}
    showDataLabels={true}
    legendColor='#64748b'
    axisColor='#64748b'
    dataLabelColor='#1e293b'
    colors={['#10b981', '#f59e0b', '#ef4444']}
    data={[
        ['部门', '目标', '实际', '差距'],
        ['销售部', 100, 115, 15],
        ['市场部', 80, 72, -8],
        ['研发部', 120, 125, 5],
        ['运营部', 90, 88, -2],
    ]}
/>
```

### D. 饼图占比（单系列 + 渐变背景）

```jsx
<Chart
    style={{ width: 600, height: 400 }}
    chartType='pieChart'
    title='市场份额分布'
    titleColor='#1f2937'
    showLegend={true}
    showDataLabels={true}
    legendColor='#37414F'
    dataLabelColor='#ffffff'
    colors={['#3b82f6', '#8b5cf6', '#ec4899', '#f59e0b']}
    background='linear-gradient(135deg, #f0f9ff 0%, #e0f2fe 100%)'
    data={[
        ['产品', '市场份额'],
        ['产品A', 35],
        ['产品B', 28],
        ['产品C', 22],
        ['产品D', 15],
    ]}
/>
```

## 8) 最佳实践

1. **尺寸**：`Chart` 默认横向拉伸，可省略宽度；务必提供 `style.height` 或 `style.flex`（如 `flex: 1` + `minHeight`），否则高度为 0。
2. **数据类型**：所有数值必须为 `number`。
3. **饼/圆环限制**：仅支持单系列（两列）。
4. **颜色策略**：`colors/background` 可渐变；文本色（`titleColor/legendColor/axisColor/dataLabelColor`）仅纯色。
5. **可读性**：`axisColor` 建议中性灰（如 `#9A9999` / `#9ca3af`），避免喧宾夺主；`dataLabelColor` 确保与背景对比足够。
6. **选型**：趋势用折线，构成用堆积面积或堆积柱，对比用柱/条，占比用饼/圆环；静态明细优先 Table。
