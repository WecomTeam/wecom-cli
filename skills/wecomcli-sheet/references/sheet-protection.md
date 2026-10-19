# 保护区域管理

对**在线表格**中的保护区域执行新增、查询、更新、删除。用户要求“锁定表头 / 保护公式区 / 限制别人编辑某块区域”时，优先使用本文件中的真实保护能力，禁止用静态样式、提示文字或隐藏列来冒充权限控制。

本文覆盖以下 CLI：

| 命令 | 风险 | 用途 |
|---|---|---|
| `wecom-cli sheet protects add` | 结构写入 | 新增受保护区域 |
| `wecom-cli sheet protects update` | 高风险写入 | 按 ID 修改保护区域范围 |
| `wecom-cli sheet protects delete` | 高风险写入 | 按 ID 删除保护区域 |
| `wecom-cli sheet protects list` | 只读 | 查询子表内保护区域清单 |

> **适用边界**：仅支持普通 `sheet` 类型子表，不支持智能表。`sheet` 使用子表 ID 或名称；`range` 一律使用纯 A1，不包含子表名。

> **通用口径**：结构性写入可先传 `dry_run: true` 预览，预览不代表已经执行。更新和删除保护区域必须显式传 `yes: true`。

## 新增保护区域 — `wecom-cli sheet protects add`

新增一个受保护区域。当前 CLI 要求显式传入 `range`。

### 命令

```bash
wecom-cli sheet protects add --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `docid` | string | 是 | — | 在线表格 ID / URL |
| `sheet` | string | 是 | — | 子表 ID 或名称，必须是普通 `sheet` 类型 |
| `range` | string | 是 | — | 保护区域，纯 A1，如 `A1:C1` |
| `dry_run` | bool | 否 | `false` | `true` 时仅预览，不落盘 |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `protect_range_id` | string | 新保护区域 ID；后续 `protects update` / `protects delete` 使用 |
| `range` | string | 实际生效区域，纯 A1 |

### 示例

```bash
wecom-cli sheet protects add --json '{"docid":"e3_ABC123","sheet":"销售","range":"A1:C1","dry_run":true}'
```

### 使用规则

- **必须传范围**：实测省略 `range` 会返回“字段 'range' 必填且不能为空”。当前 CLI 不支持通过省略 `range` 来保护整张子表；若用户要求整表保护，应先确认可用范围再传显式 A1 区域。

## 更新保护区域 — `wecom-cli sheet protects update`

按 `protect_range_id` 修改保护范围。`range` 是替换语义，不是增量扩展。

### 命令

```bash
wecom-cli sheet protects update --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `docid` | string | 是 | — | 在线表格 ID / URL |
| `sheet` | string | 是 | — | 保护区域所在子表 ID 或名称 |
| `protect_range_id` | string | 是 | — | 目标保护区域 ID；由 `protects add` / `protects list` 获得 |
| `range` | string | 是 | — | 新保护区域，纯 A1，替换原范围 |
| `yes` | bool | 是 | — | 确认执行；必须显式传 `true` |
| `dry_run` | bool | 否 | `false` | `true` 时仅预览，不落盘 |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `protect_range_id` | string | 保护区域 ID |
| `range` | string | 更新后区域，纯 A1 |

### 示例

```bash
wecom-cli sheet protects update --json '{"docid":"e3_ABC123","sheet":"销售","protect_range_id":"protect_001","range":"A1:D1","yes":true,"dry_run":true}'
```

### 使用规则

- **先 list 再改**：更新前调用 `protects list` 定位 `protect_range_id`，不要根据数组位置或记忆中的旧 ID 修改。
- **替换语义**：新 `range` 会覆盖原保护范围。扩大或缩小保护区都可能影响协作权限，先 `dry_run` 再正式执行。
- **写后回读**：正式更新后再次调用 `protects list`，确认目标 ID 的区域已变更。

## 删除保护区域 — `wecom-cli sheet protects delete`

按 `protect_range_id` 删除指定保护区域。

### 命令

```bash
wecom-cli sheet protects delete --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `docid` | string | 是 | — | 在线表格 ID / URL |
| `sheet` | string | 是 | — | 保护区域所在子表 ID 或名称 |
| `protect_range_id` | string | 是 | — | 目标保护区域 ID |
| `yes` | bool | 是 | — | 删除确认；必须显式传 `true` |
| `dry_run` | bool | 否 | `false` | `true` 时仅预览，不落盘 |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `protect_range_id` | string | 已删除的保护区域 ID |

### 示例

```bash
wecom-cli sheet protects delete --json '{"docid":"e3_ABC123","sheet":"销售","protect_range_id":"protect_001","yes":true,"dry_run":true}'
```

### 使用规则

- **删除不可逆**：先调用 `protects list` 核对目标 ID 与范围，再传 `dry_run: true` 预览。
- **禁止按区域盲删**：若多个保护区域范围相近或相交，应列出候选让用户确认，最终按稳定 `protect_range_id` 删除。

## 查询保护区域 — `wecom-cli sheet protects list`

列出指定子表中的全部保护区域。

### 命令

```bash
wecom-cli sheet protects list --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `docid` | string | 是 | — | 在线表格 ID / URL |
| `sheet` | string | 是 | — | 子表 ID 或名称，必须是普通 `sheet` 类型 |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `protect_ranges` | array | 保护区域清单；无保护区域时当前实测可能缺失该字段 |
| `protect_ranges[].protect_range_id` | string | 保护区域 ID |
| `protect_ranges[].range` | string | 保护区域，纯 A1 |

### 示例

```bash
wecom-cli sheet protects list --json '{"docid":"e3_ABC123","sheet":"销售"}'
```

### 使用规则

- **不返回名称**：返回值没有 `name`，不能按备注名直接定位；用 `range` 辅助识别，用 `protect_range_id` 执行更新或删除。
- **操作前置**：更新和删除保护区域前必须先通过本命令拿到当前 ID 与范围。

## 能力边界

- 当前 CLI 不支持通过省略 `range` 创建整表保护；`protects add` 必须传显式 A1 区域。
- 当前协议没有声明保护区域的名字、成员、角色、例外编辑者或密码字段；不要自行构造权限明细参数。
