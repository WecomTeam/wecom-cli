# 表格区域范围操作

对指定 A1 区域执行操作：插入空白单元格并移位、删除单元格并回填、清除内容或样式、按多列排序、自动填充，以及复制或剪切区域。

## 使用场景

| 操作需求 | CLI | 说明 |
| --- | --- | --- |
| 插入空白单元格 | `sheet ranges insert` | "在 B2 到 D2 插入空格，把原区域往下挪" |
| 删除单元格块 | `sheet ranges delete` | "删掉 A4 到 E6 这片区域，把下面内容往上挪" |
| 清除内容或全部内容与样式 | `sheet ranges clear` | "清空这块的值"、"这片区域全部清空还原默认" |
| 区域排序 | `sheet ranges sort` | "按 E 列降序排"、"先按部门升序再按金额降序" |
| 自动填充区域 | `sheet ranges autofill` | "根据 A1:A2 的规律填充 A3:A10"、"只复制样式" |
| 复制或剪切区域 | `sheet ranges copy` | "把 A1:B2 复制到 D1"、"剪切并只粘贴值" |

## 批量局部区域操作的串行与索引规则

当一个用户请求包含多个独立的局部单元格插入、删除或剪切操作时，前一次操作可能使后续区域的 A1 坐标失效，必须按以下规则执行：

1. **直接调用时每轮只执行一个坐标变更 CLI**：模型直接调用 CLI 时，一次响应中最多调用一个 `sheet ranges insert`、`sheet ranges delete`，或一个 `mode=cut` 的 `sheet ranges copy`。禁止在同一轮并发调用多个坐标变更 CLI；取得当前调用结果后，下一轮才能继续处理剩余目标。
2. **沿位移维度从大到小执行**：
   - `ranges insert` 使用 `shift=down` 时，按区域起始行从大到小执行；
   - `ranges insert` 使用 `shift=right` 时，按区域起始列从右到左执行；
   - `ranges delete` 使用 `shift=up` 时，按区域起始行从大到小执行；
   - `ranges delete` 使用 `shift=left` 时，按区域起始列从右到左执行。
3. **相邻且规则一致的区域优先合并**：只有合并后仍准确表达用户目标且 `shift` 相同时，才合并为一次调用；不连续区域必须拆分并串行执行。
4. **每次执行后校验剩余区域**：不得让多个调用继续使用操作前缓存的全部 A1 地址。若区域存在重叠、同行同列依赖，或前一次操作可能影响后续目标，应重新读取或重新计算下一目标。
5. **剪切操作必须重新定位**：多个 `mode=cut` 操作不能简单套用倒序。每完成一次剪切后，应重新确认剩余源区域和目标锚点；源区域、目标区域或移动路径相互影响时，先处理依赖关系。

### 目标较多时使用 Python 串行调用

当独立的插入或删除区域较多、逐轮直接调用效率过低时，可以针对当前任务临时生成 Python 脚本批量调用 CLI。Python 只用于串行编排，不改变以上位移与索引规则：

- 只批量处理同一命令、同一子表、相同 `shift` 且相互独立的区域；先合并可安全合并的相邻区域，再沿位移维度从大到小排序。
- 逐个同步调用 CLI；等待当前调用结束并检查退出状态与返回结果后，才能执行下一个。禁止使用线程池、进程池、`asyncio`、后台进程或其他并发方式。
- 任意一步失败时立即停止，明确列出已完成、失败和未执行的区域；不得自动重试写操作，也不得从头重跑完整列表。继续处理前，先按最新表格状态重新生成剩余计划。
- Python 批量串行只适用于可按位移方向稳定倒序处理的独立 `ranges insert` / `ranges delete` 区域。`mode=cut` 仍须逐步执行，并在每次剪切后重新定位，不得按初始坐标写入循环批量执行。

示例：以 `shift=up` 删除 `B3:D4` 和 `B10:D11` 时，先删除 `B10:D11`，取得结果后再删除 `B3:D4`；目标较多时也只能由 Python 按该顺序逐个同步调用。

反例：在同一轮并发删除 `B3:D4` 和 `B10:D11`。若前者先完成，下方区域会上移，后一个调用仍使用旧坐标，可能删除错误内容。

## 插入空白单元格

在指定区域插入空白单元格，并将原有单元格向下或向右移动。不要用于插入整行或整列；整行整列操作请读 `sheet-structure-operations.md`

### 命令

```bash
wecom-cli sheet ranges insert --json '<JSON>'
```

### 入参

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `docid` | `string` | 是 | 工作簿定位 |
| `sheet` | `string` | 是 | 子表 ID 或名称 |
| `range` | `string` | 是 | 待插入空白单元格的纯 A1 区域 |
| `shift` | `string` | 是 | 原有单元格的移动方向：`down`（下移）/ `right`（右移） |
| `dry_run` | `bool` | 否 | 仅预览参数与影响区域，不执行插入 |

### 出参

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `affected_range` | `string` | 实际插入的空白区域 A1 |
| `shift` | `string` | 实际使用的移动方向 |

### 使用规则

- **`shift` 描述原内容的移动方向**：`down` 表示区域内及其下方相关内容向下移动，`right` 表示向右移动
- **多个插入区域必须分轮串行执行**：`shift=down` 时按区域起始行从大到小，`shift=right` 时按区域起始列从右到左；一次模型响应中只调用一次本命令，禁止并发插入。
- **预览后正式执行**：需要确认影响范围时先传 `dry_run=true`；正式执行时去掉 `dry_run` 或传 `false`。本命令没有 `yes` 参数，不要自行添加。

## 删除单元格块

删除指定区域内的单元格，并让下方或右侧单元格回填。破坏性操作。建议先传 `dry_run=true` 预览；正式执行时必须传 `yes=true`。

### 命令

```bash
wecom-cli sheet ranges delete --json '<JSON>'
```

### 入参

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `docid` | `string` | 是 | 工作簿定位 |
| `sheet` | `string` | 是 | 子表 ID 或名称 |
| `range` | `string` | 是 | 待删除单元格块的纯 A1 区域 |
| `shift` | `string` | 是 | 删除后的回填方向：`up`（下方内容上移）/ `left`（右侧内容左移） |
| `yes` | `bool` | 条件必填 | 正式执行时必须为 `true`；`dry_run=true` 时可省略 |
| `dry_run` | `bool` | 否 | 仅预览参数与影响区域，不执行删除 |

### 出参

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `deleted_range` | `string` | 实际删除的区域 A1 |
| `shift` | `string` | 实际使用的回填方向 |

### 使用规则

- **多个删除区域必须分轮串行执行**：`shift=up` 时按区域起始行从大到小，`shift=left` 时按区域起始列从右到左；一次模型响应中只调用一次本命令，禁止并发删除。
- **删除后会改变结构**：`up` 让下方单元格上移，`left` 让右侧单元格左移。要保留单元格位置、只清内容时使用 `sheet ranges clear`。

## 清除区域内容或样式

清除指定区域的内容、样式或两者，但不删除单元格，也不移动周边数据。破坏性操作。建议先传 `dry_run=true` 预览；正式执行时必须传 `yes=true`。

### 命令

```bash
wecom-cli sheet ranges clear --json '<JSON>'
```

### 入参

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `docid` | `string` | 是 | 工作簿定位 |
| `sheet` | `string` | 是 | 子表 ID 或名称 |
| `range` | `string` | 是 | 待清除的纯 A1 区域 |
| `scope` | `string` | 否 | 清除范围：`content`（仅内容）/ `style`（仅样式）/ `all`（内容和样式），默认 `content` |
| `yes` | `bool` | 条件必填 | 正式执行时必须为 `true`；`dry_run=true` 时可省略 |
| `dry_run` | `bool` | 否 | 仅预览参数与影响区域，不执行清除 |

### 出参

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `cleared_range` | `string` | 实际清除的区域 A1 |
| `cleared_count` | `int32` | 清除的单元格数 |
| `scope` | `string` | 实际使用的清除范围 |

### 使用规则

- **`clear` 不改变结构**：单元格位置和行列布局保持不变。需要删除单元格并回填时使用 `sheet ranges delete`。

## 区域数据排序

在原区域内按一个或多个排序键重排行。排序会原地移动整行中的值、公式、样式和空值。

### 命令

```bash
wecom-cli sheet ranges sort --json '<JSON>'
```

### 入参

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `docid` | `string` | 是 | 工作簿定位 |
| `sheet` | `string` | 是 | 子表 ID 或名称 |
| `range` | `string` | 是 | 待排序的纯 A1 区域，排序键列必须位于该区域内 |
| `columns` | `SortColumn[]` | 是 | 排序键列表；按数组顺序依次作为第一、第二等排序键 |
| `has_header` | `bool` | 否 | `true` 表示 `range` 首行为表头且不参与排序，默认 `false` |
| `dry_run` | `bool` | 否 | 仅预览参数与影响区域，不执行排序 |

### `SortColumn` 入参

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `column` | `string \| int` | 是 | 子表中的绝对列：推荐使用列字母（如 `"E"`），也接受从 1 开始的列序号；不是 `range` 内的相对偏移 |
| `order` | `string` | 是 | 排序方向：`asc`（升序）/ `desc`（降序） |

### 出参

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `sorted_range` | `string` | 实际参与排序的数据区域 A1；`has_header=true` 时不包含表头行 |
| `sorted_rows` | `int` | 实际参与排序的数据行数 |

**示例：**

```bash
# 多级排序：先按 A 列升序，再按 E 列降序
wecom-cli sheet ranges sort --json '{
  "docid": "e3_ABC123",
  "sheet": "销售",
  "range": "A1:F100",
  "columns": [
    {"column": "A", "order": "asc"},
    {"column": "E", "order": "desc"}
  ],
  "has_header": true
}'
```

### 使用规则

- **排序键是子表绝对列**：若 `range` 从 C 列开始，`column: "E"` 仍表示子表 E 列，不表示 `range` 内第 E 列。
- **明确表头语义**：区域包含表头时传 `has_header=true`，否则默认值 `false` 会让首行参与排序。
- **排序区域包含合并单元格**：若 CLI 返回此错误，停止处理并告知用户手动排序。
- **排序区域与表格样式重叠**：若 CLI 返回此错误，先删除当前子表中的所有表格样式，再重新执行排序。
- **必须使用原生排序**：禁止通过“读取 → 本地排序 → 写回”模拟。写空值可能不覆盖目标单元格，且样式不会随行移动。

## 自动填充区域

根据源区域的内容或规律填充目标区域。用于扩展序列、复制单元格或仅延续样式；精确搬运一个固定区域时使用 `sheet ranges copy`。

### 命令

```bash
wecom-cli sheet ranges autofill --json '<JSON>'
```

### 入参

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `docid` | `string` | 是 | 工作簿定位 |
| `sheet` | `string` | 是 | 子表 ID 或名称 |
| `source_range` | `string` | 是 | 用于推断填充内容或规律的纯 A1 源区域 |
| `fill_range` | `string` | 是 | 实际要写入的纯 A1 目标区域，不包含 `source_range` |
| `fill_type` | `string` | 否 | `default` / `sequence` / `only_fill_style` / `fill_without_style` / `copy_cells`，默认 `default` |

### 出参

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `source_range` | `string` | 实际使用的源区域 A1 |
| `fill_range` | `string` | 实际写入的目标区域 A1 |


### 使用规则

- **源区域与目标区域分开填写**：`source_range` 是原始样本，`fill_range` 只写待填充区域。不要把源区域重复包含进目标区域。
- **按意图选择 `fill_type`**：自动判断复制或序列用 `default`；明确扩展序列用 `sequence`；只延续样式用 `only_fill_style`；填充内容但不带源样式用 `fill_without_style`；复制源单元格用 `copy_cells`。
- **自动填充与复制不同**：需要依据样本扩展规律时使用 `autofill`；需要把固定矩形区域原样搬到指定左上角时使用 `copy`。

## 复制或剪切区域

将一个固定矩形源区域复制或剪切到目标位置。源与目标可以位于同一子表、同一文档的不同子表，或不同文档；目标区域大小根据源区域自动确定。

### 命令

```bash
wecom-cli sheet ranges copy --json '<JSON>'
```

### 入参

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `docid` | `string` | 是 | 源 Sheet 文档 URL 或 docid |
| `sheet` | `string` | 是 | 源子表 ID 或名称 |
| `range` | `string` | 是 | 要复制或剪切的纯 A1 源区域 |
| `target_docid` | `string` | 否 | 目标 Sheet 文档 URL 或 docid |
| `target_sheet` | `string` | 否 | 目标子表 ID 或名称 |
| `to` | `string` | 是 | 目标子表中的左上角锚点单元格，如 `D1`；目标区域大小由源区域自动确定 |
| `mode` | `string` | 否 | `copy`（保留源区域）/ `cut`（搬运后清除源区域），默认 `copy` |
| `paste_type` | `string` | 否 | `all`（全部）/ `values`（仅值）/ `formats`（仅格式），默认 `all` |
| `dry_run` | `bool` | 否 | 仅预览源区域、目标位置及操作影响，不实际执行 |
| `yes` | `bool` | 条件必填 | `mode=cut` 正式执行时必须为 `true`；`dry_run=true` 时可省略 |

### 出参

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `copied_range` | `string` | 实际处理的源区域 A1；字段名固定，`mode=cut` 时也返回该字段 |
| `pasted_range` | `string` | 根据源区域大小计算出的实际目标区域 A1 |
| `copied_cells` | `int32` | 搬运的单元格数量 |
| `mode` | `string` | 实际使用的操作模式：`copy` / `cut` |
| `paste_type` | `string` | 实际使用的粘贴类型：`all` / `values` / `formats` |
| `note` | `string` | 本次操作的保真度限制说明 |

**示例：**

```bash
# 同一文档跨子表复制：把“销售”子表 A1:B2 复制到“汇总”子表 D1:E2
wecom-cli sheet ranges copy --json '{
  "docid": "e3_ABC123",
  "sheet": "销售",
  "range": "A1:B2",
  "target_sheet": "汇总",
  "to": "D1",
  "mode": "copy",
  "paste_type": "all"
}'

# 跨文档复制：target_docid 和 target_sheet 共同定位目标
wecom-cli sheet ranges copy --json '{
  "docid": "e3_SOURCE123",
  "sheet": "销售",
  "range": "A1:B2",
  "target_docid": "e3_TARGET456",
  "target_sheet": "归档",
  "to": "A1",
  "mode": "copy",
  "paste_type": "values"
}'
```

### 使用规则

- **分别定位源和目标**：`docid`、`sheet`、`range` 描述源；`target_docid`、`target_sheet`、`to` 描述目标。不要把目标子表填入 `sheet`。未传 `target_docid` 时目标仍在源文档，未传 `target_sheet` 时目标仍在源子表。
- **`to` 属于目标子表且只传左上角单元格**：源 `A1:B2` 搬到目标 `D1` 时，实际目标区域为 `D1:E2`；不要把 `to` 写成 `D1:E2`。
- **根据用户意图选择模式**：用户说“复制”“拷贝”时使用 `mode=copy`；说“剪切”“移动”“搬到……并删除原内容”时使用 `mode=cut`。命令名虽然是 `copy`，仍通过 `mode` 决定实际操作。
- **根据用户意图选择粘贴内容**：完整保留内容和格式用 `all`；只要单元格值用 `values`；只要格式用 `formats`。用户未指定时使用默认值 `all`。
- **剪切必须经过确认门禁**：先用 `mode=cut, dry_run=true` 核对源区域、目标文档、目标子表和自动推导出的完整目标区域；正式执行时去掉 `dry_run` 或传 `false`，并传 `yes=true`。`yes` 仅用于 `cut` 门禁，不要因普通 `copy` 自行添加。
- **多个剪切任务必须分轮串行执行**：一次模型响应中只执行一个 `mode=cut` 调用；每次剪切完成后，重新确认剩余源区域和目标锚点，不得只按初始坐标并发或连续提交。普通 `mode=copy` 不适用索引倒序规则，但目标存在覆盖关系时仍应串行执行。
- **关注保真度说明**：执行后检查返回的 `note`；若其中存在限制，向用户说明本次复制或剪切未能保真的内容。
