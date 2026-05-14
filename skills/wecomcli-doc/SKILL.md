---
name: wecomcli-doc
description: 企业微信文档、智能表格和智能文档（原名智能主页）管理技能。提供文档的创建、读取、编辑能力，智能表格的创建，以及智能文档的创建和内容导出。适用场景：(1) 以 Markdown 格式获取文档完整内容 (2) 新建文档或智能表格 (3) 用 Markdown 格式覆写文档内容 (4) 创建智能文档，将本地 Markdown 文件发布为智能文档 (5) 导出智能文档内容为 Markdown。支持通过 docid 或文档 URL 定位文档。用户提及 `https://doc.weixin.qq.com/xxxx` 格式的URL链接时，触发该技能。
metadata:
  requires:
    bins: ["wecom-cli"]
  cliHelp: "wecom-cli doc --help"
---

# 企业微信文档管理

> `wecom-cli` 是企业微信提供的命令行程序，所有操作通过执行 `wecom-cli` 命令完成。

管理企业微信文档和智能文档（原名智能主页）的创建、读取和编辑。文档接口支持通过 `docid` 或 `url` 二选一定位文档。

> ⚠️ **重要触发规则**：只有当用户明确提到「**智能文档**」或「**智能主页**」时，才使用智能文档相关接口（`smartpage_*` 系列）。其他所有涉及「文档」的场景（如"创建文档"、"写个文档"、"帮我建个文档"等），一律使用企微文档接口（`create_doc` / `get_doc_content` / `edit_doc_content`）。

## URL 品类识别与接口路由

企业微信文档有三种品类，**URL 格式不同，读取内容所用的接口也不同**，切勿混用：

| URL 模式 | 品类 | 读取内容接口 |
|---|---|---|
| `https://doc.weixin.qq.com/doc/*` | **文档**（doc_type=3） | `get_doc_content` |
| `https://doc.weixin.qq.com/smartsheet/*` | **智能表格**（doc_type=10） | `get_doc_content` |
| `https://doc.weixin.qq.com/smartpage/*` | **智能文档**（原名智能主页） | `smartpage_export_task` → `smartpage_get_export_result` |

**判断规则**：
- URL 路径以 `/doc/*` 开头 → 文档 → 用 `get_doc_content`
- URL 路径以 `/smartsheet/*` 开头 → 智能表格 → 用 `get_doc_content`
- URL 路径以 `/smartpage/*` 开头 → 智能文档（原名智能主页） → 用 `smartpage_export_task`

## 调用方式

通过 `wecom-cli` 调用，品类为 `doc`：

```bash
wecom-cli doc <tool_name> '<json_params>'
```

## 返回格式说明

所有接口返回 JSON 对象，包含以下公共字段：

| 字段 | 类型 | 说明 |
|------|------|------|
| `errcode` | integer | 返回码，`0` 表示成功，非 `0` 表示失败 |
| `errmsg` | string | 错误信息，成功时为 `"ok"` |

当 `errcode` 不为 `0` 时，说明接口调用失败，可重试 1 次；若仍失败，将 `errcode` 和 `errmsg` 展示给用户。

### 特殊错误码

| errcode | errmsg | 含义 | 处理方式 |
|---------|--------|------|----------|
| `851002` | `incompatible doc type` | 文档品类与所调用的接口不匹配 | 根据文档 URL 重新确认品类（参见上方「URL 品类识别与接口路由」表），然后使用该品类对应的正确接口重试 |

---

## 文档

### get_doc_content

获取文档完整内容数据，只能以 Markdown 格式返回。采用**异步轮询机制**：首次调用无需传 `task_id`，接口返回 `task_id`；若 `task_done` 为 false，需携带该 `task_id` 再次调用，直到 `task_done` 为 true 时返回完整内容。

建议写脚本自动轮询下载文档，不要手工复制轮询结果。下载后如果正文里包含 `data:image/...;base64,...` 图片，直接运行
`node scripts/extract-md-images.mjs <markdown-file>`
把图片导出到 `image/` 并同步改写 markdown 引用。

- 首次调用（不传 task_id）：
```bash
wecom-cli doc get_doc_content '{"docid": "DOCID", "type": 2}'
```
- 轮询（携带上次返回的 task_id）：
```bash
wecom-cli doc get_doc_content '{"docid": "DOCID", "type": 2, "task_id": "xxx"}'
```
- 或通过 URL：
```bash
wecom-cli doc get_doc_content '{"url": "https://doc.weixin.qq.com/doc/xxx", "type": 2}'
```

参见 [API 详情](references/get-doc-content.md)。

### create_doc

新建文档（doc_type=3）或智能表格（doc_type=10）。创建成功返回 url 和 docid。

- 创建文档：
```bash
wecom-cli doc create_doc '{"doc_type": 3, "doc_name": "项目周报"}'
```
- 创建智能表格：
```bash
wecom-cli doc create_doc '{"doc_type": 10, "doc_name": "项目任务表"}'
```

**注意**：
- docid 仅在创建时返回，需妥善保存
- 智能表格（doc_type=10）的详细管理功能（子表、字段、数据记录等）已迁移到 `wecomcli-smartsheet` skill，请使用该 skill 进行高级操作

参见 [API 详情](references/create-doc.md)。

### edit_doc_content

用 Markdown 内容覆写文档正文。`content_type` 固定为 `1`（Markdown）。

```bash
wecom-cli doc edit_doc_content '{"docid": "DOCID", "content": "# 标题\n\n正文内容", "content_type": 1}'
```

参见 [API 详情](references/edit-doc-content.md)。

---

## 智能文档（原名智能主页）

适用品类：智能文档（用户说「智能文档」或「智能主页」时触发）
适用 URL：`/smartpage/*`

> ⚠️ 只有当用户明确指定「智能文档」或「智能主页」时，才使用以下接口。其他「文档」场景请使用上方的企微文档接口。

适用场景：
1. 将本地 Markdown 文件创建为智能文档
2. 异步导出智能文档内容为 Markdown

### smartpage_create

创建智能文档（原名智能主页），支持传入标题和多个子页面。每个子页面可指定标题、内容类型和本地文件路径。创建成功返回 docid 和 url。

> ⚠️ **特殊语法**：此命令必须使用 `+smartpage_create`（带 `+` 前缀），加号不可省略；该 `+` 仅适用于此命令，不要泛化到其他 `doc` 子命令。

```bash
wecom-cli doc +smartpage_create '{"title": "项目概览", "pages": [{"page_title": "需求文档", "content_type": 1, "page_filepath": "/path/to/requirements.md"}]}'
```

**注意**：
- `content_type` **必须与文件实际内容匹配**：`.md` 文件或包含 Markdown 语法的内容必须传 `1`（Markdown），仅纯文本才传 `0`。绝大多数场景应传 `1`
- docid 仅在创建时返回，需妥善保存
- 每个子页面的 Markdown 文件大小不得超过 **10MB**，超过会导致创建失败。如果文件过大，需先拆分为多个子页面再创建

参见 [API 详情](references/smartpage-create.md)。

### smartpage_export_task

发起智能文档内容导出任务（异步）。传入 docid（或 url）和 content_type，返回 task_id。这是异步导出的第一步，需配合 `smartpage_get_export_result` 轮询获取导出结果。

- 通过 docid：
```bash
wecom-cli doc smartpage_export_task '{"docid": "DOCID", "content_type": 1}'
```
- 或通过 URL：
```bash
wecom-cli doc smartpage_export_task '{"url": "https://doc.weixin.qq.com/smartpage/xxx", "content_type": 1}'
```

参见 [API 详情](references/smartpage-export.md)。

### smartpage_get_export_result

查询智能文档导出任务进度。传入 task_id 进行轮询，当 `task_done` 为 `true` 时返回 `content`（导出的完整文档内容）。

```bash
wecom-cli doc smartpage_get_export_result '{"task_id": "TASK_ID"}'
```

当 `task_done` 为 `true` 时，`content` 字段即为导出的 Markdown 内容。

参见 [API 详情](references/smartpage-export.md)。

## 典型工作流

> **关键提示**：读取内容前先看 URL 判断品类。`/doc/` 或 `/smartsheet/` → `get_doc_content`；`/smartpage/` → `smartpage_export_task`。只有用户明确提到「智能文档」或「智能主页」时才走 smartpage 流程，其他文档场景一律使用企微文档接口。

### 文档操作

1. **读取文档** → 
```bash
wecom-cli doc get_doc_content '{"docid": "DOCID", "type": 2}'
```
，若 `task_done` 为 false 则携带 `task_id` 继续轮询
2. **清理图片** → 下载后的 markdown 若带 base64 图片，运行
```bash
node .scripts/extract-md-images.mjs xxx.md
```
导出图片到 `image/`，并把引用改成相对路径
3. **创建新文档** → 
```bash
wecom-cli doc create_doc '{"doc_type": 3, "doc_name": "文档名"}'
```
，保存返回的 docid
4. **编辑文档** → 先 get_doc_content 了解当前内容，再 edit_doc_content 覆写

### 智能文档操作

1. **创建智能文档**（仅当用户明确要求「智能文档」或「智能主页」时） → 
```bash
wecom-cli doc +smartpage_create '{"title": "标题", "pages": [{"page_title": "子页面", "content_type": 1, "page_filepath": "/path/to/file.md"}]}'
```
，保存返回的 docid
2. **获取智能文档内容**（URL 含 `/smartpage/`，异步两步）：
   - **第一步**：发起导出任务 →
```bash
wecom-cli doc smartpage_export_task '{"docid": "DOCID", "content_type": 1}'
```
，获取 `task_id`
   - **第二步**：轮询导出结果 →
```bash
wecom-cli doc smartpage_get_export_result '{"task_id": "TASK_ID"}'
```
，若 `task_done` 为 `false` 则继续轮询，直到 `task_done` 为 `true`，返回的 `content` 字段即为 Markdown 内容
