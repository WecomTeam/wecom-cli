# 基于材料从零创建PPT

> 🔴 **先分清材料与模板**：用户发来的 `.pptx` 若是要「照着它的样子做」（模板 / 风格 / 样式 / 套用），它是**用户模板**，不是材料——改读 [create-from-template.md](create-from-template.md)，不要对它取正文（读出来的是占位句和示例数据，会被当成事实写进大纲）。分不清时按该文件 T0 判定。

## Step0 · 先确认材料在哪

用户刚发进对话的文件，到手是 **`media_id`**（前缀 `mc`），**不是本地路径**。下方所有本地脚本只吃本地路径，把 `media_id` 直接填进去会以「文件不存在」失败。

按**目的**决定要不要先落地成本地文件：

| 目的 | 需要本地文件 | 怎么做 |
|---|:-:|---|
| **读材料正文**（提炼内容、写大纲） | ✅ | 先 `wecom-cli media download` 拿 `file_path`，再直接读取该本地文件 |
| **转页面图学风格** / **抽材料内嵌图**（要跑本地脚本） | ✅ | **先 `wecom-cli media download` 拿 `file_path`**，再把该路径喂给脚本 |
| **把这个 pptx 导入为在线文档** | ❌ | 导入接口本身支持 `media_id`，**直接传，不要下载** |

### wecom-cli media download — 把 media_id 落地成本地文件

```bash
wecom-cli media download --json '{"media_id": "<MEDIA_ID>"}'
```

| 参数 | 类型 | 必填 | 说明 |
|---|---|:-:|---|
| `media_id` | string | 是 | 媒体文件的唯一标识；只能来自真实返回值，**不可构造或猜测** |

出参 `file_path`（下载文件的本地路径），作为下方脚本的入参。

🔴 **下载产物是链路内部临时副本**：仅供本地脚本读取，用完即弃。**严禁**把该路径、或据此产出的图片 / 中间文件，以附件、本地路径、下载链接等任何形式交付给用户。

> `file_path` 只在**当前执行环境**内有效，不代表用户电脑上的路径。若当前环境无法下载或无法执行本地脚本，本节链路不可用 —— 退回下方文本兜底，不要把宿主机路径冒充可用路径。

## 快速命令（可多选）
| 任务类型 | 命令 |
|---|---|
| 提取上传材料的内容 | `wecom-cli media download` 落地后**直接读取本地文件**（pdf / docx / pptx / xlsx / 图片均可直接读） |
| 从材料中提取图片（仅支持docx、pptx、xlsx格式） | `python3 "{baseDir}/scripts/doc_image_extractor.py" "<待提取的文件路径，支持多个文件>" -o "<输出的目录>"` |
| 将上传的pptx文件转换为图片 | `python3 "{baseDir}/scripts/pptx2image.py" "<pptx文件的路径>" --outdir style_preview --grid 9 --dpi 300` |

- `{baseDir}` 表示当前 Skill 所在目录（`wecomcli-pptx`）。执行时必须替换为实际绝对路径。
- 脚本已经存在，直接运行；禁止先做脚本存在性检测。
- 脚本路径整段加双引号：技能安装路径由运行环境决定，可能含空格或非 ASCII 字符，漏引号会在空格处被拆成两个参数而找不到脚本。

> 表中的「路径」一律指 Step0 中 `media download` 返回的 `file_path`，或用户直接给出的本地路径。
>
> **读正文一律先下载再本地直读**：`media_id` 先 `wecom-cli media download` 拿 `file_path`，再直接读取该文件；本地已有路径的直接读。

**注意：PDF 文件同样先 `media download` 落地，再直接读取。**

> **读 pptx 风格的两级，依次来，不是二选一**：
>
> 1. **转图**：`pptx2image.py`（命令见上表）。唯一的**系统二进制**依赖是 LibreOffice（`soffice`，pptx→pdf）；PDF→PNG 用 `pypdfium2`、拼图用 Pillow（均为预装 pip 包）。拿到的是页面真实长相——配色、字体层级、版式节奏、留白密度，看图最准。
> 2. **读结构**：报「soffice not found」/「DeploymentException」或 `pypdfium2` 缺失时，**不要装这些依赖、也不要跑 `pip install`**，改跑 `python3 "{baseDir}/scripts/build_page.py" template "<file_path>" --dir style_params`（纯标准库，解析 `ppt/theme/theme1.xml` 与各页 xml），然后只读 `style_params/template_style.json`：模板实际用色与频次、主色辅色、字体与衬线类、逐页明暗、素材角色。直接作为 `DESIGN.md` 的填色与字体依据。（它同时会在 `style_params/` 下生成外壳与素材，**材料路径用不到，不要去读、也不要搬到工作目录**；那些是用户模板路径的产物。）
>
> 两级都拿到时以转图为准、结构参数作补充（转图给不了原始底图文件，结构参数给不了"看起来像什么"）。**用户模板走 [create-from-template.md](create-from-template.md)**，那边只取底图 / 配色 / 字体，不读版心结构。

## 严格遵循约束
- 禁止反复下载或读取材料的内容（同一份材料只下载一次、只读一次，结果自己留存复用）；
- 可以通过读取pptx转换的图片（或兜底时解析其 xml）去学习pptx文件的风格（包括基本的颜色系统、字体层级、母版与版式、视觉元素，禁止一成不变，模仿无法复现的风格样式）；
- **必须审核从材料中获取的图片质量**：图片落地本地后**逐张直接读取查看**，确认清晰度、完整性与内容；**每次只读 1 张**，避免多图混入同一上下文导致张冠李戴。
  
  若材料中图片清晰、完整且满足要求，则在PPT中引用材料中的图片（更符合用户需求）；若图片模糊、不完整或不适用，该图位改用 SVG / 色块 / 版式留白承载（见 [component-image.md](component-image.md) §P1）——**当前环境没有生图工具，不提供 AI 配图**；
- 在高置信度场景（如财务数据分析、学术科研汇报、法律政策解读等）中，不仅PPT的**文本内容**要严格来源于用户上传的材料，**更要优先提取并直接使用材料中原有的图片、图表和数据**。必须保持信息的绝对准确性和一致性；
- ⚠️ **材料图须先上传拿 `url`**（`media_id` 优先直传 `smartpage images upload`，无需落地；本地文件传 `file_path`）；拿到的 `url` 直接写进 JSX 的 `src` 进页面。详见 [component-image.md](component-image.md) §P0。

## 创建流程
材料`内容`和`图片`获取完成之后，请读取并遵循 [create-from-scratch.md](create-from-scratch.md) 中的标准流程来创建PPT。