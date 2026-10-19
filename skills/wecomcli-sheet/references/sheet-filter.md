# 筛选器管理

对**在线表格**指定区域查询、设置 / 更新列筛选条件，或移除筛选器。筛选会隐藏不匹配的整行，包括 `range` 列范围之外的同行内容；范围上下边界外的行不参与本次筛选。筛选器只改变行的可见状态，不删除数据或移动单元格；禁止通过删除不符合条件的行、覆盖原表或复制结果到新表来冒充筛选器。

> **全局筛选器**：筛选器为**子表级全局对象**，每个子表至多一个，维护一个当前生效范围，可包含多个列条件。更新范围以后一次传入的范围为准；更新列条件只影响传入的列，不会清除未传入列的条件。

> **依赖关系**：`sheet` 使用子表 ID 或名称；`range` 一律使用纯 A1，其首行作为筛选表头，不参与条件判断，其余行参与筛选。应从真实表头行开始选取范围，避免将首条数据当作表头。按列名筛选前，先读取表头与少量数据样本，确认目标列和值的实际形态。

## 操作选择

| 用户意图 | 调用方式 |
|---|---|
| 查询当前筛选状态 | `filter list` |
| 首次设置筛选范围和列条件 | `filter update`，传入 `range` 和 `columns` |
| 范围不变，新增或修改列条件 | `filter update`，只传需要更新的 `columns`，省略 `range` |
| 调整范围并更新列条件 | `filter update`，传入新 `range` 和本次要更新的 `columns` |
| 取消某一列的筛选限制，恢复全选 | `filter update`，该列使用 `select_all: true` |
| 移除目标子表整个筛选器 | `filter clear` |

## 查询筛选器 — `wecom-cli sheet filter list`

查询当前子表是否存在筛选器，以及当前生效范围和列条件。

### 命令

```bash
wecom-cli sheet filter list --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `docid` | string | 是 | — | 在线表格 ID / URL，仅支持 sheet 类型文档 |
| `sheet` | string | 是 | — | 子表 ID 或名称 |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `exists` | bool | 当前子表是否存在筛选器 |
| `range` | string | 筛选器生效区域 A1 |
| `columns` | array | 各列筛选条件；没有限制可见值的列条件时，该列条件为空 |
| `columns[].column` | string | 筛选列；使用列字母；基准是整个子表 |
| `columns[].values` | array<string> | 保持可见的值白名单 |
| `note` | string | 提示信息 |

### 示例

```bash
wecom-cli sheet filter list --json '{"docid":"DOCID_EXAMPLE","sheet":"Sheet1"}'
```

```json
{
  "exists": true,
  "range": "A1:D100",
  "columns": [
    {
      "column": "B",
      "values": ["华北", "华东"]
    }
  ]
}
```

### 使用规则

- **读取当前状态**：更新或清除筛选器前先 list，确认当前 `exists`、`range` 和列条件。
- **不存在时不要臆造**：`exists=false` 表示当前子表没有筛选器；
- **按操作目标判断列条件**：预期保留指定值时，回读应能核对相应列的白名单；如果缺失，检查是否仅执行了预览、范围和目标列、列定位是否正确、值是否与单元格一致。

## 更新筛选器 — `wecom-cli sheet filter update`

设置或更新当前子表的筛选范围和列条件，也可让指定列恢复全选。一次请求可同时设置范围和多个列条件；已有筛选器时，只更新传入的列条件，未传入的列保持不变。

### 命令

```bash
wecom-cli sheet filter update --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `docid` | string | 是 | — | 在线表格 ID / URL |
| `sheet` | string | 是 | — | 子表 ID 或名称 |
| `range` | string | 条件必填 | 原区域 | 筛选区域，纯 A1，应从真实表头行开始；子表无筛选器时必填；已有筛选器时省略则沿用原范围，只有范围需要变化时才传入 |
| `columns` | array | 是 | — | 本次要更新的列条件数组；未传入的列条件保留，结构见下方 `columns[]`|
| `dry_run` | bool | 否 | `false` | `true` 时仅预览，不落盘 |

`columns[]` 对象结构：

| 子字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `column` | string | 是 | — | 筛选列；使用列字母；基准是整个子表，目标列应在筛选范围内 |
| `values` | array<string> | 条件必填 | — | 使用值白名单时必填，至少包含一个值；恢复全选时省略|
| `select_all` | bool | 否 | 未指定 | `true` 表示该列全部值可见，取消该列的值筛选限制；使用值白名单时省略 |

构造每个列对象时选择一种模式：`column + 非空 values`，或 `column + select_all: true`。不要同时传入 `values` 和 `select_all`，也不要为恢复全选读取并枚举该列的全部值。

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `filtered_columns` | int | 接口返回的筛选列计数；实际范围和列条件以 `filter list` 回读为准 |

### 示例

已有筛选器且范围不变，仅把 B 列改为“华北”；其他列条件保留，不需要重新传入：

```bash
wecom-cli sheet filter update --json '{"docid":"e3_ABC123","sheet":"销售","columns":[{"column":"B","values":["华北"]}],"dry_run":true}'
```

范围不变，仅取消 B 列的筛选限制；F 列等其他列条件继续生效：

```bash
wecom-cli sheet filter update --json '{"docid":"e3_ABC123","sheet":"销售","columns":[{"column":"B","select_all":true}],"dry_run":true}'
```

### 使用规则

- **列定位示例**：若筛选范围为 `C1:F100`，要筛选子表 E 列仍传 `"column":"E"`。
- **按列更新，不是整体覆盖**：同一列传入新的 `values` 后，以本次白名单为准，不与旧值合并；连续更新 B、F 列会分别保留两列条件，无需每次重传完整条件集合。
- **范围不变就省略**：如果update的时候不需要调整范围，则不要传入 `range` 字段。
- **单列全选不等于整体移除**：`select_all: true` 只取消目标列的筛选限制，保留筛选器范围和其他列条件；行仍可能被其他列条件筛掉。
- **正式写入后回读**：调用 `filter list` 核对 `exists`、`range` 和列条件。还可以通过`sheet-read-data.md`中的`cells get`回读数据验证。

## 移除筛选器 — `wecom-cli sheet filter clear`

移除当前子表的整个筛选器，包括筛选范围和全部列条件，不删除原始数据。仅取消某一列的筛选限制应使用 `filter update` 的 `select_all: true`，不要调用本接口。

### 命令

```bash
wecom-cli sheet filter clear --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `docid` | string | 是 | — | 在线表格 ID / URL |
| `sheet` | string | 是 | — | 子表 ID 或名称 |
| `yes` | bool | 是 | — | 移除确认；必须显式传 `true` |
| `dry_run` | bool | 否 | `false` | `true` 时仅预览，不落盘 |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `cleared` | bool | 是否成功清除当前子表的筛选器 |

### 示例

移除当前子表筛选器：

```bash
wecom-cli sheet filter clear --json '{"docid":"e3_ABC123","sheet":"销售","yes":true,"dry_run":true}'
```

### 使用规则

- **作用于目标子表**：`filter clear` 只清除 `sheet` 指定子表的筛选器；确认操作对象再执行。
- **先确认存在**：清除前先调用 `filter list`；如果 `exists=false`，不要继续调用 `filter clear`。
- **清除的是筛选配置**：先传 `dry_run: true` 预览影响范围，正式执行时保留 `yes: true`；操作不会删除单元格数据，如需恢复筛选配置须重新设置。
- **结果必须核对**：正式执行后检查 `cleared` 是否为 `true`，再调用 `filter list` 确认 `exists=false`。

## 能力边界

- 每个子表只有一个全局筛选器，不支持多筛选器共存或按 ID 精确操作；
- 不支持大于 / 小于 / 介于等条件筛选；
- 不支持正则、颜色、日期周期筛选；
- 不支持筛选视图；
- 支持通过 `filter update` 的 `select_all: true` 让指定列恢复全选；`filter clear` 只能移除整个子表的筛选器，不能用于单列操作。

用户需求超出这些能力时，明确说明限制并询问可接受方案，禁止通过删除原始数据模拟筛选结果。
