---
name: component-faicon
description: FAIcon 图标组件规范 — Font Awesome 图标集成、必填属性、常用图标速查
---

# FAIcon 组件规范

FAIcon 是本 DSL 中承载所有**矢量图标**的组件，集成 Font Awesome 图标集。相比 Emoji 更统一、可着色、尺寸可控，在专业演示文稿中**优先使用 FAIcon，严禁混用 Emoji**。

## 必填属性

```jsx
<FAIcon name='chart-line' style={{ fill: '#3b82f6', width: 32, height: 32 }} />
```

| 属性 | 必填 | 说明 |
| :--- | :--- | :--- |
| `name` | ✅ | 图标名称（如 `'heart'` / `'chart-line'` / `'check-circle'`），必须是 Font Awesome 中存在的名称 |
| `fill` | ✅ | 填充色。**不设置则完全不可见**（默认透明） |
| `width` / `height` | 推荐 | 尺寸，默认 24px，推荐范围 24-48px；超大图标（巨型 Header）可到 64-128px |
| `stroke` / `strokeWidth` | 可选 | 描边颜色与宽度（仅部分线框图标支持） |

## 常用图标速查

| 类别 | 图标名 |
| :--- | :--- |
| **基础** | `home` / `user` / `users` / `heart` / `star` / `check` / `times` / `plus` |
| **箭头** | `arrow-right` / `arrow-left` / `arrow-up` / `arrow-down` / `chevron-right` |
| **文件** | `file` / `folder` / `file-text` / `copy` / `clipboard` |
| **商务** | `briefcase` / `chart-line` / `chart-bar` / `dollar` / `calendar` / `clock` |
| **状态** | `check-circle` / `times-circle` / `exclamation-circle` / `info-circle` |
| **安全** | `lock` / `unlock` / `shield` / `key` / `eye` |
| **工具** | `cog` / `wrench` / `edit` / `pencil` / `trash` |
| **通讯** | `envelope` / `phone` / `comment` / `share` |

需要其他图标时先查 Font Awesome 官方图标库确认名称存在，**严禁编造图标名**（编造会导致校验失败 / 构建出空白图形）。

## 使用场景

### 要点卡片锚点

```jsx
<Box style={{ flexDirection: 'row', gap: 16, alignItems: 'flex-start' }}>
    <FAIcon name='check-circle' style={{ fill: '#10b981', width: 32, height: 32 }} />
    <Box>
        <Text style={{ fontSize: 18, fontWeight: 'bold' }}>自动化流程</Text>
        <Text style={{ fontSize: 14, color: '#64748b' }}>减少 80% 人工干预</Text>
    </Box>
</Box>
```

### 带背景容器的强调图标

```jsx
<Box style={{
    width: 64, height: 64,
    borderRadius: 16,
    background: '#eff6ff',
    justifyContent: 'center',
    alignItems: 'center',
}}>
    <FAIcon name='rocket' style={{ fill: '#3b82f6', width: 32, height: 32 }} />
</Box>
```

### 巨型 Header 锚点

```jsx
<FAIcon name='chart-line' style={{ fill: '#3b82f6', width: 96, height: 96, opacity: 0.9 }} />
```

## 风格一致性

- **粗细与圆角**必须在整份 PPT 中保持统一，不要混用"实心图标 + 线框图标"或"方形 + 圆形"
- 同一卡片组内的图标**尺寸统一**（全 32×32 或全 48×48）
- 图标着色优先用主题色（`accent1` / `accent2`），避免自创对比色