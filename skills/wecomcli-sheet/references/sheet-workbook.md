# 工作簿与子表管理

从零新建一篇企微在线表格，导入本地 CSV / Excel 文件为企微在线表格，读取整个在线表格工作簿，并管理其中子表的新增、删除、重命名、复制和顺序。

| 命令                              | 风险 | 用途 |
|---------------------------------|---|---|
| `wecom-cli doc create`          | 创建 | 从零新建一篇在线表格 |
| `wecom-cli doc import`        | 文件写入 | 导入本地文件为在线表格 |
| `wecom-cli sheet workbook get`  | 只读 | 获取工作簿中的全部子表 |
| `wecom-cli sheet subsheets create` | 结构写入 | 新增子表 |
| `wecom-cli sheet subsheets remove`  | 高风险写入 | 删除子表 |
| `wecom-cli sheet subsheets rename`  | 结构写入 | 重命名子表 |
| `wecom-cli sheet subsheets copy`    | 结构写入 | 复制子表 |
| `wecom-cli sheet subsheets move`    | 结构写入 | 调整子表顺序 |
| `wecom-cli sheet subsheets visible` | 结构写入 | 显示或隐藏子表 |

## 通用规则

- 结构性写入、高风险写入均支持 `dry_run`。需要预览时先传 `"dry_run":true`，确认后再正式执行。

## 新建在线表格

从零新建企微在线表格时，先按用户需求从零创建一个本地 `.xlsx`，完成质量校验后，再通过`doc import`导入为企微在线表格。本流程仅适用于从零生成在线表格；用户已有的 xlsx 本地文件不走本流程，直接按「导入在线表格」处理。

### 标准流程
- 使用 `openpyxl` 创建本地 `.xlsx`。本地文件只写入子表名称、单元格值和公式，并将工作表已用区域内的所有单元格统一设置为水平居中、垂直居中；禁止设置字体、字号、填充、边框、数字格式、自动换行、斑马纹、行高和列宽等其他样式。
- 导入前只校验本地 `.xlsx` 是否存在且非空、能否正常打开，以及工作表名称、数量、关键数据和公式是否正确。校验不通过时先修复本地文件，不得导入未验证产物。
- 调用下方「导入在线表格」的 `wecom-cli doc import`，把已校验的 `.xlsx` 导入为企微在线表格，并确认 `task_status` 为成功状态。
- 导入成功后，读取 `sheet-visual-standards.md`，严格按照其中的【整体执行流程】设置字体、数字格式、斑马纹、行高、列宽和自动换行。
- 完成后必须按照 `sheet-visual-standards.md` 的【验收标准】回读验证，特别检查在线表格的样式、行高和列宽；缺少任何适用检查时，禁止报告“已完成”。
- 向用户返回企微在线表格 URL。除非用户明确要求，否则禁止返回或向用户暴露本地 `.xlsx` 中间文件。

### 使用规则

- 新建流程适用于从零创建表格，以及用户对当前任务中刚创建的表格不满意并提出修改意见的情况；后者即使表述为“修改”，也应按新建流程重新生成。仅当操作对象是此前已存在的企微在线表格时，才使用读取、修改、追加、子表管理等在线接口，禁止重新生成 XLSX 后覆盖导入。

## 导入在线表格

把本地文件（`.csv` / `.xls` / `.xlsx`）导入为企微在线表格。

### 命令

```bash
wecom-cli doc import --json '<JSON 参数>'
```

### 参数

| 字段          | 类型 | 必填 | 默认值 | 语义                      |
|-------------|---|---|---|-------------------------|
| `doc_type`  | string | 是 | — | 导入文档类型，固定为`sheet`       |
| `file_name` | string | 是 | — | 二进制文件名（含后缀），用于业务判断源文件类型 |
| `file_path` | string | 是 | — | 源文件的本地绝对路径              |
| `passwd`    | string | 否 | — | Office 文件加密密码（若有）       |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `docid` | string | 导入完成后的表格 ID |
| `url` | string | 导入完成后的访问链接 |
| `task_status` | string | 任务状态枚举，如 `succ` 成功 |

### 使用规则

- 区分导入来源：本命令同时用于两种场景——作为「新建在线表格」标准流程的导入步骤（文件由从零生成），以及直接导入用户已有文件。前者按新建流程继续美化；后者已有文件导入后保持原样：不读取 `sheet-visual-standards.md`，不进行任何样式修改操作。例外：只有用户明确要求美化或调整格式时，才在导入后按用户指定的范围修改样式。

## 获取工作簿信息

列出工作簿中的全部子表，返回子表 ID、名称和行列范围，是后续子表操作的探测入口。

### 命令

```bash
wecom-cli sheet workbook get --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `docid` | string | 是 | — | 在线表格 ID 或 URL |

### 返回

| 字段                      | 类型       | 说明                                     |
|-------------------------|----------|----------------------------------------|
| `sheets`                | object[] | 子表列表                                   |
| `sheets[].sheet_id`     | string   | 子表 ID        |
| `sheets[].name`        | string   | 子表名称                                   |
| `sheets[].range`        | string   | 子表实际数据范围；空表返回 `A1:A1`                    |

### 使用规则

- 工作簿内存在同名子表、用户只描述了模糊名称，或后续操作具有破坏性时，应先通过本接口拿到 `sheet_id` 再操作。
- **`A1:A1` 需读取确认**：该范围既可能表示空表，也可能表示只有 A1 单元格有内容；不得仅凭 `range` 判定为空表，应使用 `cells get` 读取 `A1:A1` 后确认。

## 新增子表

### 命令

```bash
wecom-cli sheet subsheets create --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义                |
|---|---|---|---|-------------------|
| `docid` | string | 是 | — | 在线表格 ID 或 URL     |
| `name` | string | 否 | 引擎生成默认名 | 子表名称，长度不超过 31 个字符 |
| `position` | int | 否 | 追加到末尾 | 插入位置，1-based      |
| `dry_run` | bool | 否 | `false` | `true` 时仅预览，不落盘   |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `sheet_id` | string | 新建子表 ID |
| `name` | string | 实际子表名称 |
| `position` | int | 实际插入位置，1-based |

### 示例

```bash
wecom-cli sheet subsheets create --json '{"docid":"e3_ABC123","name":"2026汇总","position":2}'
```

### 使用规则

- 需要空白子表时使用本接口；已有模板且需要继承结构与格式时优先使用 `sheet subsheets copy`。
- 从零新建子表并写入数据后，需要读取 `sheet-visual-standards.md`，设置字体、填充、对齐、数字格式、斑马纹、行高、列宽和自动换行。设置完后需要回读验证，特别检查在线表格的样式、行高和列宽；缺少任何适用检查时，禁止报告“已完成”。
- 仅新建空白子表、通过 `sheet subsheets copy` 复制已有子表，或操作此前已存在的子表时，不执行上述美化流程。

## 删除子表

### 命令

```bash
wecom-cli sheet subsheets remove --json '<JSON 参数>'
```

### 参数

| 字段         | 类型 | 必填 | 默认值 | 语义              |
|------------|---|---|---|-----------------|
| `docid`    | string | 是 | — | 在线表格 ID 或 URL   |
| `sheet` | string | 是 | — | 待删除子表 ID        |
| `yes` | bool | 是 | — | 确认执行（缺省拒绝）      |
| `dry_run`  | bool | 否 | `false` | `true` 时仅预览，不落盘 |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `sheet_id` | string | 已删除子表 ID |
| `name` | string | 已删除子表名称 |

## 重命名子表

### 命令

```bash
wecom-cli sheet subsheets rename --json '<JSON 参数>'
```

### 参数

| 字段         | 类型 | 必填 | 默认值 | 语义 |
|------------|---|---|---|---|
| `docid`    | string | 是 | — | 在线表格 ID 或 URL |
| `sheet` | string | 是 | — | 子表 ID |
| `name`     | string | 是 | — | 新名称，长度不超过 31 个字符 |
| `dry_run`  | bool | 否 | `false` | `true` 时仅预览，不落盘 |

### 返回

| 字段         | 类型 | 说明 |
|------------|---|---|
| `sheet_id` | string | 子表 ID |
| `name` | string | 重命名后的名称 |

## 复制子表

新复制的子表，默认放在源子表的右侧

### 命令

```bash
wecom-cli sheet subsheets copy --json '<JSON 参数>'
```

### 参数

| 字段                | 类型 | 必填 | 默认值 | 语义 |
|-------------------|---|---|---|---|
| `docid`           | string | 是 | — | 在线表格 ID 或 URL |
| `sheet` | string | 是 | — | 源子表 ID |
| `name`        | string | 否 | 引擎生成默认名 | 副本名称 |
| `dry_run`         | bool | 否 | `false` | `true` 时仅预览，不落盘 |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `sheet_id` | string | 新副本的子表 ID |

## 移动子表顺序

### 命令

```bash
wecom-cli sheet subsheets move --json '<JSON 参数>'
```

### 参数

| 字段         | 类型 | 必填 | 默认值 | 语义 |
|------------|---|---|---|---|
| `docid`    | string | 是 | — | 在线表格 ID 或 URL |
| `sheet` | string | 是 | — | 子表 ID |
| `position`   | int | 是 | — | 目标位置，1-based |
| `dry_run`  | bool | 否 | `false` | `true` 时仅预览，不落盘 |

## 显示或隐藏子表 — `wecom-cli sheet subsheets visible`

设置指定子表显示或隐藏。隐藏子表不会删除内容，适合收起辅助计算表、中间过程表或暂不展示的历史表。

### 命令

```bash
wecom-cli sheet subsheets visible --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `docid` | string | 是 | — | 在线表格 ID 或 URL |
| `sheet` | string | 是 | — | 子表 ID 或名称 |
| `visible` | bool | 是 | — | `true` 表示显示，`false` 表示隐藏 |
| `dry_run` | bool | 否 | `false` | `true` 时仅预览，不落盘 |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `sheet_id` | string | 子表 ID |
| `visible` | bool | 当前可见状态 |

### 示例

```bash
wecom-cli sheet subsheets visible --json '{"docid":"e3_ABC123","sheet":"销售","visible":false}'
```

### 使用规则

- **隐藏不是删除**：用户说“先收起来/隐藏辅助表/不要展示给使用者”时使用本命令；不要用删除子表代替隐藏。
- **先确认目标子表**：隐藏前调用 `workbook get` 获取稳定 `sheet_id`，避免隐藏错同名或相似名称子表。
- **谨慎隐藏关键表**：隐藏源数据、公式依赖表或唯一入口表前，先向用户确认影响。
- **不能隐藏最后一个可见子表**：实测隐藏最后一个可见子表会失败，错误信息为 `cannot hide the last visible sheet`。
- **dry-run 响应同形**：`dry_run:true` 的响应结构与正式执行一致，但不落盘。
- **无法只读回读**：实测 `workbook get` 的 `sheets[]` 不返回 `visible` 字段。写后只能以 `subsheets visible` 响应确认。
