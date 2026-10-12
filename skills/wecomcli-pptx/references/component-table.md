---
name: component-table
description: Table component - cells format, defaultTextStyle, zebra striping, border config
---

# Table 组件规范

Table 组件用于展示结构化数据、对比信息或规格参数。它支持简化的单元格格式，能够自动处理对齐和基础样式。

> 继承关系：`Table` 继承自 `Box`，完整兼容 `Box` 的 `style` 属性，可直接设置背景、边距、阴影等样式。

## 核心属性

- **style**: 必填。通过 `style.width` / `style.height` 指定 Table 尺寸（必填），否则可能导致布局问题。`style.width` 支持百分比（例如 `width: '100%'`）。
    - 固定数值：`width={800} height={450}`
    - 百分比：`width="100%" height="100%"` （相对于父容器）- 推荐用于 LLM 生成，避免溢出父容器
- **cells**: 二维数组，支持字符串或对象格式。
- **defaultTextStyle**: 必填。默认文本样式 (推荐: `{ fontSize: 16, textAlign: 'center' }`)。
- **defaultCellStyle**: 必填。默认单元格样式（推荐包含边框、背景色等基础样式）。

## 布局与样式

- **默认布局**: `display: 'block'`。
- **单元格对象格式**:
    - `text`: 文本内容。
    - `textStyle`: 字体大小、颜色、加粗等。
    - `cellStyle`: 背景色、边框等。
- **注意事项**:
    - 所有行的列数必须一致。
    - 单元格边框支持 `left`, `right`, `top`, `bottom` 独立设置。

## 示例代码

### 基础用法 (带样式与斑马纹)

```jsx
// 百分比尺寸 - 推荐用于 LLM 生成，避免溢出父容器
<Table
    style={{ width: '100%', height: '100%' }}
    defaultTextStyle={{ fontSize: 16, textAlign: 'center', color: '#222' }}
    defaultCellStyle={{
        border: {
            left: { width: 1, color: '#e5e5e5' },
            right: { width: 1, color: '#e5e5e5' },
            top: { width: 1, color: '#e5e5e5' },
            bottom: { width: 1, color: '#e5e5e5' },
        },
    }}
    cells={[
        // 表头
        [
            { text: '功能', textStyle: { bold: true, color: '#fff' }, cellStyle: { background: { color: '#333' } } },
            { text: '基础版', textStyle: { bold: true, color: '#fff' }, cellStyle: { background: { color: '#333' } } },
            { text: '专业版', textStyle: { bold: true, color: '#fff' }, cellStyle: { background: { color: '#333' } } },
        ],
        // 数据行 (斑马纹示例)
        ['用户数', '10人', { text: '无限制', textStyle: { color: '#007aff', bold: true } }],
        [
            { text: '存储', cellStyle: { background: { color: '#f9f9f9' } } },
            { text: '5GB', cellStyle: { background: { color: '#f9f9f9' } } },
            { text: '100GB', cellStyle: { background: { color: '#f9f9f9' } } },
        ],
    ]}
/>

// 固定尺寸
<Table
    style={{ width: 800, height: 450 }}
    // ... 其他属性
/>
```

## 最佳实践

- **明确宽高**: 始终通过 `style` 为 Table 设置 `width` 和 `height`。推荐使用百分比尺寸（如 `style={{ width: '100%', height: '100%' }}`），避免溢出父容器。
- **视觉区分**: 为表头设置深色背景，为数据行设置交替背景色（斑马纹）以增强可读性。
- **对齐一致**: 数值类信息建议保持居中或右对齐。
