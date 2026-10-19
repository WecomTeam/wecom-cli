# 浮动图片处理

对**在线表格**子表中的浮动图片执行创建、替换、删除、批量插入与查询。浮动图片是悬浮在表格上的对象，通过锚点单元格定位并返回独立 `image_id`；它不同于 `sheet cells set` / `cells batchset` 中写入单元格的图片。

> **依赖关系**：`sheet` 使用子表 ID 或名称；`anchor` 使用单个纯 A1 单元格，如 `F50`，不包含子表名。图片字段使用可被文档访问的 URL。

## 图片 URL 前置转换

浮动图片的 `image` / `new_image` 字段只接受图片 URL。若源图片是本地文件或 `media_id`，先调用 `wecom-cli sheet images upload` 上传到目标文档空间，获取可用 URL。

上传本地图片时，将本地图片路径作为 string 传入 `file_path`：

```bash
wecom-cli sheet images upload --json '{"file_path":"/path/to/image.png","docid":"<文档ID>"}'
```

上传 `media_id` 对应的图片时：

```bash
wecom-cli sheet images upload --json '{"media_id":"<media_id>","docid":"<文档ID>"}'
```

**入参：**

| 字段 | 类型 | 必填 | 说明                                                                    |
|---|---|---|-----------------------------------------------------------------------|
| `docid` | string | 是 | 目标在线表格文档 ID                                                           |
| `file_path` | string | 条件必填 | 本地图片路径；与 `media_id` 二选一                                               |
| `media_id` | string | 条件必填 | 媒体文件 ID，来自用户消息或 `wecomcli-media` 技能的 `media upload`；与 `file_path` 二选一 |

**出参：**

| 字段 | 类型 | 说明 |
|---|---|---|
| `url` | string | 上传后的文件访问 URL；填入 `float image set` 的 `image` 或 `float image update` 的 `new_image` |

> `file_path` 与 `media_id` 不能同时传入。不要把本地路径或 `media_id` 直接填进 `image` / `new_image`。

## 使用场景

| 操作需求 | CLI | 说明 |
| --- | --- | --- |
| 在指定位置新增一张浮动图片 | `sheet float image set` | 返回 `image_id`，用于后续替换或删除 |
| 替换已有浮动图片的图片内容 | `sheet float image update` | 按 `image_id` 替换为 `new_image` |
| 删除一张浮动图片 | `sheet float image delete` | 按 `image_id` 删除 |
| 一次插入多张浮动图片 | `sheet float image batchset` | 每张图独立指定 `image`、`anchor`、尺寸 |
| 查询子表浮动图片 | `sheet float image list` | 获取 `image_id`、锚点和尺寸 |

## 定位前置检查

1. 先确认目标 `docid` 和 `sheet`；用户只给工作簿名时，先通过文档搜索或工作簿信息定位。
2. 用户描述“放到某列旁边”“放到数据末尾”时，先读取表头或当前数据范围，推导单个 A1 锚点。
3. 更新或删除前先调用 `float image list` 获取 `image_id`，不要按锚点或返回顺序盲改。
4. 如果用户要求图片作为记录字段参与排序、筛选或数据处理，优先使用 `sheet-write-cells.md` 的单元格图片能力，不用浮动图片。

## 创建浮动图片 — `wecom-cli sheet float image set`

### 命令

```bash
wecom-cli sheet float image set --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `docid` | string | 是 | — | 在线表格 ID / URL |
| `sheet` | string | 是 | — | 子表 ID 或名称 |
| `image` | string | 是 | — | 图片 URL；若源图片是本地路径或 `media_id`，先通过 `images upload` 换取 URL |
| `anchor` | string | 是 | — | 图片锚点单元格，纯 A1，如 `F50` |
| `width` | int | 否 | — | 图片显示宽度，单位 px |
| `height` | int | 否 | — | 图片显示高度，单位 px |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `anchor` | string | 实际锚点单元格，纯 A1 |
| `image_id` | string | 新建浮动图片 ID；后续更新 / 删除使用 |

### 示例

```bash
wecom-cli sheet float image set --json '{"docid":"e3_AHsALgbHACcCN1HvTa5u7SFefia8Y","sheet":"数据","image":"https://wdoc-76491.picgzc.qpic.cn/x3WIu_1764596460","anchor":"F50","width":320,"height":240}'
```

### 使用规则

- **锚点是单格**：`anchor` 传 `F50` 这类单个单元格，不要传 `F50:H60`。
- **保存 `image_id`**：创建成功后记录返回 ID；替换和删除都依赖它。
- **尽量保持原比例**：如果上传返回的图片 URL 带有 `w` / `h` 参数，或已知原图像素尺寸，指定 `width` / `height` 时优先按原图宽高比换算，避免拉伸变形。用户明确指定宽高时以用户要求为准。

## 更新浮动图片 — `wecom-cli sheet float image update`

### 命令

```bash
wecom-cli sheet float image update --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `docid` | string | 是 | — | 在线表格 ID / URL |
| `sheet` | string | 是 | — | 子表 ID 或名称 |
| `image_id` | string | 是 | — | 要替换的浮动图片 ID |
| `new_image` | string | 是 | — | 新图片 URL；若源图片是本地路径或 `media_id`，先通过 `images upload` 换取 URL |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `image_id` | string | 被更新的浮动图片 ID |

### 示例

```bash
wecom-cli sheet float image update --json '{"docid":"e3_AHsALgbHACcCN1HvTa5u7SFefia8Y","sheet":"数据","image_id":"1106075089","new_image":"https://wdoc-76491.picgzc.qpic.cn/6ddbdae8f"}'
```

### 使用规则

- **只替换图片内容**：当前开放字段只包含 `new_image`，不要在 update 中同时尝试移动锚点或改尺寸。
- **先 list 再改**：用户只说“替换 F50 那张图”时，先用 `float image list` 找到对应 `image_id`。

## 删除浮动图片 — `wecom-cli sheet float image delete`

### 命令

```bash
wecom-cli sheet float image delete --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `docid` | string | 是 | — | 在线表格 ID / URL |
| `sheet` | string | 是 | — | 子表 ID 或名称 |
| `image_id` | string | 是 | — | 要删除的浮动图片 ID |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `image_id` | string | 已删除的浮动图片 ID |

### 使用规则

- **按 ID 删除**：删除前先 `float image list` 核对图片锚点和 `image_id`。

## 批量插入浮动图片 — `wecom-cli sheet float image batchset`

### 命令

```bash
wecom-cli sheet float image batchset --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `docid` | string | 是 | — | 在线表格 ID / URL |
| `sheet` | string | 是 | — | 子表 ID 或名称 |
| `images` | array | 是 | — | 要插入的图片数组，元素结构见下方 `images[]` |

`images[]` 对象结构：

| 子字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `image` | string | 是 | — | 图片 URL；若源图片是本地路径或 `media_id`，先通过 `images upload` 换取 URL |
| `anchor` | string | 是 | — | 图片锚点单元格，纯 A1，如 `H2` |
| `width` | int | 否 | — | 图片显示宽度，单位 px |
| `height` | int | 否 | — | 图片显示高度，单位 px |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `results` | array | 批量插入结果数组 |
| `results[].anchor` | string | 实际锚点单元格，纯 A1 |
| `results[].image_id` | string | 新建浮动图片 ID |

### 示例

```bash
wecom-cli sheet float image batchset --json '{"docid":"e3_AHsALgbHACcCN1HvTa5u7SFefia8Y","sheet":"数据","images":[{"image":"https://wdoc-76491.picgzc.qpic.cn/Iu_1764596460","anchor":"H2","width":200,"height":150},{"image":"https://wdoc-76491.picgzc.qpic.cn/b6ddbdae8f","anchor":"H10","width":200,"height":150}]}'
```

### 使用规则

- **逐项指定锚点**：每张图片都必须有自己的 `anchor`；不要用一个起始位置让后端自动排布。
- **尽量保持原比例**：如果上传返回的图片 URL 带有 `w` / `h` 参数，或已知原图像素尺寸，逐张按原图宽高比设置显示尺寸；用户明确指定宽高时以用户要求为准。
- **批量后保存映射**：保留 `anchor` 到 `image_id` 的映射，便于后续替换或删除。
- **失败项单独处理**：若回包出现部分失败，按失败项单独重试或向用户说明。

## 查询浮动图片 — `wecom-cli sheet float image list`

### 命令

```bash
wecom-cli sheet float image list --json '<JSON 参数>'
```

### 参数

| 字段 | 类型 | 必填 | 默认值 | 语义 |
|---|---|---|---|---|
| `docid` | string | 是 | — | 在线表格 ID / URL |
| `sheet` | string | 是 | — | 子表 ID 或名称 |

### 返回

| 字段 | 类型 | 说明 |
|---|---|---|
| `images` | array | 子表中的浮动图片列表 |
| `images[].image_id` | string | 浮动图片 ID |
| `images[].url`      | string | 浮动图片 URL     |
| `images[].anchor` | string | 图片锚点单元格，纯 A1 |
| `images[].width` | int | 图片宽度，单位 px |
| `images[].height` | int | 图片高度，单位 px |

### 使用规则

- **用于定位对象**：更新和删除都先 list，确认 `image_id`、锚点和尺寸。
- **锚点可能不唯一**：同一锚点可能有多张图片时，结合尺寸、创建上下文或用户描述消歧。
- **宽高单位统一**：`set` / `batchset` / `list` 的 `width`、`height` 均按 px 理解。

## 能力边界

- 当前 reference 只开放用户表格中列出的字段；不支持移动已有浮动图片、调整已有浮动图片尺寸、分页查询或按区域过滤。
- 浮动图片通过锚点单元格定位，适合做子表上的悬浮展示对象。需要图片作为单元格内容写入，或需要明确绑定某条记录并按单元格数据处理时，使用 `sheet-write-cells.md` 的单元格图片能力。
