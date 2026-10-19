# 单元格搜索与替换

在指定的单个子表中搜索文本命中位置，或在该子表中查找并替换文本。`cells find` 与 `cells replace` 均必须传 `sheet`，一次调用只能作用于一个子表。

## 使用场景

| 操作需求 | CLI | 说明 |
| --- | --- | --- |
| 搜索文本并返回命中位置 | `sheet cells find` | 只读搜索，必须指定单个子表 |
| 在指定区域查找并替换文本 | `sheet cells replace` | 必须指定单个子表；写操作可先用 `dry_run` 预览替换结果 |

## 单元格搜索

### 命令

```bash
wecom-cli sheet cells find --json '<JSON>'
```

### 入参

| 字段 | 类型 | 必填 | 说明                                      |
| --- | --- | --- |-----------------------------------------|
| `docid` | `string` | 是 | 工作簿定位                                   |
| `sheet` | `string` | 是 | 单个子表 ID 或名称；不能省略或一次传入多个子表 |
| `keyword` | `string` | 是 | 搜索关键字                                   |
| `range` | `string` | 否 | 限定该子表的纯 A1 区域，不填搜索该子表全部区域 |
| `match_case` | `bool` | 否 | 大小写敏感，默认 `false`                        |
| `match_entire_cell` | `bool` | 否 | 整单元格完整匹配，默认 `false`                     |
| `use_regex` | `bool` | 否 | 按正则表达式搜索，默认 `false`                     |
| `match_formulas` | `bool` | 否 | 搜索公式字符内容，默认 `false`                     |
| `include` | `string[]` | 否 | 是否返回公式内容：`formula`                      |
| `cursor` | `string` | 否 | 分页游标。首页传空字符串或不传；后续页传上一页的 `next_cursor`  |
| `limit` | `int32` | 否 | 每页条数，默认 `50`，最大 `100`                   |

### 出参

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `matches` | `Match[]` | **当前页**命中项，结构见下 |
| `total` | `int` | 命中总数（**所有页合计**）|
| `next_cursor` | `string` | 下一页游标。**为空字符串表示已是最后一页** |

**`Match` 结构：**

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `sheet_id` | `string` | 命中所在的子表 ID |
| `address` | `string` | A1 地址，如 `"A2"` |
| `cell` | `Cell` | 命中单元格的内容，结构同读写 `Cell`（见 `sheet-write-cells.md` 的 `Cell` 定义）|

### 大数据直接落盘

支持把回包内容保存为 JSON 文件，无需先将完整回包载入模型上下文后再另行写入文件：

```bash
wecom-cli sheet cells find --json '<JSON 参数>' --output '<文件路径>'
```

### 使用规则

- 必须传 `sheet`，只在该子表内搜索；`range` 只限定该子表内的 A1 区域，不能用于跨子表搜索。反例：仅传 `docid` 和 `keyword`、省略 `sheet` 期望搜索整个工作簿。
- 目标子表未知，或需要在多个子表中查找字段、分类值或记录时：先用 `sheet workbook get` 获取子表列表，再对每个相关子表分别调用 `cells find`，每次传一个 `sheet`。不得将多个子表 ID 填进一次请求；只搜索一个子表未命中，不能断言整个工作簿不存在。
- `matches` 只代表当前子表的当前页。每个子表存在 `next_cursor` 时持续翻页直到为空；需要全量计数或工作簿范围的否定结论时，汇总所有相关子表的完整结果。

### 请求示例

```bash
wecom-cli sheet cells find --json '{"docid":"e3_ABC123","sheet":"sheet_id_1","keyword":"订单号"}'
```

## 查找替换单元格内容

### 命令

```bash
wecom-cli sheet cells replace --json '<JSON>'
```

### 入参

| 字段 | 类型 | 必填 | 说明                                 |
| --- | --- |----|------------------------------------|
| `docid` | `string` | 是  | 工作簿定位                              |
| `sheet` | `string` | 是  | 单个子表 ID 或名称；不能省略或一次传入多个子表 |
| `range` | `string` | 否  | 查找替换的纯 A1 区域，不填默认全子表               |
| `find_text` | `string` | 是  | 要查找的文本                             |
| `replace_text` | `string` | 是  | 替换后的文本                             |
| `match_case` | `bool` | 否  | 是否区分大小写；`true` 表示区分，默认 `false`     |
| `match_entire_cell` | `bool` | 否  | 是否要求整单元格完整匹配，默认 `false`            |
| `find_in_formula` | `bool` | 否  | 是否匹配公式表达式；`true` 表示匹配公式，默认 `false` |
| `dry_run` | `bool` | 否  | 预览替换结果但不执行，默认 `false`              |

### 出参

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `replaced_cells` | `int` | 完成替换的单元格数 |
| `replacements` | `Replacement[]` | 替换明细，结构见下 |
| `replacements_truncated` | `bool` | `true` 表示替换明细因数量限制被截断 |

**`Replacement` 结构：**

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `sheet_id` | `string` | 替换所在的子表 ID |
| `address` | `string` | 被替换单元格的 A1 地址 |
| `cell` | `string` | 替换后的单元格文本 |

### 使用规则

- 必须传 `sheet`，一次只替换该子表内的匹配内容；`range` 只限定该子表内的 A1 区域。反例：省略 `sheet` 期望替换整个工作簿，或在一次请求中传入多个子表。
- 用户要求替换多个子表时，先用 `sheet workbook get` 确认目标子表，再对每个目标子表分别调用 `cells replace`；对每个子表分别预览和核对影响范围，不得默认一次调用覆盖全部子表。
- 替换是写操作；范围较大或匹配条件可能过宽时，先传 `dry_run=true` 查看影响范围，再正式执行。
- `find_in_formula=true` 时匹配公式表达式；否则按单元格显示文本查找。

### 请求示例

```bash
wecom-cli sheet cells replace --json '{"docid":"e3_ABC123","sheet":"sheet_id_1","find_text":"旧值","replace_text":"新值","dry_run":true}'
```
