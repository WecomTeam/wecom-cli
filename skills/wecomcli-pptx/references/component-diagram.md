---
name: component-diagram
description: 流程图/架构图/时序图选型路由 — 默认走 SVG（见 component-svg.md），<Diagram> 仅作 Kroki 兜底
---
# 图示组件路由（流程图 / 架构图 / 时序图 / 关系图）

## 🚨 渲染策略（硬规则）

| 优先级         | 方案                                              | 渲染路径                  | 稳定性          | 写法在哪                          |
| -------------- | ------------------------------------------------- | ------------------------- | --------------- | --------------------------------- |
| **首选** | 内联 `<svg>`                                    | 浏览器原生                | ✅              | [component-svg.md](component-svg.md) |
| 次选           | `<Box>` + `<Text>` 绝对定位                   | DOM                       | ✅              | [component-box.md](component-box.md) |
| 兜底           | `<Diagram type='mermaid\|plantuml\|graphviz\|d2'>` | **远程 Kroki HTTP** | ❌ 常超时 / 502 | 本文 § 三                        |

**`<Diagram>` 4 种 type 全部走 Kroki**，一旦服务不可用整段报错且无降级。**默认走 SVG**，仅当节点 >10 + 交叉连线复杂 + 接受偶发失败时才考虑 Diagram。

## 选型一句话

| 场景                                            | 选                                                        |
| ----------------------------------------------- | --------------------------------------------------------- |
| 流程/步骤（≤8 节点）、含曲线、思维导图、时序图 | **SVG**（[component-svg.md](component-svg.md)）        |
| 4 象限 / SWOT / 卡片矩阵 / 纯矩形节点           | **Box + Text**（[component-box.md](component-box.md)） |
| 节点 >10 + 复杂交叉连线 + 接受偶发失败          | `<Diagram>`                                             |

---

## 三、`<Diagram>` 组件（兜底，慎用）

> ⚠️ 依赖远程 Kroki 服务（mermaid / plantuml / graphviz / d2 **4 种全部走 Kroki**），常出现超时 / 502。**Kroki 超时不要重试，直接改 SVG。**

### 强制规则

- **禁止在图中使用 emoji**（节点标题、注释、分组、图例、标签）
- `code` 字段只写对应 DSL 源码，不要混入 Markdown / 多余缩进
- 强调用颜色/线型/分组，不靠 emoji

### 核心属性

- `type: 'mermaid' | 'plantuml' | 'graphviz' | 'd2'`
- `code: string`（DSL 源码）
- `width: number` / `height: number`（必须明确设置）
- Mermaid 专属：`theme: 'default' | 'forest' | 'dark' | 'neutral'`，`themeVariables: Record<string,string>`

### 最小示例

```jsx
<Diagram
  type='mermaid'
  width={900}
  height={520}
  theme='default'
  code={`
flowchart TB
  A[需求] --> B[评审]
  B --> C[上线]
`}
/>
```
