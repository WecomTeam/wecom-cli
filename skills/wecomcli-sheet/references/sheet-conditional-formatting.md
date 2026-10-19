# 条件格式管理

对**在线表格**的条件格式规则执行新增、更新、删除与按区域查询。条件格式是独立规则对象：用户要求“按条件标红 / 高亮 / 标记重复值 / 突出前 N 项 / 标出平均值以上数据”时，必须使用本文件中的 `cf` 命令，禁止用静态单元格样式代替。

> **依赖关系**：`sheet` 使用子表 ID 或名称；`range` 一律使用纯 A1，不包含子表名。按字段名称配置规则前，先读取表头与少量样本确认目标列及数据类型。

> **通用口径**：结构性写入可先传 `dry_run: true` 预览，预览不代表已经执行。

## 新增条件格式 — `wecom-cli sheet cf add`

在指定区域新增一条条件格式规则。

### 命令

```bash
wecom-cli sheet cf add --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `docid` | string | 是 | — | 在线表格 ID / URL |
| `sheet` | string | 是 | — | 子表 ID 或名称 |
| `range` | string | 是 | — | 规则作用区域，纯 A1，如 `C2:C100` |
| `cf_type` | string | 是 | — | 规则类型：`cell_is` / `unique` / `duplicate` / `top10` / `above_average` |
| `operator` | string | 条件必填 | — | `cf_type="cell_is"` 时必填 |
| `values` | array<string> | 条件必填 | — | `cf_type="cell_is"` 时必填；数量由 `operator` 决定 |
| `rank` | int | 条件必填 | — | `cf_type="top10"` 时必填 |
| `bottom` | bool | 否 | `false` | `cf_type="top10"` 时，是否从底部统计 |
| `percent` | bool | 否 | `false` | `cf_type="top10"` 时，是否按百分比统计 |
| `above_average` | bool | 否 | — | `cf_type="above_average"` 时，控制高于 / 低于平均值 |
| `equal_average` | bool | 否 | — | `cf_type="above_average"` 时，是否包含等于平均值 |
| `std_dev` | int | 否 | — | `cf_type="above_average"` 时，标准差偏移 |
| `font_color` | string | 否 | — | 命中后的字体颜色，ARGB hex |
| `bg_color` | string | 否 | — | 命中后的背景颜色，ARGB hex |
| `bold` | bool | 否 | — | 命中后加粗 |
| `italic` | bool | 否 | — | 命中后斜体 |
| `underline` | bool | 否 | — | 命中后加下划线 |
| `strikethrough` | bool | 否 | — | 命中后加删除线 |
| `dry_run` | bool | 否 | `false` | `true` 时仅预览，不落盘 |

`operator` 合法取值：

| 取值 | 含义 | `values` 数量 |
|---|---|---|
| `greater` | 大于 | 1 |
| `less` | 小于 | 1 |
| `equal` | 等于 | 1 |
| `greater_equal` | 大于等于 | 1 |
| `less_equal` | 小于等于 | 1 |
| `not_equal` | 不等于 | 1 |
| `between` | 介于 | 2 |
| `not_between` | 不介于 | 2 |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `rule_id` | string | 新增规则 ID；后续 `cf update` / `cf delete` 使用 |
| `range` | string | 实际生效区域 |
| `cf_type` | string | 实际规则类型 |

### 示例

```bash
wecom-cli sheet cf add --json '{"docid":"e3_ABC123","sheet":"销售","range":"C2:C100","cf_type":"cell_is","operator":"greater","values":["1000"],"bg_color":"FFFF0000","bold":true,"dry_run":true}'
```

### 使用规则

- **样式至少一项**：`font_color` / `bg_color` / `bold` / `italic` / `underline` / `strikethrough` 至少传一项，否则服务端拒绝。
- **颜色格式统一**：颜色使用 ARGB hex，例如红色为 `FFFF0000`，不要传 `#FF0000`。
- **类型字段分派**：只传当前 `cf_type` 对应的专属字段；不要把 `rank` 填进 `cell_is`，也不要为 `unique` 构造 `operator`。
- **能力边界**：当前协议不支持数据条、色阶、图标集或自定义公式条件，禁止自行拼接未声明类型。
- **写后保留原参数**：保存完整新增参数。后续更新是整体替换，且 `cf list` 无法读取完整规则内容。
- **写后定位校验**：新增成功后调用 `cf list` 查询目标区域，确认返回的 `rule_id` 与 `ranges` 正确。

## 更新条件格式 — `wecom-cli sheet cf update`

修改已有条件格式规则。更新为**整体替换**，不是局部 patch。

### 命令

```bash
wecom-cli sheet cf update --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `docid` | string | 是 | — | 在线表格 ID / URL |
| `sheet` | string | 是 | — | 子表 ID 或名称 |
| `rule_id` | string | 是 | — | 目标规则 ID；由 `cf add` / `cf list` 获得 |
| `range` | string | 是 | — | 新作用区域，纯 A1，如 `C2:C100` |
| `cf_type` | string | 是 | — | 新规则类型 |
| `operator` / `values` | mixed | 条件必填 | — | `cell_is` 规则的完整条件 |
| `rank` / `bottom` / `percent` | mixed | 条件必填 | — | `top10` 规则的完整条件 |
| `above_average` / `equal_average` / `std_dev` | mixed | 否 | — | `above_average` 规则参数 |
| `font_color` / `bg_color` | string | 否 | — | 新颜色，ARGB hex |
| `bold` / `italic` / `underline` / `strikethrough` | bool | 否 | — | 新字体样式 |
| `yes` | bool | 是 | — | 整体替换确认；必须显式传 `true` |
| `dry_run` | bool | 否 | `false` | `true` 时仅预览，不落盘 |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `rule_id` | string | 条件格式规则 ID |
| `range` | string | 更新后作用区域 |
| `cf_type` | string | 更新后规则类型 |

### 示例

```bash
wecom-cli sheet cf update --json '{"docid":"e3_ABC123","sheet":"销售","rule_id":"cf_001","range":"C2:C100","cf_type":"cell_is","operator":"greater","values":["2000"],"bg_color":"FFFF0000","bold":true,"yes":true,"dry_run":true}'
```

### 使用规则

- **整体替换**：传入配置会覆盖原规则，不要把本命令当成局部更新。
- **无法先查后补全**：`cf list` 只返回 `rule_id` / `priority` / `ranges`，不返回 `cf_type`、条件值或样式。更新时必须使用先前保存的完整创建参数，或由用户提供完整目标配置。
- **禁止伪装局部修改**：缺少旧配置时，不得承诺“只改颜色并保留其他未知字段”；应明确说明将用新配置整体覆盖。
- **先预览再执行**：先传 `dry_run: true` 检查目标与请求；正式执行时保留 `yes: true`，移除 `dry_run` 或设为 `false`。
- **写后校验**：再次调用 `cf list`，确认目标 `rule_id` 仍存在且区域符合预期。查询接口不返回条件与样式，不得声称已通过 list 验证这些字段。

## 删除条件格式 — `wecom-cli sheet cf delete`

按稳定规则 ID 删除一条或多条条件格式规则。

### 命令

```bash
wecom-cli sheet cf delete --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `docid` | string | 是 | — | 在线表格 ID / URL |
| `sheet` | string | 是 | — | 子表 ID 或名称 |
| `rule_ids` | array<string> | 是 | — | 要删除的规则 ID 列表 |
| `yes` | bool | 是 | — | 删除确认；必须显式传 `true` |
| `dry_run` | bool | 否 | `false` | `true` 时仅预览，不落盘 |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `removed_rule_ids` | array<string> | 实际删除的规则 ID 列表 |

### 示例

```bash
wecom-cli sheet cf delete --json '{"docid":"e3_ABC123","sheet":"销售","rule_ids":["cf_001","cf_002"],"yes":true,"dry_run":true}'
```

### 使用规则

- **删除不可逆**：先用 `cf list` 定位规则，禁止根据数组位置或猜测 ID 删除。
- **先预览再执行**：先传 `dry_run: true`；确认无误后再正式执行。
- **结果必须核对**：检查 `removed_rule_ids` 是否包含全部目标 ID，再调用 `cf list` 确认这些规则已不再返回。

## 查询条件格式 — `wecom-cli sheet cf list`

列出子表全部条件格式规则，或只返回与指定区域相交的规则。

### 命令

```bash
wecom-cli sheet cf list --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `docid` | string | 是 | — | 在线表格 ID / URL |
| `sheet` | string | 是 | — | 子表 ID 或名称 |
| `range` | string | 否 | — | 区域过滤；只返回与该区域相交的规则，不传则返回子表全部规则 |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `rules` | array | 条件格式规则定位信息 |
| `rules[].rule_id` | string | 规则 ID |
| `rules[].priority` | int | 优先级；`0` 最高，越小越优先 |
| `rules[].ranges` | array<string> | 规则当前生效的全部纯 A1 区域 |

### 示例

```bash
wecom-cli sheet cf list --json '{"docid":"e3_ABC123","sheet":"销售","range":"C2:F100"}'
```

### 使用规则

- **区域语义是相交**：规则范围只要与查询区域相交就会返回，不要求完全包含。
- **只用于定位**：返回值不含规则类型、判断条件、比较值或样式，不能用于还原完整规则。
- **优先级解释**：`priority` 越小越优先，`0` 为最高。
