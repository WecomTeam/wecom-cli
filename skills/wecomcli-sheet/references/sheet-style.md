# 表格样式操作

对指定 A1 区域设置或清除单元格样式、边框与自动换行。

## 使用场景

| 操作需求 | CLI | 说明 |
| --- | --- | --- |
| 设置单元格样式 | `sheet style set` | "表头加粗居中"、"金额列两位小数"、"标题字号 14" |
| 还原默认样式 | `sheet style clear` | "清掉这块的格式"、"恢复默认样式" |
| 设置区域边框 | `sheet border set` | "加全框"、"只加外边框"、"加内部横线" |
| 清除区域边框 | `sheet border clear` | "去掉边框" |
| 设置自动换行 | `sheet autotab set` | "让这块内容自动换行" |
| 取消自动换行 | `sheet autotab delete` | "取消这块的自动换行" |

## 设置单元格样式

对 A1 区域应用字体 / 颜色 / 对齐 / 数字格式等样式。

### 命令

```bash
wecom-cli sheet style set --json '<JSON>'
```

### 入参

| 字段 | 类型 | 必填 | 说明                                                                                                 |
| --- | --- | --- |----------------------------------------------------------------------------------------------------|
| `docid` | `string` | 是 | 在线表格 ID                                                                                            |
| `sheet` | `string` | 是 | 子表 ID 或名称                                                                                          |
| `range` | `string` | 是 | 设置的 A1 区域                                                                                          |
| `font_family` | `string` | 否 | 字体名                                                                                                |
| `font_size` | `int` | 否 | 字号 pt                                                                                              |
| `bold` | `bool` | 否 | 粗体                                                                                                 |
| `italic` | `bool` | 否 | 斜体                                                                                                 |
| `underline` | `string` | 否 | `none` / `single` / `double` / `single_accounting` / `double_accounting`                           |
| `strike` | `bool` | 否 | 删除线                                                                                                |
| `font_color` | `string` | 否 | 字体色 ARGB hex                                                                                       |
| `bg_color` | `string` | 否 | 背景色 ARGB hex                                                                                       |
| `horizontal_align` | `string` | 否 | `general` / `left` / `center` / `right` / `fill` / `justify` / `center_continuous` / `distributed` |
| `vertical_align` | `string` | 否 | `top` / `center` / `bottom` / `justify` / `distributed`                                            |
| `wrap` | `bool` | 否 | 自动换行                                                                                               |
| `number_format` | `string` | 否 | Excel Number Format Code，如 `0.00%` / `#,##0` / `yyyy-mm-dd`。传 `""` 清除格式，不传保持原值                     |
| `dry_run` | `bool` | 否 | 预览不执行                                                                                              |

### 出参

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `affected_range` | `string` | 应用样式区域 A1 |
| `style_count` | `int` | 应用样式的单元格数 |

**示例：**

```bash
# 表头加粗居中、黄底黑字
wecom-cli sheet style set --json '{
  "docid": "e3_ABC123",
  "sheet": "销售",
  "range": "A1:B1",
  "font_family": "Microsoft YaHei",
  "font_size": 12,
  "bold": true,
  "font_color": "FF000000",
  "bg_color": "FFFFFF00",
  "horizontal_align": "center"
}'
```

### 使用规则

- **未传即保持原值**：所有可选样式字段只传要改的；`number_format` 传 `""` 清除格式，不传保持原值。不需要构造完整样式对象整体下发。
- **颜色 ARGB hex**：`font_color` / `bg_color` 用 8 位 ARGB（前 2 位 alpha），如 `FFFF0000` 不透明红、`FFFFFF00` 不透明黄。

## 清除区域单元格样式

破坏性操作，必须传 `yes` 确认；建议先 `dry_run` 预览，确认后带 `yes` 执行。

### 命令

```bash
wecom-cli sheet style clear --json '<JSON>'
```

### 入参

| 字段 | 类型 | 必填 | 说明         |
| --- | --- | --- |------------|
| `docid` | `string` | 是 | 表格 ID      |
| `sheet` | `string` | 是 | 子表 ID 或名称  |
| `range` | `string` | 是 | 纯 A1 区域    |
| `yes` | `bool` | 是 | 确认执行（缺省拒绝） |
| `dry_run` | `bool` | 否 | 预览不执行      |

### 出参

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `cleared_range` | `string` | 清除样式区域 A1 |
| `cleared_count` | `int` | 清除样式的单元格数 |

## 设置单元格边框

可同时设外框 + 内部线，也可只设外框或只设内部线。

### 命令

```bash
wecom-cli sheet border set --json '<JSON>'
```

### 入参

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `docid` | `string` | 是 | 表格 ID |
| `sheet` | `string` | 是 | 子表 ID 或名称 |
| `range` | `string` | 是 | 纯 A1 区域 |
| `positions` | `string[]` | 否 | 边框方位：`outer` / `top` / `bottom` / `left` / `right` / `inner_horizontal` / `inner_vertical` / `diagonal_up` / `diagonal_down`。`outer` 一次设置区域四个方向外边框，等价于 `top` + `bottom` + `left` + `right`；**缺省四边外框** |
| `style` | `string` | 否 | 线型：`none` / `thin` / `medium` / `dashed` / `dotted` / `thick` / `double` / `hair`，默认 `thin` |
| `color` | `string` | 否 | 边框色 ARGB hex，默认黑色 |
| `dry_run` | `bool` | 否 | 预览不执行 |

### 出参

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `affected_range` | `string` | 应用边框区域 A1 |
| `positions` | `string[]` | 实际生效的方位集合 |
| `style` | `string` | 实际生效的线型 |

**示例：**

```bash
# 只给 A1:F20 加四个方向外边框
wecom-cli sheet border set --json '{
  "docid": "e3_ABC123",
  "sheet": "销售",
  "range": "A1:F20",
  "positions": ["outer"],
  "style": "thin",
  "color": "FF85888C"
}'

# 给 A1:F20 加全框（四边外框 + 内部横竖线）
wecom-cli sheet border set --json '{
  "docid": "e3_ABC123",
  "sheet": "销售",
  "range": "A1:F20",
  "positions": ["outer", "inner_horizontal", "inner_vertical"],
  "style": "thin",
  "color": "FF000000"
}'
```

### 使用规则

- **外边框优先用 `outer`**：需要整块区域四边外框时传 `["outer"]`，不要逐个列出 `top` / `bottom` / `left` / `right`；`outer` 只作用于区域外围，不产生内部横竖线。
- **按需组合内部线**：需要内部线时在 `outer` 基础上追加 `inner_horizontal` / `inner_vertical`。

## 清除单元格边框

破坏性操作，必须传 `yes` 确认；不传 `positions` 即清除全部方位，传 `positions` 只清指定方位。

### 命令

```bash
wecom-cli sheet border clear --json '<JSON>'
```

### 入参

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `docid` | `string` | 是 | 表格 ID |
| `sheet` | `string` | 是 | 子表 ID 或名称 |
| `range` | `string` | 是 | 纯 A1 区域 |
| `positions` | `string[]` | 否 | 要清除的方位集合（方位取值同 `border set`），**缺省清除全部方位** |
| `yes` | `bool` | 是 | 确认执行（缺省拒绝） |
| `dry_run` | `bool` | 否 | 预览不执行 |

### 出参

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `cleared_range` | `string` | 清除边框区域 A1 |
| `positions` | `string[]` | 实际清除的方位集合 |

**示例：**

```bash
# 清除全部边框
wecom-cli sheet border clear --json '{
  "docid": "e3_ABC123",
  "sheet": "销售",
  "range": "A1:F20",
  "yes": true
}'
```

## 设置自动换行

### 命令

```bash
wecom-cli sheet autotab set --json '<JSON>'
```

### 入参

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `docid` | `string` | 是 | 表格 ID |
| `sheet` | `string` | 是 | 子表 ID 或名称 |
| `range` | `string` | 是 | 设置自动换行的纯 A1 区域 |

### 出参

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `affected_range` | `string` | 实际设置自动换行的区域 A1 |
| `style_count` | `int` | 应用样式的单元格数 |

## 取消单元格自动换行

### 命令

```bash
wecom-cli sheet autotab delete --json '<JSON>'
```

### 入参

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `docid` | `string` | 是 | 表格 ID |
| `sheet` | `string` | 是 | 子表 ID 或名称 |
| `range` | `string` | 是 | 取消自动换行的纯 A1 区域 |

### 出参

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `affected_range` | `string` | 实际取消自动换行的区域 A1 |
| `style_count` | `int32` | 应用样式的单元格数 |
