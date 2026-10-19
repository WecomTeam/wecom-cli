# 表格数据写操作

## 图片写入前置转换

单元格的 `image` 字段只接受图片 URL。若源图片是本地文件或 `media_id`，先调用 `wecom-cli sheet images upload` 上传到目标文档空间，获取可用 URL。

```bash
wecom-cli sheet images upload --json '{"file_path": "/path/to/image.png", "docid": "<目标文档ID>"}'
```

```bash
wecom-cli sheet images upload --json '{"media_id": "<media_id>", "docid": "<目标文档ID>"}'
```

`file_path` 与 `media_id` 二选一，不能同时传入。拿到返回的 `url` 后，再把它填入下方任一写图片命令的 `image` 字段；不要把本地路径或 `media_id` 直接填入 `image`。

### 单元格图片能力边界

- 单元格图片只支持通过 `image` 传入图片 URL，不支持设置 `width`、`height` 或缩放比例。
- 用户指定图片宽高、像素或显示尺寸时，必须直接读取 `sheet-float-image.md`，使用 `sheet float image set`，并将用户指定的单元格位置作为 `anchor`。

## 写入边界与回读校验参数

- **`verify`**：写后自动读回本次写入区域，用于检查写入是否生效，并发现公式中的 `#REF!` / `#VALUE!` 等错误。它只校验当前请求，不代表多个批次组成的整表任务已完成，也不提供事务保证。
- **`allow_overwrite`**：默认 `true`，直接覆盖非空单元格；传 `false` 时目标区域存在非空单元格则报错。它只能防止覆盖非空格，不能识别目标值是否在读取后被其他人修改。
- **`dry_run`**：仅预览和校验本次请求，不执行写入。它不会锁定表格，也不保证正式执行时内容、范围和权限仍未变化。

> 多批写入是逐批提交，可能出现部分批次成功、后续批次失败。当前协议没有声明跨批事务或自动回滚能力。

## 新增或追加数据的强制前置流程

`cells batchset`、`cells set`、`cells append` 只负责写入数据，不负责保证新增区域继承现有表格样式。凡写入会新增记录、追加行或增加字段，必须先通过 `sheet-structure-operations.md` 的 `sheet dims insert` 插入承载数据的整行和/或整列，再执行写入。

适用边界：

- **必须插入**：新增记录、表尾追加、在现有记录之间插入新记录、增加新字段、向右或向下扩展数据区。
- **不得插入**：仅修改、纠正或补全已有单元格内容。此时直接写入原单元格，避免 `dims insert` 导致现有数据、公式和引用整体偏移。
- **禁止做法**：先直接调用 `batchset` / `set` / `append` 写入新增区域，再手工复制样式；或依赖写接口自动扩展边界、自动继承相邻样式。

## 共用数据结构：`Cell`

`cells batchset`、`cells set` 和 `cells append` 三个接口共用 `Cell` 数据结构。普通值可直接使用裸值，如 `"名称"`、`120`、`true`；写入时会自动包装为 `{"value": ...}`。需要写入公式、链接、图片、样式或富文本时，使用 `Cell` 对象。

| 字段 | 类型 | 说明 |
|---|---|---|
| `value` | any | 单元格值 |
| `formula` | string | 公式，只需传 `formula` |
| `link` | string | 超链接；传空字符串 `""` 清除该格所有链接，与 `value` 互斥 |
| `link_text` | string | 超链接展示文本，与 `link` 搭配使用 |
| `image` | string | 图片 URL；若源图片是本地路径或 `media_id`，先通过 `images upload` 换取 URL |
| `style` | object | 单元格样式，字段见下方「`CellStyle` 可设样式」 |
| `rich_string` | array | 富文本分段数组，一个单元格内混排多段文本、样式和超链接 |

### `Cell` 写入规则

为避免不同调用方对空值和组合字段产生不同解释，按以下规则调用：

1. `value`、`formula`、`link`、`image`、`rich_string` 视为内容字段；一次写入只选择一种主要内容类型，`style` 可以与主要内容一起传入。`link_text` 只能与 `link` 一起使用。
2. 批量写公式必须传 `Cell` 对象，如 `"cells": [[{"formula": "=SUM(A1:A10)"}]]`，裸字符串会被自动包装为文本，表格显示公式原文、不参与计算、`verify` 的 `verify_result.cells` 会原样回显公式文本。目前表格公式暂不支持进行跨文档引用和同一文档内跨子表引用（如VLOOKUP等函数）。
3. 公式写入的验收判定：写入前读 `sheet-formulas.md` 进行试算。写入后用 `sheet cells get` 带 `include:["formula"]` 读回。返回 `{"value": <数字>, "formula": "=..."}` → 公式生效。返回裸字符串，或 `value` 等于 `=...` 原文 → 该格是文本，公式未生效，必须用 `{"formula": ...}` 重写。`verify=true` 时 `verify_result.cells` 若原样回显 `=...` 文本，即为写入失败信号，不要当作成功。
4. 当前协议明确支持的清除语义只有：`link: ""` 清除链接、`number_format: ""` 清除数字格式。清除值、公式、图片或富文本需使用对应的清除命令或等待协议明确，不能自行猜测。

### `RichString` 元素结构

一个单元格由若干 `RichString` 段按顺序拼接而成，用于同一格内“文本 + 超链接混排”或“不同片段不同字体/颜色”。每段只能是一种内容：`value`（文本）与 `link`（链接）二选一。

| 字段 | 类型 | 说明 |
|---|---|---|
| `value` | any | 文本段的文本值 |
| `link` | string | 链接段的超链接 URL |
| `link_text` | string | 链接段的显示文字，与 `link` 搭配使用 |
| `style` | object | 该段样式，字段同下方 `CellStyle` |

> 放多个超链接时，拆成多段，每段使用一组 `link` + `link_text`，不要在一段里放数组或嵌套对象。

### `CellStyle` 可设样式

`Cell.style` 可携带以下字段，未传的字段保持原值：

| 字段 | 类型 | 说明 |
|---|---|---|
| `font_family` | string | 字体名称，如 `"Microsoft YaHei"` |
| `font_size` | int | 字号（pt） |
| `bold` | bool | 加粗 |
| `italic` | bool | 斜体 |
| `underline` | string | 下划线：`none` / `single` / `double` / `single_accounting` / `double_accounting` |
| `strike` | bool | 删除线 |
| `font_color` | string | 字体色，ARGB hex |
| `bg_color` | string | 背景色，ARGB hex |
| `horizontal_align` | string | 水平对齐：`general` / `left` / `center` / `right` / `fill` / `justify` / `center_continuous` / `distributed` |
| `vertical_align` | string | 垂直对齐：`top` / `center` / `bottom` / `justify` / `distributed` |
| `wrap` | bool | 自动换行 |
| `number_format` | string | 数字格式，Excel 格式代码，见下 |

**颜色 ARGB hex**：8 位十六进制，前 2 位为 alpha（不透明度）。如 `"FFFF0000"` = 不透明红、`"FFFFFF00"` = 不透明黄。

**`number_format`**：Excel 数字格式代码自由字符串（非枚举），只影响显示，不改变值类型；传空字符串 `""` 表示清除格式并恢复默认。常用取值：

| 分类 | 格式代码 | 显示效果 |
|---|---|---|
| 文本 | `@` | 文本显示格式；需要保留前导零或长编号时，仍应把值作为字符串传入 |
| 整数 | `#,##0` | `1,234` |
| 两位小数 | `#,##0.00` | `1,234.00` |
| 百分比 | `0%` / `0.00%` | `88%` / `87.65%`（值 `0.88`） |
| 货币 | `¥#,##0.00` / `$#,##0.00` | `¥1,234.00` |
| 日期 | `yyyy-mm-dd` / `yyyy"年"m"月"d"日` | `2026-08-28` / `2026年8月28日` |
| 时间 | `hh:mm:ss` | `13:45:30` |

分段语法：用 `;` 分隔最多 4 段，依次为正数、负数、零、文本，如 `#,##0.00;[Red]-#,##0.00`。

## 使用场景

| 场景 | 使用命令 | 原因 |
|---|---|---|
| 批量写入一块区域（多行多列） | `sheet cells batchset` | 支持 `cells` 二维数组或 `csv_data`；一次写入值、公式、链接、图片或样式 |
| 只写一个单元格 | `sheet cells set` | 单格写入操作 |
| 在表格末尾追加一行 | `sheet cells append` | 不需要自行计算末尾行号 |

`cells batchset` 的两种数据源二选一：

- 数据含公式、链接、图片、样式、富文本，或需要明确控制字符串类型时，使用 `cells`
- 纯二维值且不含复杂类型时，可以使用 `csv_data`

## 单次写入上限与分批原则

`cells batchset` 单次最多写入 **400 个单元格**。待写入数据超过 400 格时，必须在调用前拆分为多个批次。

### 连续矩形与非连续目标行

`cells batchset` 的 `range` 与二维 `cells` / `csv_data` 始终描述一个连续矩形区域。处理非连续目标地址时，按以下优先级选择方案：

1. 多个旧值分别统一替换为固定新值：改读 `sheet-search-replace.md`，使用限定子表、限定列范围的精确替换。
2. 每个目标值不同，但目标行可组成若干连续段：先按实际行号排序并拆成连续段，每个连续段单独调用 `cells batchset`。
3. 目标地址高度离散且数量很少：逐个调用 `cells set`。

### 二维分块

1. 优先按完整行分批：`rows_per_batch = floor(400 / total_columns)`。
2. 若单行超过 400 列，先按列拆分，每个列片不超过 400 列，再在列片内按行拆分。
3. 每批必须满足：`batch_rows × batch_columns <= 400`。
4. A1 列号换算必须支持 `Z → AA → AB`，禁止用单字符加减计算列名。

### 使用 Python 分批写入

Python 分批写入必须按“预览试跑 → 固定批次正式写入 → 失败批次重试 → 整体校验”的顺序执行。

写入前必须根据目标起始位置、总行数和总列数计算最终写入范围，并确认该范围未超出当前子表的行列边界。若当前边界不足，必须先按照 `sheet-structure-operations.md` 中的「扩展当前子表边界」扩展所需行或列；扩展完成后重新计算批次坐标，再进行预览试跑和正式写入，不得依赖写入操作自动扩展边界。

#### 预览试跑

先确认目标起始位置、总行数、总列数和预期写入格数。根据二维分块规则确定每批行数、列数和格数；每批不得超过 400 格。批次大小一旦通过预览试跑，正式运行期间不得根据上一批响应动态修改。

正式写入前，先写试运行的Python脚本，用 `"dry_run": true` 运行至少5个批次：

预览时使用与正式写入完全相同的 `range`、`cells/csv_data`、`allow_overwrite`，只增加 `"dry_run": true`。

- 所有代表批次均未出现参数过长、维度错误或其他参数错误，才认为当前分块大小合理，可以开始正式运行。
- 如果出现参数过长（Argument Long），缩小单批行数或列数，并重新运行代表批次预览；不能只修改报错批次后继续沿用旧坐标。
- 如果出现权限、非法公式、数据类型或矩阵维度错误，先修正请求或源数据；这类错误不能靠持续重试解决。

#### 失败处理与重试

- **明确执行失败**：重试当前批次；成功前不进入下一批。
- **限流或临时服务错误**：退避间隔重试同一批次。
- **执行超时**：超时表示结果未知，不等于写入失败。先回读该批固定 range：回读结果不一致或确认未写入，重试同一批次；
- 对可重试失败持续处理，直到全部批次通过；但权限、参数、非法公式、维度错误等永久错误必须停止并修正，禁止无限重试。 。

## 批量写入指定区域

### 命令

```bash
wecom-cli sheet cells batchset --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `docid` | string | 是 | 在线表格 ID |
| `sheet` | string | 是 | 子表 ID 或名称；自动化任务优先传 ID |
| `range` | string | 是 | 目标 A1 区域；推荐传左上角锚点，由 2D 数据维度推导实际区域 |
| `cells` | array（2D） | 条件 | 非空矩形二维数组，每元素为 `Cell`，裸值可简写；与 `csv_data` 二选一 |
| `csv_data` | string | 条件 | 标准 CSV 文本；与 `cells` 二选一，仍受 400 格上限约束 |
| `allow_overwrite` | bool | 否 | 默认 `true`；`false` 时目标存在非空单元格则报错 |
| `verify` | bool | 否 | 写后自动读回本次写入区域 |
| `dry_run` | bool | 否 | 预览本次请求，不执行写入 |

> 若传完整 A1 区域而不是左上角锚点，调用前必须保证区域维度与 2D 数据严格一致；当前返回没有定义维度不一致时的具体错误码，因此更推荐只传锚点。

### 请求示例（cells 模式 · 复杂类型）

```json
{
  "docid": "e3_ABC123",
  "sheet": "销售",
  "range": "A1",
  "cells": [
    [{"value": "名称"}, {"link": "https://x.com", "link_text": "看详情", "style": {"font_color": "FF0000FF", "underline": "single"}}],
    [{"value": "苹果"}, {"formula": "=SUM(C2:C10)"}],
    [{"value": "带样式", "style": {"bold": true, "bg_color": "FFFFFF00"}}, {"image": "https://x.com/logo.png"}]
  ],
  "verify": true
}
```

### 请求示例（rich_string 模式）

```json
{
  "docid": "e3_ABC123",
  "sheet": "销售",
  "range": "A1",
  "cells": [
    [{"rich_string": [
      {"value": "详情见 "},
      {"link": "https://x.com", "link_text": "官网", "style": {"font_color": "FF0000FF", "underline": "single"}},
      {"value": " 页面"}
    ]}]
  ]
}
```

### 请求示例（裸值简写）

```json
{
  "docid": "e3_ABC123",
  "sheet": "销售",
  "range": "A1",
  "cells": [["名称", "数量"], ["苹果", 120], ["香蕉", 80]]
}
```

### 请求示例（CSV 模式 · 纯数据）

```json
{
  "docid": "e3_ABC123",
  "sheet": "销售",
  "range": "A1",
  "csv_data": "名称,数量,单价\n苹果,120,3.5\n香蕉,80,2.8\n橙子,95,4.2"
}
```

### 返回

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `affected_range` | string | 是 | 实际写入区域 A1 |
| `written_count` | int32 | 是 | 写入单元格数 |
| `verify_result` | object | 否 | `verify=true` 时的读回结果：含 `affected_range`、`written_count` 和 `cells`（按区域行列顺序回读的单元格值二维数组），回读值与写入值相同才视为写入生效 |

## 单格写入

### 命令

```bash
wecom-cli sheet cells set --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `docid` | string | 是 | 工作簿定位 |
| `sheet` | string | 是 | 子表 ID 或名称 |
| `range` | string | 是 | 单格 A1，如 `B2` |
| `cell` | `Cell` | 是 | 写入内容，使用文首 `Cell` 规则 |
| `verify` | bool | 否 | 写后读回校验 |

### 返回

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `affected_range` | string | 是 | 实际写入单格 A1 |
| `cell` | Cell | 否 | `verify=true` 时读回的单元格内容 |

### 使用规则

- `cell.link` 传空字符串 `""` 即清除该格全部超链接，无需独立清链接命令。

## 表格末尾追加一行数据

在指定子表末尾追加一行数据，无需自行计算末尾行号。`row` 中各元素按子表列顺序依次写入。

```bash
wecom-cli sheet cells append --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `docid` | string | 是 | 工作簿定位 |
| `sheet` | string | 是 | 子表 ID 或名称 |
| `row` | `Cell[]` | 是 | 要追加的一行非空数据，按列顺序排列 |

`row` 中的复杂元素使用文首 `Cell` 规则，可使用 `value`、`formula`、`style` 等字段。

### 请求示例

```json
{
  "docid": "e3_ABC123",
  "sheet": "销售",
  "row": [
    "苹果",
    120,
    true,
    {
      "formula": "=SUM(A1:A10)",
      "style": {
        "bold": true,
        "number_format": "0.00"
      }
    }
  ]
}
```
