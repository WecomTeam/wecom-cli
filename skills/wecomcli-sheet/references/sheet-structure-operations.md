# 子表结构与布局操作

读取和调整**在线表格某个子表**的整行整列结构、行高列宽和合并单元格。查看工作簿及子表范围请先读 `sheet-workbook.md`；本文覆盖以下结构与布局操作：

| 命令                           | 风险 | 用途 |
|------------------------------|---|---|
| `wecom-cli sheet info get` | 只读 | 读取指定子表的合并范围、行高、列宽 |
| `wecom-cli sheet dims insert` | 结构写入 | 插入整行或整列 |
| `wecom-cli sheet dims delete` | 高风险写入 | 删除整行或整列 |
| `wecom-cli sheet dims move`  | 高风险写入 | 移动连续整行或整列 |
| `wecom-cli sheet dims visible` | 结构写入 | 显示或隐藏连续整行或整列 |
| `wecom-cli sheet dims resize` | 结构写入 | 调整行高或列宽 |
| `wecom-cli sheet merge set`  | 结构写入 | 合并单元格 |
| `wecom-cli sheet merge clear` | 高风险写入 | 取消合并 |
| `wecom-cli sheet merge get`  | 只读 | 查询合并范围 |

## 通用规则

- `docid` 用于定位在线表格；`sheet` 接受子表 ID 或名称，优先按 ID 精确匹配，未命中再按名称匹配。
- `range`、合并范围等区域参数只写纯 A1 表达式，不得包含子表名。
- 整行范围写成 `3:7`，整列范围写成 `C:F`；行位置使用 1-based 数字，列位置使用列字母。
- 结构性写入、高风险写入应先传 `"dry_run":true` 预览影响，确认后再正式执行。

## 结构变更前预检

插入、删除或移动行列会改变原有坐标和引用关系。执行前按影响范围检查以下内容：

1. **检查合并区域**：调用本文件的 `sheet info get` 或 `sheet merge get`，确认目标范围及其相邻区域是否存在合并单元格。
2. **检查引用影响**：结构变更可能使公式、图表、透视表、条件格式或筛选器的数据源范围发生偏移。若任务涉及这些对象，应先通过对应读取接口检查其引用范围；当前无法完整枚举所有引用时，应明确提示存在潜在影响，不得编造引用数量。
3. **先预览再执行**：删除、移动等高风险写入先传 `"dry_run":true`，核对目标范围后再正式执行。

## 批量结构操作的串行与索引规则

当一个用户请求包含多个独立的行列插入、删除或移动操作时，这些操作可能改变后续目标的行号、列号或引用关系，必须按以下规则执行：

1. **直接调用时每轮只执行一个结构变更 CLI**：模型直接调用 CLI 时，一次响应中最多调用一个 `sheet dims insert`、`sheet dims delete` 或 `sheet dims move`。禁止在同一轮并发调用多个结构变更 CLI；取得当前调用结果后，下一轮才能继续处理剩余目标。
2. **插入、删除按索引从大到小执行**：多个行目标按行号从大到小执行，多个列目标按列序从右到左执行。先处理较大索引，避免前面的操作改变剩余较小目标的原始位置。
3. **连续目标优先合并**：多个相邻行或列应尽量合并为一个连续范围，或使用一次带 `count` 的插入调用；不连续目标才拆成多个串行调用。不得为了减少调用而合并不相邻目标，否则会误处理目标之间的行列。
4. **每次执行后使用最新结构**：不得让多个调用继续使用操作前缓存的坐标。若前一次操作可能影响剩余目标，应根据返回结果或重新读取的子表范围，校验下一次调用的位置。
5. **移动操作不得只套用倒序规则**：多个 `dims move` 必须逐个执行，并在每次移动后重新计算剩余操作的源范围和目标位置；源范围、目标位置或中间区域存在重叠时，不得直接连续执行。

### 目标较多时使用 Python 串行调用

当独立的插入或删除目标较多、逐轮直接调用效率过低时，可以针对当前任务临时生成 Python 脚本批量调用 CLI。Python 只用于串行编排，不改变以上索引规则：

- 先合并相邻目标，再将行目标按行号从大到小、列目标按列序从右到左排序。
- 逐个同步调用 CLI；等待当前调用结束并检查退出状态与返回结果后，才能执行下一个。禁止使用线程池、进程池、`asyncio`、后台进程或其他并发方式。
- 任意一步失败时立即停止，明确列出已完成、失败和未执行的目标；不得自动重试写操作，也不得从头重跑完整列表。继续处理前，先按最新子表结构重新生成剩余计划。
- Python 批量串行只适用于可按倒序稳定处理的独立 `dims insert` / `dims delete` 目标。`dims move` 仍须逐步执行，并在每次移动后重新定位，不得按初始坐标写入循环批量执行。

示例：删除原始第 3、8、12 行时，按 `12:12` → `8:8` → `3:3` 分轮串行执行；目标较多时也只能由 Python 按该顺序逐个同步调用，不得并发提交。

反例：先删除 `3:3`，再按原始索引删除 `8:8`。第 3 行删除后，原第 8 行已变为第 7 行，后续操作会删除错误行。

## 定位语法速查

| 操作 | 定位字段 | 正确示例 | 常见错误                  |
|---|---|---|-----------------------|
| 插入行列 | `position` + `count` | `{"position":"3","count":2}` / `{"position":"C","count":2}` | 把列字母换算成 0-based 数字    |
| 删除行列 | `range` | `"3:7"` / `"C:F"` | 使用局部区域 `"A3:F7"`      |
| 移动行列 | `range` + `position` | `{"range":"3:7","position":"12"}` | 行范围搭配列目标，如 `"position":"H"` |
| 显示/隐藏行列 | `range` | `"3:7"` / `"C:F"` | 使用局部区域 `"A3:F7"` |
| 调整尺寸 | `range` | `{"range":"C:E"}` | 行 `range` 用 `col_width` 等维度错配      |
| 合并操作 | `range` | `"A1:B2"` | 在范围中嵌入子表名             |

## 获取子表布局信息

读取指定子表的合并范围、行高、列宽。该接口只返回布局和结构元信息，不读取单元格值；要读取表头、样本、公式或样式，请使用 `sheet-read-data.md` 中的 `cells get`。

### 命令

```bash
wecom-cli sheet info get --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `docid` | string | 是 | — | 在线表格 ID 或 URL |
| `sheet` | string | 是 | — | 子表 ID 或名称 |
| `range` | string | 否 | 整张子表 | 限定查询范围，纯 A1，如 `A1:F200` |
| `include` | string[] | 否 | 全部已支持项 | 返回维度：`merges`（合并单元格） / `row_heights`（行高） / `col_widths`（列宽）。缺省返回全部已支持项 |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `merges` | string[] | 合并范围列表，纯 A1 |
| `row_heights` | object | 行高映射，键为行索引，值为像素 px |
| `col_widths` | object | 列宽映射，键为列索引，值为像素 px |

### 示例

```bash
wecom-cli sheet info get --json '{"docid":"e3_ABC123","sheet":"销售","include":["merges","col_widths"]}'
```

### 使用规则

- 调整行列尺寸、写入合并区域、放置图表或执行结构变更前，可先读取现有布局，避免覆盖未知结构。
- 默认行高 20px，列宽 100px。返回的 `row_heights` 和 `col_widths` 只展示和默认值不同的行和列。不展示的行列即是默认值。
- 如果 `merges` 为空，说明当前子表没有单元格合并的情况。

## 插入整行或整列

在指定行或列之前/之后插入连续的整行或整列，自动继承原表格的样式。

### 命令

```bash
wecom-cli sheet dims insert --json '<JSON 参数>'
```

### 参数

| 字段              | 类型 | 必填 | 默认值 | 语义 |
|-----------------|---|---|---|---|
| `docid`         | string | 是 | — | 在线表格 ID 或 URL |
| `sheet`         | string | 是 | — | 子表 ID 或名称 |
| `position`      | string | 是 | — | 插入位置；行用 1-based 数字字符串，如 `"3"`；列用字母，如 `"C"` |
| `count`         | int | 是 | — | 插入数量，必须大于 `0` |
| `direction`     | string | 否 | `before` | `before` / `after` |
| `dry_run`       | bool | 否 | `false` | `true` 时仅预览，不落盘 |

### 示例

```bash
wecom-cli sheet dims insert --json '{"docid":"e3_ABC123","sheet":"销售","position":"C","count":3,"direction":"after"}'
```

### 扩展当前子表边界

当插入行列的目的是把表格可用范围向右或向下扩展时，先用 `sheet-workbook.md` 中的 `sheet workbook get` 确认当前子表范围，再用**当前边界内已有的最后一行或最后一列**作为 `position`，通过 `direction:"after"` 和 `count` 向外扩展。`position` 不要填目标新增区域的第一行或第一列；它必须落在当前范围内，否则会报错：“操作超出工作表边界”。

当前子表范围为 `A1:H10` 时，向右扩展 3 列，新增 `I:K`：

```bash
wecom-cli sheet dims insert --json '{"docid":"e3_ABC123","sheet":"销售","position":"H","count":3,"direction":"after"}'
```

当前子表范围为 `A1:H10` 时，向下扩展 5 行，新增第 `11:15` 行：

```bash
wecom-cli sheet dims insert --json '{"docid":"e3_ABC123","sheet":"销售","position":"10","count":5,"direction":"after"}'
```

### 使用规则

- **扩展边界必须锚在边界内**：向右扩展用当前末列；向下扩展用当前末行。不要把新位置直接当作 `position`。
- **多个不连续插入位置必须分轮串行执行**：多个行位置按行号从大到小，多个列位置按列序从右到左；一次模型响应中只调用一次本命令，取得结果后再处理下一位置。
- 插入后原有数据的行号、列字母和引用范围可能偏移。后续写入、合并、调整尺寸等操作必须按新结构重新计算位置，不要复用插入前缓存的 A1 地址。

## 删除整行或整列

删除一个连续的整行或整列范围。破坏性操作，必须传 `yes` 确认；建议先 `dry_run` 预览，确认后带 `yes` 执行。

### 命令

```bash
wecom-cli sheet dims delete --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义                       |
|---|---|---|---|--------------------------|
| `docid` | string | 是 | — | 在线表格 ID 或 URL            |
| `sheet` | string | 是 | — | 子表 ID 或名称                |
| `range` | string | 是 | — | 整行范围如 `3:7`，或整列范围如 `C:F` |
| `yes` | bool | 是 | — | 确认执行（缺省拒绝）               |
| `dry_run` | bool | 否 | `false` | `true` 时仅预览，不落盘          |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `deleted_range` | string | 实际删除的 A1 范围 |

### 使用规则

- 这里只能删除**整行或整列**。删除局部单元格块并让周边数据回填，不属于本接口，请读取 `sheet-range-operations.md`。
- **多个不连续删除范围必须分轮串行执行**：先合并相邻目标，再按行号从大到小或列序从右到左删除；一次模型响应中只调用一次本命令，禁止并发删除。
- 删除不可逆。执行前先确认目标范围；必要时通过数据读取接口读取该范围的内容，并检查相交的合并区域和引用关系。
- 删除后原有数据的坐标会前移，后续操作必须重新读取或计算目标位置。

## 移动整行或整列

将连续的整行或整列作为一个整体移动到新位置；值、公式和样式随行列一起移动。破坏性操作，必须传 `yes` 确认；建议先 `dry_run` 预览参数与影响区域，确认后带 `yes` 执行。

### 命令

```bash
wecom-cli sheet dims move --json '<JSON 参数>'
```

### 参数

| 字段        | 类型 | 必填 | 默认值    | 语义                         |
|-----------|---|---|--------|----------------------------|
| `docid`   | string | 是 | —      | 在线表格 ID 或 URL              |
| `sheet`   | string | 是 | —      | 子表 ID 或名称                  |
| `range`   | string | 是 | —      | 源范围：整行 `3:7` 或整列 `C:F`     |
| `position`  | string | 是 | —      | 目标位置；行用 1-based 数字字符串，列用字母 |
| `dry_run` | bool | 否 | `false` | `true` 时仅预览，不落盘            |
| `yes` | `bool` | 是 | —      | 确认执行（缺省拒绝）                 |

### 示例

```bash
wecom-cli sheet dims move --json '{"docid":"e3_ABC123","sheet":"销售","range":"3:7","position":"12","yes":true}'
```

### 使用规则

- 源范围与目标位置必须属于同一维度：`range="3:7"` 时 `position` 必须是行号；`range="C:F"` 时 `position` 必须是列字母。禁止 `{"range":"3:7","position":"H"}` 这类行列混用。
- **多个移动任务必须分轮串行执行**：一次模型响应中只调用一次本命令；每次移动完成后，重新定位剩余任务的源范围和目标位置，不得只按初始坐标并发或连续提交。
- 移动会改变源范围及其间隔区域的坐标。后续操作应以移动后的结构重新定位，不要继续使用移动前缓存的 A1 地址。

## 显示或隐藏行列

显示或隐藏连续整行 / 整列。隐藏行列只改变可见性，不删除数据、公式或样式。

### 命令

```bash
wecom-cli sheet dims visible --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `docid` | string | 是 | — | 在线表格 ID 或 URL |
| `sheet` | string | 是 | — | 子表 ID 或名称 |
| `range` | string | 是 | — | 整行范围如 `3:7`，或整列范围如 `D:D` / `C:F` |
| `visible` | bool | 是 | — | `true` 表示显示，`false` 表示隐藏 |
| `dry_run` | bool | 否 | `false` | `true` 时仅预览，不落盘 |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `dimension` | string | 处理维度：`row` / `col` |
| `affected_range` | string | 实际生效范围 |
| `count` | int | 生效的行/列数量 |
| `visible` | bool | 当前可见状态 |

### 示例

隐藏 D 列：

```bash
wecom-cli sheet dims visible --json '{"docid":"e3_ABC123","sheet":"销售","range":"D:D","visible":false}'
```

显示第 3 到 7 行：

```bash
wecom-cli sheet dims visible --json '{"docid":"e3_ABC123","sheet":"销售","range":"3:7","visible":true}'
```

### 使用规则

- **只能整行或整列**：`range` 必须是 `3:7`、`D:D`、`C:F` 这类整行/整列范围，不要传 `A3:F7` 这种局部单元格区域。
- **只支持单个连续范围**：不支持 `D:D,F:F` 这类逗号分隔多段范围；需要隐藏多个不连续范围时分多次调用。
- **隐藏不是删除**：用户要求“暂时不显示/收起辅助列”时用本命令；用户要求彻底移除数据时才考虑 `dims delete`。

## 调整行高或列宽

按 `range` 指定的维度统一或逐项设置行高 / 列宽，单位均为 pixel。

### 命令

```bash
wecom-cli sheet dims resize --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义                                                     |
|---|---|---|---|--------------------------------------------------------|
| `docid` | string | 是 | — | 在线表格 ID 或 URL                                          |
| `sheet` | string | 是 | — | 子表 ID 或名称                                              |
| `range` | string | 是 | — | 目标行/列范围（纯 A1），格式决定维度：纯数字为行（如 `2:5`），字母为列（如 `B:D`） |
| `row_height` | int | 条件必填 | — | 统一行高（pixel）；`range` 为行时与 `row_heights` 二选一 |
| `row_heights` | object | 条件必填 | — | 逐行行高（pixel）；`range` 为行时与 `row_height` 二选一，键为行号 |
| `col_width` | int | 条件必填 | — | 统一列宽（pixel）；`range` 为列时与 `col_widths` 二选一 |
| `col_widths` | object | 条件必填 | — | 逐列列宽（pixel）；`range` 为列时与 `col_width` 二选一，键为列字母 |
| `dry_run` | bool | 否 | `false` | `true` 时仅预览，不落盘 |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `dimension` | string | 处理维度：`rows` / `cols` |
| `affected_range` | string | 实际生效区域 A1 |
| `resized` | int | 实际调整的行/列数 |

### 示例

```bash
# 逐列列宽（range 为列）
wecom-cli sheet dims resize --json '{"docid":"e3_ABC123","sheet":"销售","range":"A:E","col_widths":{"A":100,"C:E":140}}'

# 统一行高（range 为行）
wecom-cli sheet dims resize --json '{"docid":"e3_ABC123","sheet":"销售","range":"2:10","row_height":30}'
```

### 使用规则

- `range` 格式决定维度：纯数字为行（`2:5`），字母为列（`B:D`）。同一调用只能调一种维度——行用 `row_height`（统一）或 `row_heights`（逐行），列用 `col_width`（统一）或 `col_widths`（逐列），不可行列混用。
- `row_heights` / `col_widths` 的键可以是单项或连续区间（如 `{"A":100,"C:E":140}`），可在一次调用设多个不连续尺寸组，不要为了不同尺寸拆成多次调用。

## 合并单元格

### 命令

```bash
wecom-cli sheet merge set --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `docid` | string | 是 | — | 在线表格 ID 或 URL |
| `sheet` | string | 是 | — | 子表 ID 或名称 |
| `range` | string | 是 | — | 待合并区域，纯 A1 |
| `merge_type` | string | 否 | `all` | `all` / `rows` / `columns` |
| `dry_run` | bool | 否 | `false` | `true` 时仅预览，不落盘 |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `merged_range` | string | 实际合并区域 |

### 使用规则

- `all` 将整个区域合为一个单元格。`rows` 将区域内每一行分别合并；`columns` 将区域内每一列分别合并。
- 合并前先调用 `sheet merge get` 检查相交的已有合并范围，避免与现有合并块冲突。

## 取消合并单元格

取消合并单元格。破坏性操作，必须传 `yes` 确认；建议先 `dry_run` 预览参数与影响区域，确认后带 `yes` 执行。

### 命令

```bash
wecom-cli sheet merge clear --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义              |
|---|---|---|---|-----------------|
| `docid` | string | 是 | — | 在线表格 ID 或 URL   |
| `sheet` | string | 是 | — | 子表 ID 或名称       |
| `range` | string | 是 | — | 待取消合并的纯 A1 区域   |
| `dry_run` | bool | 否 | `false` | `true` 时仅预览，不落盘 |
| `yes` | bool | 是 | — | 确认执行（缺省拒绝）      |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `unmerged_range` | string | 实际取消合并的区域 |

### 使用规则

- 区域不明确时先用 `sheet merge get` 查询，再对准确的合并范围执行取消。

## 查询合并范围

### 命令

```bash
wecom-cli sheet merge get --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `docid` | string | 是 | — | 在线表格 ID 或 URL |
| `sheet` | string | 是 | — | 子表 ID 或名称 |
| `range` | string | 是 | — | 查询区域，纯 A1 |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `merges` | string[] | 与查询区域相交的合并范围列表，纯 A1；无合并时为空数组 |
