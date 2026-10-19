# 版本历史管理

查询**在线表格工作簿级别**的当前版本号与历史版本，并将整个工作簿还原到指定历史版本。

| 命令 | 风险 | 用途 |
|---|---|---|
| `wecom-cli sheet version get` | 只读 | 获取当前文档版本号 |
| `wecom-cli sheet version list` | 只读 | 查询文档历史版本 |
| `wecom-cli sheet version rollback` | 高风险写入 | 将整个工作簿还原到指定历史版本，并生成一个新版本 |

## 通用规则

- **版本作用于整个工作簿**：版本查询与回滚均以 `docid` 为操作对象，不能只回滚某个子表或单元格区域。
- **先查询再回滚**：执行回滚前，先调用 `version get` 获取当前版本，再调用 `version list` 确认目标版本确实存在，并向用户说明当前版本与目标版本。
- **明确目标版本**：用户未指定版本号、版本号不存在或存在多个语义候选时，先列出历史版本供用户选择，禁止猜测。
- **回滚会生成新版本**：回滚不会把当前版本号直接改成目标历史版本号，而是以目标版本内容生成一个新的当前版本。

## 获取当前版本号

获取指定在线表格当前的文档版本号。

### 命令

```bash
wecom-cli sheet version get --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `docid` | string | 是 | — | 在线表格 ID 或 URL |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `version` | int | 当前文档版本号 |

### 使用规则

- 回滚前必须读取当前版本号，用于确认操作起点。
- 回滚后再次调用本接口；返回版本号应与回滚响应中的 `new_version` 一致。

## 查询历史版本

分页查询指定在线表格的历史版本，返回版本号和修改时间。

### 命令

```bash
wecom-cli sheet version list --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义                                  |
|---|---|---|---|-------------------------------------|
| `docid` | string | 是 | — | 在线表格 ID 或 URL                       |
| `limit` | int | 否 | — | 单页返回数量                              |
| `cursor` | string | 否 | `""` | 分页游标；首次传空字符串，后续传上一页返回的 `next_cursor` |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `versions` | object[] | 历史版本列表 |
| `versions[].version` | int | 历史版本号 |
| `versions[].modify_time` | string | 该版本的修改时间，格式示例：`2026-09-21 20:00:00` |
| `note` | string | 接口提示信息 |
| `next_cursor` | string | 下一页游标；为空字符串时表示没有下一页 |

### 示例

```bash
wecom-cli sheet version list --json '{"docid":"DOCID_EXAMPLE","limit":20,"cursor":""}'
```

### 使用规则

- **按游标翻页**：只要 `next_cursor` 非空，就把它作为下一次请求的 `cursor` 继续查询；`next_cursor` 为空时停止。

## 还原指定版本

将指定在线表格的全部内容还原到某个历史版本。执行后保留原版本历史，并生成一个新的当前版本。

### 命令

```bash
wecom-cli sheet version rollback --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `docid` | string | 是 | — | 在线表格 ID 或 URL |
| `version` | int | 是 | — | 要还原到的历史版本号；必须来自 `version list` 的实际返回 |
| `yes` | bool | 是 | — | 回滚确认；必须显式传 `true` |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `previous_version` | int | 回滚前的当前版本号 |
| `new_version` | int | 回滚操作生成的新版本号 |
| `rollback_to` | int | 本次还原所依据的历史版本号 |

### 示例

```bash
wecom-cli sheet version rollback --json '{"docid":"DOCID_EXAMPLE","version":12,"yes":true}'
```

### 使用规则

- **高风险操作**：回滚影响整个工作簿。只有用户明确要求把目标文档回滚到具体版本时，才传 `"yes":true`；仅要求“看看历史版本”时不得回滚。
- **响应一致性检查**：确认 `previous_version` 等于回滚前 `version get` 的结果，`rollback_to` 等于用户指定版本，且 `new_version` 为回滚后 `version get` 返回的当前版本。
