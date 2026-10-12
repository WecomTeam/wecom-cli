---
name: component-animation
description: Animation 组件规范 — 入场/退出/强调动画，包裹单个子元素，导出 OOXML timing
---

# Animation 组件规范

`<Animation>` 包裹**单个子元素**为其添加动画，本身不占布局。导出时写入 OOXML `<p:timing>`，在 PowerPoint 放映模式下按顺序播放。

## 基础用法

```jsx
<Animation animType="fadeIn" duration={800}>
    <Text>标题</Text>
</Animation>

<Animation animType="fadeIn" startType="afterPrevious">
    <Text>第一句</Text>
</Animation>
<Animation animType="fadeIn" startType="afterPrevious">
    <Text>第二句</Text>
</Animation>
```

一个 `<Animation>` 只能包**一个**子元素。若需要一组元素同步入场，用 Box 包起来再套 Animation。

## 属性速查

| 属性 | 说明 |
| :--- | :--- |
| `animType` | **必填**。动画类型字符串，见下方白名单 |
| `startType` | `'click'`（默认，点击触发）/ `'withPrevious'`（与前一个同时）/ `'afterPrevious'`（前一个播完自动播） |
| `duration` | 时长毫秒，默认 500 |
| `delay` | 延迟毫秒，默认 0 |
| `direction` | 方向类动画（flyIn / peekIn 等）用，取值 `'top' / 'bottom' / 'left' / 'right' / 'topLeft' / 'topRight' / 'bottomLeft' / 'bottomRight' / 'center' / 'toCenter' / 'horizontal' / 'vertical' / 'clockwise' / 'counterClockwise'` 等 |

## 常用 animType 白名单

**入场（entrance）**：`appear` / `fadeIn` / `flyIn` / `wipe` / `zoom` / `floatIn` / `floatDown` / `bounce` / `blinds` / `box` / `checkerboard` / `circle` / `diamond` / `dissolve` / `peekIn` / `plus` / `randomBars` / `stretch` / `barnDoor` / `peekFrom` / `strips` / `wedge` / `wheel` / `flip` / `credits` / `easeIn` / `lightSpeed` / `growShrink` / `curveUp` / `riseUp` / `spiralIn` / `compress` / `ascend` / `swivel` / `spinner` / `sling` / `centerRevolve` / `pinwheel` / `swish` / `split`

**退出（exit）**：`disappear` / `fadeOut` / `flyOut` / `floatOut` / `floatDownOut` / `exitWipe` / `exitBlinds` / `exitBox` / `exitZoom` / `exitCircle` / `exitDiamond` / `exitFlip` / …（`exit*` 变体一一对应上方入场）

**强调（emphasis）**：`pulse` / `spin` / `transparency` / `fillColor` / `fontColor` / `lineColor` / `boldFlash` / `wave`

## 嵌套（一个元素叠多个动画）

```jsx
<Animation animType="fadeIn" duration={600}>
    <Animation animType="pulse" delay={800}>
        <Text style={{ fontSize: 48 }}>核心数据</Text>
    </Animation>
</Animation>
```

## 硬性限制

- 每个 `<Animation>` **只能有一个直接子元素**（多个会报错）
- `animType` 必填，且必须在上方白名单中
- 一期不支持：路径动画（`animMotion`）、trigger 触发器（点 A 触发 B）
- `startType` 默认 `'click'`。**不显式声明** = 需要观众手动点击才播；连续自动播必须显式写 `'afterPrevious'`

## 使用建议

- **不是每页都要动画**。优先用于封面 / 章节过渡 / 关键数字揭示；正文页密集列表禁止逐条 fadeIn（放映节奏拖沓）
- 单页动画数量 ≤ 4，全篇风格统一（要么都 `fadeIn`，要么都 `flyIn`，避免混用）
- `duration` 建议 300–800 ms，`delay` ≤ 500 ms，超过会显得卡顿
- 装饰元素、页脚、页码**不加**动画
