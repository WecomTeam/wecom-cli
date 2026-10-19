# 表格数据验证管理

读取、设置、更新或清除指定区域的数据验证规则。数据验证用于限制单元格可输入的值，例如单选下拉、多选下拉、整数范围、日期范围、时间范围或文本长度。

## 使用场景

| 操作需求 | CLI | 说明 |
| --- | --- | --- |
| 读取数据验证 | `sheet validation list` | 查询指定区域现有的数据验证设置 |
| 首次设置或清除数据验证 | `sheet validation set` | 给没有规则的区域创建规则，或用 `validation_type=none` 清除规则 |
| 修改已有数据验证 | `sheet validation update` | 仅在目标区域已有数据验证规则时使用 |
| 删除数据验证 | `sheet validation delete` | 兼容删除指定区域的数据验证规则；清除规则优先用 `validation set` |

## 执行路径

1. 先用 `validation list` 查看目标区域是否已有数据验证规则。
2. 没有规则时，用 `validation set` 首次设置。
3. 已有规则时，用 `validation update` 修改类型、条件或选项。
4. 用户要求清除数据验证、去掉下拉时，用 `validation set` 并传 `validation_type=none`。

## `wecom-cli sheet validation list`

读取指定区域的数据验证设置。

```bash
wecom-cli sheet validation list --json '<JSON>'
```

### 入参

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `docid` | `string` | 是 | 工作簿定位，支持文档 URL 或 docid |
| `sheet` | `string` | 是 | 子表 ID 或名称 |
| `range` | `string` | 是 | 要查询数据验证的带行号 A1 区域，如 `A2:A100`；不支持只写列字母 |

### 出参

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `items` | `ValidationItem[]` | 数据验证规则列表；无规则时为空数组 |

**`ValidationItem` 结构：**

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `validation_type` | `string` | 数据验证类型，如 `list` / `multiple_list` / `whole` |
| `ranges` | `string[]` | 应用该规则的 A1 区域列表 |
| `cell_count` | `int32` | 应用该规则的单元格数 |
| `styled_options` | `string[]` | 下拉类数据验证的字段；条件类规则没有该字段 |
| `has_unreadable_condition` | `bool` | 条件类规则通常为 `true`，表示条件存在但不可从 list 回包读出 |

## `wecom-cli sheet validation set`

首次设置或清除指定区域的数据验证规则。目标区域没有数据验证规则时，必须用本命令创建规则；不要直接调用 `validation update`。

```bash
wecom-cli sheet validation set --json '<JSON>'
```

### 入参

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `docid` | `string` | 是 | 工作簿定位，支持文档 URL 或 docid |
| `sheet` | `string` | 是 | 子表 ID 或名称 |
| `range` | `string` | 是 | 目标区域；区域模式写 `B2:B100`，整列模式写 `A` |
| `validation_type` | `string` | 是 | 数据验证类型；见下方类型表。清除规则时传 `none` |
| `styled_options` | `object[]` | 下拉类必填 | 下拉选项；用于 `list` / `multiple_list`，每项可只填文本，也可带颜色 |
| `op` | `string` | 条件类必填 | 比较运算符；用于 `whole` / `decimal` / `date` / `time` / `text_length` |
| `formula1` | `string` | 条件类必填 | 第一个比较值，只接受字面量，不支持公式或单元格引用 |
| `formula2` | `string` | 范围比较必填 | 第二个比较值；仅 `op=btw` / `op=not_btw` 时使用 |
| `ignore_rows` | `int32` | 否 | 整列模式下忽略表头前 N 行，如 `1` 表示跳过第 1 行；区域模式不用传 |
| `dry_run` | `bool` | 否 | 预览不落盘，返回 `applied=false` |

**`styled_options` 元素结构：**

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `text` | `string` | 是 | 选项文本 |
| `bg_color` | `string` | 否 | 选项背景色，格式 `#RRGGBB` |
| `text_color` | `string` | 否 | 选项文本颜色，格式 `#RRGGBB` |

### 支持的 `validation_type`

| 类型 | 含义 | 必填字段 | 禁止字段 |
| --- | --- | --- | --- |
| `list` | 单选下拉 | 非空 `styled_options` | `op` / `formula1` / `formula2` |
| `multiple_list` | 多选下拉 | 非空 `styled_options` | `op` / `formula1` / `formula2` |
| `whole` | 整数校验 | `op` / `formula1`，范围比较还要 `formula2` | `styled_options` |
| `decimal` | 小数校验 | `op` / `formula1`，范围比较还要 `formula2` | `styled_options` |
| `date` | 日期校验 | `op` / `formula1`，范围比较还要 `formula2` | `styled_options` |
| `time` | 时间校验 | `op` / `formula1`，范围比较还要 `formula2` | `styled_options` |
| `text_length` | 文本长度校验 | `op` / `formula1`，范围比较还要 `formula2` | `styled_options` |
| `none` | 清除数据验证 | 无 | `styled_options` / `strict` / `op` / `formula1` / `formula2` |

### 支持的 `op`

| `op` | 含义 | 参数要求 |
| --- | --- | --- |
| `btw` | 在范围内 | 必须传 `formula1` 和 `formula2` |
| `not_btw` | 不在范围内 | 必须传 `formula1` 和 `formula2` |
| `eq` | 等于 | 只传 `formula1` |
| `ne` | 不等于 | 只传 `formula1` |
| `gt` | 大于 | 只传 `formula1` |
| `lt` | 小于 | 只传 `formula1` |
| `gte` | 大于等于 | 只传 `formula1` |
| `lte` | 小于等于 | 只传 `formula1` |

### 出参

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `range` | `string` | 实际应用的区域；区域模式如 `B2:B100`，整列模式如 `A` |
| `applied` | `bool` | 是否已落盘执行；`dry_run=true` 时为 `false` |
| `validation_type` | `string` | 实际生效的数据验证类型 |

### 示例：设置单选下拉

```bash
wecom-cli sheet validation set --json '{
  "docid": "e3_ABC123",
  "sheet": "销售",
  "range": "A2:A100",
  "validation_type": "list",
  "styled_options": [
    {"text": "待处理"},
    {"text": "进行中"},
    {"text": "已完成"}
  ]
}'
```

### 示例：设置整数范围

```bash
wecom-cli sheet validation set --json '{
  "docid": "e3_ABC123",
  "sheet": "销售",
  "range": "D2:D100",
  "validation_type": "whole",
  "op": "btw",
  "formula1": "60",
  "formula2": "90"
}'
```
## `wecom-cli sheet validation update`

修改已有区域的数据验证规则。调用前必须确认目标区域已经存在数据验证规则；如果 `validation list` 查不到规则，先用 `validation set` 创建。

```bash
wecom-cli sheet validation update --json '<JSON>'
```

### 入参

`validation update` 的入参和 `validation set` 基本一致，但清除规则不要用 `update`，应改用 `validation set` 且传 `validation_type=none`。

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `docid` | `string` | 是 | 工作簿定位，支持文档 URL 或 docid |
| `sheet` | `string` | 是 | 子表 ID 或名称 |
| `range` | `string` | 是 | 目标区域；区域模式写 `B2:B100`，整列模式写 `A` |
| `validation_type` | `string` | 是 | 新数据验证类型，见 `validation set` 的类型表；不要传 `none` |
| `styled_options` | `object[]` | 否 | 下拉选项；用于 `list` / `multiple_list`，每项可只填文本，也可带颜色 |
| `op` | `string` | 条件类必填 | 比较运算符；用于 `whole` / `decimal` / `date` / `time` / `text_length` |
| `formula1` | `string` | 条件类必填 | 第一个比较值，只接受字面量，不支持公式或单元格引用 |
| `formula2` | `string` | 范围比较必填 | 第二个比较值；仅 `op=btw` / `op=not_btw` 时使用 |
| `ignore_rows` | `int32` | 否 | 整列模式下忽略表头前 N 行，如 `1` 表示跳过第 1 行；区域模式不用传 |
| `dry_run` | `bool` | 否 | 预览不落盘，返回 `applied=false` |
| `yes` | `bool` | 落盘时必填 | 确认执行更新；`dry_run=true` 时不用传 |

### 出参

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `range` | `string` | 实际更新的区域；区域模式如 `B2:B100`，整列模式如 `A` |
| `applied` | `bool` | 是否已落盘执行；`dry_run=true` 时为 `false` |
| `validation_type` | `string` | 实际应用的数据验证类型 |

## `wecom-cli sheet validation delete`

删除指定区域的数据验证规则。清除下拉或清除普通数据验证时，优先使用 `validation set` 并传 `validation_type=none`。

```bash
wecom-cli sheet validation delete --json '<JSON>'
```

### 入参

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `docid` | `string` | 是 | 工作簿定位，支持文档 URL 或 docid |
| `sheet` | `string` | 是 | 子表 ID 或名称 |
| `range` | `string` | 是 | 要删除数据验证的 A1 区域，如 `A2:A100` |
| `yes` | `bool` | 否 | 确认执行删除；兼容旧调用时可省略 |

### 出参

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `range` | `string` | 实际操作的区域 A1 |
| `applied` | `bool` | 是否成功应用删除操作 |

## 使用约束

- 执行设置、更新或清除前，优先用 `validation list` 查看目标区域现有规则。
- 目标区域没有数据验证规则时，用 `validation set`；目标区域已有规则且要修改时，用 `validation update`。直接对无规则区域调用 `validation update` 会报错，提示改用 set。
- 清除数据验证规则时，优先用 `validation set` 并传 `validation_type=none`；`validation delete` 也能删除规则，但返回中没有 `validation_type`。
- `range` 使用纯 A1 写法，不带子表名；子表通过 `sheet` 单独定位。
- `validation set` / `validation update` 的区域模式用于精确单元格范围，如 `B2:B100`；整列模式用于让新增行继承规则，如 `B`。
- `validation list` 只接受带行号区域；整列设置后如需复查，用实际数据区域查询，如 `B2:B100`。
- 整列模式需要跳过表头时传 `ignore_rows`；区域模式直接把起始行写进 `range`，不用传 `ignore_rows`。
- `list` / `multiple_list` 必须提供非空 `styled_options`；不要在设置或更新时传 `options`。
- 条件类规则必须显式提供完整条件：`op`、`formula1`，以及范围比较所需的 `formula2`。条件类规则的 `validation list` 回包不返回 `op` / `formula1` / `formula2`，只能看到 `has_unreadable_condition=true`。
- `formula1` / `formula2` 只写字面量；不要写公式、单元格引用或命名区域。
- 单边比较如 `gt` / `lt` / `gte` / `lte` / `eq` / `ne` 不要传 `formula2`。
