# 表格样式 — `wecom-cli sheet table style add` / `update` / `delete` / `list`

对**在线表格**中的连续 A1 区域套用、修改、删除或查询内置表格样式。表格样式是独立对象，包含标题行、首列 / 末列强调、筛选按钮及各部位颜色等配置；它不同于 `sheet style set` 的普通单元格字体 / 填充 / 对齐，也不同于条件格式规则。

> **依赖关系**：`sheet` 使用子表 ID 或名称；`range` 一律使用纯 A1，不包含子表名。创建前先定位真实数据区域，避免把空白尾行尾列纳入表格样式范围。

## 使用场景

| 操作需求 | CLI | 说明 |
| --- | --- | --- |
| 给一块数据套用内置表格样式 | `sheet table style add` | 如“给 A1:D10 套用蓝色隔行填充，保留标题行和筛选按钮” |
| 修改已有表格样式 | `sheet table style update` | 按 `tab_style_id` 修改标题行、首末列强调、筛选按钮或各部位颜色 |
| 删除已有表格样式 | `sheet table style delete` | 可选择是否保留单元格当前呈现出来的格式 |
| 查询表格样式 | `sheet table style list` | 按子表、区域查询已有表格样式|

## 定位前置检查

1. 先用 `sheet-workbook.md` 或 `sheet-read-data.md` 确认目标子表和真实数据范围。
2. 用户按“表头”“数据区”“整张表”描述时，先读取数据确定精确 A1 区域，再调用 `table style add`。
3. 更新或删除时优先通过 `table style list` 获取稳定 `tab_style_id`，不要按返回顺序或外观盲选。
4. 表格样式和普通单元格样式可以并存。用户要求“套用表格格式”时用本文件；只改字体、底色、边框时用 `sheet-style.md`。

## 自定义颜色

创建和更新时可按部位覆盖表格样式颜色：`header_fill_color`、`band_fill_color`、`second_band_fill_color`、`row_band_border_color`、`column_band_border_color`、`first_column_fill_color`、`outer_border_color`。颜色统一使用 `#RRGGBB`，同时兼容不透明 ARGB `FFRRGGBB`；未传时使用接口默认颜色。第一、第二条纹填充色交替作用于正文条纹，隔行线和隔列线颜色分别控制行间横线和列间竖线。

## 新增表格样式 — `wecom-cli sheet table style add`

### 命令

```bash
wecom-cli sheet table style add --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `docid` | string | 是 | — | 在线表格 ID / URL，仅支持 sheet 类型文档 |
| `sheet` | string | 是 | — | 子表 ID 或名称 |
| `range` | string | 是 | — | 要套用表格样式的目标区域，纯 A1，如 `A1:D10`；区域至少包含标题行和一行数据 |
| `header_row` | bool | 否 | `true` | 是否把首行作为标题行 |
| `filter_button` | bool | 否 | `true` | 是否显示标题行筛选按钮 |
| `dry_run` | bool | 否 | `false` | 是否仅预览结果、不落盘 |
| `header_fill_color` | string | 否 | 接口默认颜色 | 标题行填充色，格式为 `#RRGGBB`，兼容不透明 ARGB `FFRRGGBB` |
| `band_fill_color` | string | 否 | 接口默认颜色 | 第一条纹填充色，与 `second_band_fill_color` 交替作用于正文条纹；格式为 `#RRGGBB`，兼容不透明 ARGB `FFRRGGBB` |
| `second_band_fill_color` | string | 否 | 接口默认颜色 | 第二条纹填充色，与 `band_fill_color` 交替作用于正文条纹；格式为 `#RRGGBB`，兼容不透明 ARGB `FFRRGGBB` |
| `row_band_border_color` | string | 否 | 接口默认颜色 | 隔行线色（行间横线），格式为 `#RRGGBB`，兼容不透明 ARGB `FFRRGGBB` |
| `column_band_border_color` | string | 否 | 接口默认颜色 | 隔列线色（列间竖线），格式为 `#RRGGBB`，兼容不透明 ARGB `FFRRGGBB` |
| `first_column_fill_color` | string | 否 | 接口默认颜色 | 标题列（首列强调）填充色，格式为 `#RRGGBB`，兼容不透明 ARGB `FFRRGGBB` |
| `outer_border_color` | string | 否 | 接口默认颜色 | 表格外边框色，格式为 `#RRGGBB`，兼容不透明 ARGB `FFRRGGBB` |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `tab_style_id` | string | 新建表格样式 ID，后续更新 / 删除使用 |
| `range` | string | 实际生效区域，纯 A1 |
| `header_fill_color` | string | 实际生效的标题行填充色；非自定义颜色时不返回 |
| `band_fill_color` | string | 实际生效的第一条纹填充色；非自定义颜色时不返回 |
| `second_band_fill_color` | string | 实际生效的第二条纹填充色；非自定义颜色时不返回 |
| `row_band_border_color` | string | 实际生效的隔行线色；非自定义颜色时不返回 |
| `column_band_border_color` | string | 实际生效的隔列线色；非自定义颜色时不返回 |
| `first_column_fill_color` | string | 实际生效的标题列（首列强调）填充色；非自定义颜色时不返回 |
| `outer_border_color` | string | 实际生效的表格外边框色；非自定义颜色时不返回 |

### 示例

创建带自定义标题行、双色条纹和边框颜色的表格样式，保留标题行和筛选按钮，不强调首列或末列：

```bash
wecom-cli sheet table style add --json '{"docid":"REAL_SHEET_DOCID","sheet":"Sheet1","range":"A1:D10","header_row":true,"first_column":false,"last_column":false,"filter_button":true,"dry_run":false,"header_fill_color":"#D4E5FC","band_fill_color":"#FFFFFF","second_band_fill_color":"#E4F9FE","row_band_border_color":"#DCE3ED","column_band_border_color":"#DCE3ED","outer_border_color":"#BAC8DA"}'
```

### 使用规则

- **区域要先定位**：不要直接把子表已用范围全部套样式，除非用户明确要求整张已用区域。
- **创建后保存 ID**：后续修改或删除必须使用 `tab_style_id`；同一子表中可以有多个表格样式对象。
- **标题行语义**：`header_row=true` 时区域首行会作为表头参与样式和筛选按钮逻辑；不应把标题外的说明行纳入 `range`。
- **自定义颜色按部位覆盖**：仅传需要覆盖的颜色字段；未传字段使用接口默认颜色。`band_fill_color` 与 `second_band_fill_color` 交替作用于正文条纹；`row_band_border_color` 控制行间横线，`column_band_border_color` 控制列间竖线。
- **预览后落盘**：需要先确认效果时传 `dry_run=true`；确认后用同一组参数改为 `dry_run=false` 正式创建。

## 修改表格样式 — `wecom-cli sheet table style update`

### 命令

```bash
wecom-cli sheet table style update --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `docid` | string | 是 | — | 在线表格 ID / URL，仅支持 sheet 类型文档 |
| `sheet` | string | 是 | — | 子表 ID 或名称 |
| `tab_style_id` | string | 是 | — | 表格样式 ID；由 `table style add` 或 `table style list` 获得 |
| `header_row` | bool | 否 | 沿用当前状态 | 是否把首行作为标题行 |
| `first_column` | bool | 否 | `false` | 是否强调首列 |
| `last_column` | bool | 否 | `false` | 是否强调末列 |
| `filter_button` | bool | 否 | `true` | 是否显示标题行筛选按钮 |
| `dry_run` | bool | 否 | `false` | 是否仅预览结果、不落盘 |
| `header_fill_color` | string | 否 | 接口默认颜色 | 标题行填充色，格式为 `#RRGGBB`，兼容不透明 ARGB `FFRRGGBB` |
| `band_fill_color` | string | 否 | 接口默认颜色 | 第一条纹填充色，与 `second_band_fill_color` 交替作用于正文条纹；格式为 `#RRGGBB`，兼容不透明 ARGB `FFRRGGBB` |
| `second_band_fill_color` | string | 否 | 接口默认颜色 | 第二条纹填充色，与 `band_fill_color` 交替作用于正文条纹；格式为 `#RRGGBB`，兼容不透明 ARGB `FFRRGGBB` |
| `row_band_border_color` | string | 否 | 接口默认颜色 | 隔行线色（行间横线），格式为 `#RRGGBB`，兼容不透明 ARGB `FFRRGGBB` |
| `column_band_border_color` | string | 否 | 接口默认颜色 | 隔列线色（列间竖线），格式为 `#RRGGBB`，兼容不透明 ARGB `FFRRGGBB` |
| `first_column_fill_color` | string | 否 | 接口默认颜色 | 标题列（首列强调）填充色，格式为 `#RRGGBB`，兼容不透明 ARGB `FFRRGGBB` |
| `outer_border_color` | string | 否 | 接口默认颜色 | 表格外边框色，格式为 `#RRGGBB`，兼容不透明 ARGB `FFRRGGBB` |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `tab_style_id` | string | 被修改的表格样式 ID |
| `range` | string | 表格当前区域，纯 A1 |

### 示例

将已有对象更新为绿色主题的自定义标题行、双色条纹和边框颜色，保留标题行，不强调首列或末列：

```bash
wecom-cli sheet table style update --json '{"docid":"REAL_SHEET_DOCID","sheet":"Sheet1","tab_style_id":"table_style_abc123","header_row":true,"first_column":false,"last_column":false,"filter_button":true,"dry_run":false,"header_fill_color":"#D2EED8","band_fill_color":"#FFFFFF","second_band_fill_color":"#F4FCF6","row_band_border_color":"#DCE8E0","column_band_border_color":"#DCE8E0","outer_border_color":"#B9D1C1"}'
```

### 使用规则

- **先查后改**：调用前先用 `table style list` 确认 `tab_style_id` 和当前区域。
- **传最终期望状态**：建议显式传入 `first_column`、`last_column`、`filter_button` 以及所需颜色字段的最终期望值；除 `header_row` 省略时沿用当前状态外，其余配置字段省略时均使用默认值，不会沿用原配置。
- **自定义颜色按部位覆盖**：仅传需要覆盖的颜色字段；未传字段使用接口默认颜色。`band_fill_color` 与 `second_band_fill_color` 交替作用于正文条纹；`row_band_border_color` 控制行间横线，`column_band_border_color` 控制列间竖线。
- **预览后落盘**：需要先确认效果时传 `dry_run=true`；确认后用同一组参数改为 `dry_run=false` 正式更新。
- **原地修改**：修改标题行、首末列强调、筛选按钮或各部位颜色时使用 update，不要删除后重建，以免丢失对象 ID 或影响用户已建立的引用。

## 删除表格样式 — `wecom-cli sheet table style delete`

### 命令

```bash
wecom-cli sheet table style delete --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `docid` | string | 是 | — | 在线表格 ID / URL，仅支持 sheet 类型文档 |
| `sheet` | string | 是 | — | 子表 ID 或名称 |
| `tab_style_id` | string | 是 | — | 要删除的表格样式 ID |
| `keep_cell_style` | bool | 否 | — | 删除表格样式对象后是否保留当前呈现出的单元格视觉格式；`true` 保留字体 / 底色 / 条纹等显示效果，`false` 移除表格样式带来的视觉格式 |
| `yes` | bool | 是 | — | 删除确认；必须显式传 `true` |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `tab_style_id` | string | 已删除的表格样式 ID |

### 示例

```bash
wecom-cli sheet table style delete --json '{"docid":"REAL_SHEET_DOCID","sheet":"Sheet1","tab_style_id":"table_style_abc123","keep_cell_style":true,"yes":true}'
```

### 使用规则

- **删除前先定位**：先 `table style list` 核对 `tab_style_id` 和区域，避免误删相邻表格对象。
- **删除对象不删数据**：删除的是表格样式对象及其结构化样式效果，不删除区域内的单元格值。

## 查询表格样式 — `wecom-cli sheet table style list`

### 命令

```bash
wecom-cli sheet table style list --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `docid` | string | 是 | — | 在线表格 ID / URL，仅支持 sheet 类型文档 |
| `sheet` | string | 是 | — | 子表 ID 或名称 |
| `ranges` | array<string> | 否 | — | 查询区域列表，元素为纯 A1，如 `["A1:D10"]`|

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `items` | array | 表格样式列表 |
| `items[].tab_style_id` | string | 表格样式 ID |
| `items[].range` | string | 表格当前区域，纯 A1 |
| `items[].has_header` | bool | 是否包含标题行 |
| `items[].first_column` | bool | 是否强调首列 |
| `items[].last_column` | bool | 是否强调末列 |
| `items[].column_count` | int | 表格样式区域的列数 |

### 使用规则

- **查询用于定位 ID**：更新 / 删除前必须先拿到稳定 `tab_style_id`。
