# 公式预计算

对**在线表格**中的公式进行批量试算并返回结果，不修改文档内容。适用于写入公式前的预计算、校验公式语义、检查相对引用在指定锚点下的结果，以及确认数组公式展开结果。

> **依赖关系**：`sheet` 使用子表 ID 或名称；公式必须以 `=` 开头；`anchor` 使用单个纯 A1 单元格，不包含子表名。

## 使用场景

| 操作需求 | CLI | 说明 |
| --- | --- | --- |
| 写入前试算公式 | `sheet formulas calculate` | 不落盘，只返回公式结果 |
| 批量校验公式是否报错 | `sheet formulas calculate` | 通过 `errors`、`eval_error` 和 `cells[].is_error` 判断 |

## 试算公式

### 命令

```bash
wecom-cli sheet formulas calculate --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `docid` | string | 是 | — | 在线表格 ID / URL，仅支持 sheet 类型文档 |
| `sheet` | string | 是 | — | 默认子表 ID 或名称；优先按 ID 匹配，未命中按名称匹配 |
| `formulas` | array | 是 | — | 待试算公式数组，单次 1-100 条；元素结构见下方 `formulas[]` |

`formulas[]` 对象结构：

| 子字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `formula` | string | 是 | — | 公式原文，必须以 `=` 开头，如 `=SUM(A1:A10)` |
| `anchor` | string | 否 | `A1` | 相对引用的锚点单元格，纯 A1；不传时按 `A1` 计算 |
| `sheet` | string | 否 | 请求级 `sheet` | 子表 ID 或名称 |

### 返回

| 字段 | 类型 | 说明                                         |
|---|---|--------------------------------------------|
| `total` | int | 请求公式总数                                     |
| `errors` | int | 计算失败的公式条数                                  |
| `results` | array | 逐条结果，与请求 `formulas` 顺序一致                   |
| `results[].index` | int32 | 对应请求 `formulas` 的下标，从 0 开始                 |
| `results[].sheet_id` | string | 实际使用的子表 ID                                 |
| `results[].anchor` | string | 实际使用的锚点单元格 A1                              |
| `results[].cells` | array | 结果单元格；数组公式会展开为多个单元格                        |
| `results[].cells[].address` | string | 结果单元格 A1 地址                                |
| `results[].cells[].value` | string / number / bool / null | 计算值；公式错误时为错误码字符串                           |
| `results[].cells[].is_error` | bool | `true` 表示 `value` 为公式错误码，如 `#REF!`、`#DIV/0!` |
| `results[].eval_error` | string | 整条公式无法评估（语法错误、sheet 不存在等）   |

### 示例

```bash
wecom-cli sheet formulas calculate --json '{"docid":"DOCID_EXAMPLE","sheet":"Sheet1","formulas":[{"formula":"=1+2"}]}'
```

### 使用规则

- **不修改文档**：本命令只试算公式，不写入单元格。用户要把公式真正写进表格时，使用 `sheet-write-cells.md` 中的 `cells set` 或 `cells batchset`。
- **公式必须带等号**：`formula` 必须以 `=` 开头；不要传裸表达式 `1+2`。
- **批量上限**：单次 `formulas` 数组为 1-100 条；超过 100 条时分批调用，并按 `index` 对回原请求顺序。
- **相对引用看锚点**：未传 `anchor` 时按 `A1` 计算；公式包含相对引用时，按用户目标单元格或计划写入位置设置 `anchor`。
- **错误分两类**：`eval_error` 表示条目级计算失败；`cells[].is_error=true` 表示公式成功计算但结果是公式错误码。

## 能力边界

- 本接口不创建、修改或保存公式；只用于预计算和校验。
- 不返回依赖单元格清单或完整计算链路。
