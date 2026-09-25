# 修改表格内容 — `wecom-cli sheet contents update`

修改**在线表格**指定区域的内容与格式，通过 `grid_data` 指定写入的起始位置与各单元格数据。

## 命令

```bash
wecom-cli sheet contents update --json '<JSON 参数>'
```

## 参数

| 字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `docid` | string | 是 | — | 在线表格 ID |
| `sheet_id` | string | 是 | — | 工作表 ID；通过 `sheet get` 获取 |
| `grid_data` | object | 是 | — | 写入区域的数据 |
| `grid_data.start_row` | int | 是 | — | 起始行号，从 0 起 |
| `grid_data.start_column` | int | 是 | — | 起始列号，从 0 起 |
| `grid_data.rows` | array | 是 | — | 各行数据 |

`grid_data.rows[].values[]` 对象结构：

| 子字段 | 类型 | 说明 |
|---|---|---|
| `cell_value` | object | 单元格值，见下方「cell_value 类型选择」 |
| `data_type` | string | 与 `cell_value` 对应的数据类型 |
| `cell_format` | object | 单元格样式；传空对象 `{}` 表示默认样式 |

### cell_value 类型选择

| 形态 | 结构 | 适用场景 |
|---|---|---|
| `text` | `{"text": "<纯文本>"}` | 纯文本内容（如姓名、说明、标签、编号字符串等） |
| `number` | `{"number": 123.45}` | 数值，用于金额、数量、比率等需要参与公式计算或聚合的数据；值为 JSON 数字类型，不加引号 |
| `formula` | `{"formula": "=SUM(A1,A2)"}` | 任何以 `=` 开头的公式，包括 `=SUM(...)`、`=A1+B1`、`=IF(...)`、`=VLOOKUP(...)` 等 |
| `link` | `{"link": {"url": "<URL>", "text": "<显示文本>"}}` | 超链接 |

## 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `grid_data` | object | 写入的数据，结构与入参 `grid_data` 一致 |

## 使用规则

- **索引与行映射**：`start_row` / `start_column` 均从 0 起；`grid_data.rows[0]` 必须原样写入 `start_row` 指定的行，后续元素逐行递增。禁止因目标区域含表头而跳过 `rows[0]`，也禁止在未读取现有内容时自行把起始行加一。
- **写后读回校验**：写入前记录目标区域的期望二维单元格矩阵；接口返回成功后，必须按 `sheet-ranges-get.md` 读取刚写入的完整目标区域，逐行逐列比较单元格值与预期。只有完全一致才能报告修改成功；少行、错位或内容不一致时报告失败和具体差异，不得只凭写接口成功响应下结论。
- **格式与已有内容对齐**：向已有内容的表格追加数据时，新行的样式应尽量与现有表格保持一致，避免出现字体、字号、对齐、边框、底色等风格突兀的行。
