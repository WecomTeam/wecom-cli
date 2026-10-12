# PPT 接口层

本文件是 `wecomcli-pptx` **`wecom-cli slide` / `wecom-cli doc create` / `wecom-cli smartpage images upload` 命令的唯一来源**，其他文档一律以「见 `pptx-api.md` §xxx」引用，不写命令原文。

（不含：本地脚本调用如 `scripts/*.py`、跨 skill 命令如 `wecom-cli media download` / `wecom-cli doc import` —— 这些写在各自相关文档中。）

## 三条硬约束

1. **调用任何 `wecom-cli slide` 命令前，先用 `--schema` 探测"有没有、怎么传"**：

   ```bash
   wecom-cli slide --schema                      # 当前下发的命令全集（动态下发，随版本变化）
   wecom-cli slide pages addjsx --schema         # 单条命令的参数 schema
   ```

   本文件只固化 §生成类接口 与 §编辑类接口 列出的命令，**其余编辑接口不在 skill 中固化**——先 `wecom-cli slide --schema` 探测目标命令是否已下发，再按其 schema 传参；探测不到即当前版本未开放，换已固化的命令组合实现。**「固化」的只是命令名与用途，不是入参**：上表及 [pptx-edit.md](pptx-edit.md) 列出的命令（含只读的 `slide get` / `slide pages get`）调用前同样先 `--schema` 现查参数再传。**填值也照 schema**：类型 / 必填 / 枚举 / 范围以返回为准，schema 没有的取值不要自己编、没有的字段不要加。**嵌套结构也照 schema**：字段是对象就按它的 `properties` 嵌套传（如 `position: {x, y, w, h}`），不要拍平成平铺字段。

   > **例外**：`wecom-cli doc create` 不查 schema，照 §生成类接口 直接传——通用文档入口的 schema 是六类并集（约 1 万 token），PPT 只用其中 `doc_type` + `doc_name` 2 个字段。

2. **`wecom-cli slide designs update` 是所有写命令的前置门**：设计契约未持久化之前，一切插页、插图、改文字操作会被服务端**直接拒绝**。契约有效期 **24 小时**，跨会话编辑前须先 `wecom-cli slide designs get` 确认仍有效，失效则用本地副本重新写入。

3. **改 / 删任何资源前，先 get 确认它存在**：`slide get` 拿页数与 `slide_ids`，`slide pages get` 拿某页的形状列表，`slide shapes get` 拿单个形状。凭印象对页码 / `shape_id` 动手，命中不存在的资源即报错。

## 命令形态

```
wecom-cli slide <资源> <动作> [--json '{...}'] [--schema]
```

一条命令即一个能力。**无需区分读写通道**，也无需先查工具清单再拼 `tool` 名。命令全集**动态下发**：skill 只固化 §生成类接口 与 [pptx-edit.md](pptx-edit.md) 列出的命令，其余以 `wecom-cli slide --schema` 实时探测为准。

> 🔴 **payload 含多行文本 / 嵌套引号 / 中文长段**（`designs update` 的整份 `DESIGN.md`、`addjsx` 的 `jsx_code`）时，不要一行 `--json`——shell 转义会报「命令语法不完整」。改用 python：`subprocess.run(["wecom-cli","slide",...,  "--json", json.dumps(payload, ensure_ascii=False)])`，传 **argv 数组、不 `shell=True`**。短 JSON 仍可一行直传。

> 🔴 **长文本 payload 一律从本地文件读，不把原文写进命令**。`jsx_code` / `DESIGN.md` 在写的时候已经落过盘（页面源码的文件命名见 [component-slide.md](component-slide.md) §文件组织，`DESIGN.md` 见 `create-from-scratch.md` Step4），提交时在 python 里读回来即可：
>
> ```python
> import json, subprocess
> from pathlib import Path
>
> payload = {
>     "docid": DOCID,
>     "jsx_code": Path("03_market_analysis.jsx").read_text(encoding="utf-8"),  # 源码留在盘上
>     "page_index": 2,
> }
> subprocess.run(["wecom-cli", "slide", "pages", "addjsx", "--json",
>                 json.dumps(payload, ensure_ascii=False)], check=True)
> ```
>
> 把源码原文再贴进命令 = 同一份内容在上下文里出现两遍（落盘一次、提交一次）。一份十几页的 PPT 因此多烧 20~40K token，是本 skill 触发上下文超限的主要来源。`designs update` 传 `DESIGN.md` 同理。

### 文档标识

一律使用 `docid`（与企微文档链接、`wecomcli-doc-manage` / `wecomcli-disk` 搜索结果中的 `docid` 同名同义）。需要用链接定位时用 `url`。

## 生成类接口

| 步骤 | 命令 | 说明 |
|---|---|---|
| 创建空文档 | `wecom-cli doc create` | 见下方，**不查 schema** |
| **持久化设计契约** | `wecom-cli slide designs update` | 🚨 **写类前置门**，跳过则后续全部失败；🚨 **`design_md` 与 `story_md` 必须同一次调用一起提交**，只传一个会失败 |
| 读回设计契约 | `wecom-cli slide designs get` | 跨会话编辑前确认是否仍有效 |
| 插入一页 | `wecom-cli slide pages addjsx` | 写 JSX 源码生成内容页。**单页命令**，N 页 = N 次调用；**异步任务**，出参 `task_id` + `slide_id` |
| 核对结构 | `wecom-cli slide get` | 页数 / 页序 / 画布尺寸 |

> `wecom-cli slide pages add` 是「按版式建空页」，**不是**插入内容页；写 JSX 一律用 `addjsx`。

### 创建空文档

```bash
wecom-cli doc create --json '{"doc_type": "ppt", "doc_name": "季度汇报"}'
```

`doc_type` 固定 `"ppt"`，不传会建成非 PPT；`doc_name` ≤ 255 字符、不能含 `/ \ : * ? " < >` 与竖线，默认中文。新建文档自带一页空白首页，**建完立即删**（`slide pages delete`，索引 0）。

### addjsx 的图片机制：上传后的 URL 直接写进 `src`

JSX 里的 `<Image src="...">` **直接写上传口返回的 `url`**，服务端编译时下载渲染。先把图传过上传口拿到 `url`，再把它填进 jsx 的 `src`：

```jsx
// 落盘的页面源码文件（如 03_market_analysis.jsx）里长这样，src 已是上传口的 url
<Slide>...<Image src="https://...（上传口拿到的 URL）" style={{...}}/>...</Slide>
```

提交时按 §命令形态 的红线，用 python 读该文件组装 `{docid, jsx_code, page_index}`，**不要把源码抄进命令行**。

- 🔴 **`page_index` 必填，且必须是明确递增索引**（**第 N 页大纲 → `page_index=N-1`**，即 0、1、2…）。**禁止省略、禁止传 `-1` 追加**——即使 schema 标它可选也必须传。`doc create` 新建的文档自带一页空白首页，**建文档后立即删除**（`slide pages delete`，索引 0），之后生成页从 `page_index=0` 起递增插入。显式索引让每次插入的位置可预期、可复核，失败重插同页时直接用同一索引。
- 🔴 **`src` 只接受上传口返回的 `url`**，**不接受**本地路径、base64、任何公网图片直链。
- 🔴 **不要用 `image_replacements` / `base64_replacements`**——schema 里即使有这两个入参，本 skill **不使用**：`url` 直接写进 `src` 即可，base64 会撑爆上下文。看到 schema 里有它们也不要传。
- 出参 `slide_id`、`task_id`。CLI 返回时任务已完成：成功 = `errcode == 0`；失败时退出码非 0，`error.message` 给出原因（如 `[JSX_PARSE_ERROR]`），按提示修 jsx 重提本页。**不要用 `task_id` 轮询、不要 `sleep`**。
- 🔴 **严格串行，一页一确认**：**本页确认成功之前，绝不提交下一页**。本页失败就停在本页修完重提，不要跳过去继续往后插——异步任务并发提交会打乱页序，且失败页会在中间留下空洞。

## 编辑类接口

skill 固化的编辑命令（5 条）与「其余接口动态下发、用 `--schema` 探测」的完整说明，统一见 [pptx-edit.md](pptx-edit.md)。

## 配图接口

🔴 **唯一出口**：凡最终用于 PPT 的图片，**无论来源，都必须经 `wecom-cli smartpage images upload` 上传绑定到目标文档**，拿返回的 `url` **直接写进 JSX 的 `src`** 进页面（见 §生成类接口）。会话图片的 `media_id`、本地抽取图——**都不能直接进 PPT**，一律先过这个上传口。

```bash
wecom-cli smartpage images upload --json '{"docid": "<DOCID>", "media_id": "<MEDIA_ID>"}'
# 或本地文件：
wecom-cli smartpage images upload --json '{"docid": "<DOCID>", "file_path": "/abs/a.png"}'
```

入参 **`docid`（目标文档，必填）+ `file_path` / `media_id` 二选一**，出参 `url`。**直接调 CLI，无需任何脚本包装**。

> ⚠️ **`smartpage images upload`（上传拿 `url`）≠ `slide images add`（往已有页面插图）**——前者是配图准备、生成与编辑都用；后者仅编辑场景给已存在页面加图。两者同资源段、尾词不同，照抄即可，不要混用。

> ⚠️ **上传需要 `docid`，故顺序固定**：先 `doc create` 建文档（Step5），再做配图准备（Step6），最后 `addjsx` 插页（Step7）。

两个来源到达出口的路径：

| 来源 | 路径 |
|---|---|
| **用户材料图**（会话发图） | 到手的 `media_id` → 直接上传（**首选，无需落地**） |
| **材料内嵌图**（pptx / docx / xlsx 里的图） | `media download` 下载文档 → `doc_image_extractor.py` 抽出本地图 → 以 `file_path` 上传 |

### 无生图工具（不支持 AI 配图）

**当前环境没有生图工具，本 skill 不提供 AI 配图能力。** 不要尝试调用任何生图接口、不要反复重试、不要换其他方式生图。

图位按此处理：能用材料图的用材料图（优先级本就最高），其余图位改用 `<SVG>` / 色块 / 版式留白承载。材料图与 SVG 两条路径不依赖生图工具，可独立完成。**对外不提配图**——没有生图能力是既定形态不是故障，不说「无法生成配图」、不把它说成服务问题、不建议用户手动加图（口径见 `templates/README.md` §5）。

### 材料图的安全约束

材料图来自用户上传的财报 / 合同 / 内部截图。需要**原样呈现**的材料必须**原图直传**上传口、零重绘。

> 🔑 **材料内嵌图**的 `media download` 产物属链路内部临时副本，用完即弃，**不得以附件 / 路径 / 下载链接形式交付给用户**。详见 [create-from-material.md](create-from-material.md) §Step0。

**编辑场景**：给**已存在的页面**加图时无法重写整页 JSX——插图命令（如 `slide images add`）不在 skill 固化清单内，先 `wecom-cli slide images add --schema` 探测是否已下发，已下发则按其 schema 传参（URL 同样来自上传口）；未下发则退回整页重做。

**尺寸适配**：JSX 图位的 `width` / `height` 由 `DESIGN.md` 布局决定，原图与图位宽高比不一致时用 `objectFit: 'cover'` 裁切适配（见 `component-image.md`）；以配图前置核对阶段的目检为准，避免裁掉关键主体。

## 禁止导出

本环境**没有任何导出能力**：任何把在线 PPT 导出 / 下载为本地文件的诉求（无论是否本次生成），一律不调用 `doc export` 或任何导出命令、不伪造下载路径，直接告知没有导出能力。交付物只有在线文档链接。详见 `SKILL.md`「禁止导出」。

## 结果判定与错误处置

统一看 `is_success` 字段。**不要**自己去判断「响应里有没有 `result`、`error` 是否为空串」——CLI 已把上游的多种响应形态收敛为这一个判断点。

失败时结构化输出 `{code, type, message, trace_id, raw}`：

| code | 含义 | 处置 |
|---|---|---|
| `400006` | 票据失效 | 告知用户重新授权，不重试 |
| `400016` | 文档类型不匹配 | 品类用错——确认目标确实是 slide 文档 |
| `6044135` | 获取权限数据失败 | **可重试**（稍后重试一次） |
| `640207` | 下游偶发错误 | **退避 3 秒后重试一次** |
| `850005` | 限流 | **退避 30 秒后重试** |
| `-32601` | 命令不存在 | 命令名拼错，或该接口当前版本未动态下发；`wecom-cli slide --schema` 复核命令全集，确未下发则按 [pptx-edit.md](pptx-edit.md) 换固化命令组合（如整页重做） |
| 其他 | — | 原样透出 `message`，不自行改写语义 |

## 待确认（实测后回填本文件）

| 项 | 内容 |
|---|---|
| 上传 `url` 时效 | `smartpage images upload` 的 `url` 绑定文档后是否长期有效（决定长期打开是否裂图、跨会话编辑是否需重新上传） |
| `designs update` | 是否校验设计契约的内容格式或来源 |
| `images add` | 编辑场景插图：入参是否直接收 URL，是否返回新建形状的标识 |
