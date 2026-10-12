## 设计流程契约（design-principle）

> 本文件只管**流程与结构**，不再规定色板、字体、装饰、母版 px、密度门禁——这些一律由所选范例套的 `DESIGN.preset.md`（路径取 `assets/templates/index.json` 的 `preset` 字段）给出。任何「预设 vs 本文件」的冲突，预设赢。用户上传了自己的 PPT 模板时，`template` 生成的 `DESIGN.template.md` 取代预设的位置（create-from-template.md）。
>
> 页型、信息下限、内容硬闸门不在本文件，在 [content-contract.md](content-contract.md)（通用）与 [government-contract.md](government-contract.md)（政务增量）。

### 1. 路由链

1. 用户显式指令（具体 hex、字体名、页数、比例、密度偏好）最高优先，只覆盖它点名的常量。
2. 内容结构一律读 [content-contract.md](content-contract.md)；Step4 的政务闸门命中时再叠加 [government-contract.md](government-contract.md)，同一维度取严者。
3. 版式从 `assets/templates/index.json` 的**扁平 9 套**里按 Step4 的打分选（政务闸门命中时只在政务两套里取）。
4. 用户选定一套后，该套 `DESIGN.preset.md` 即本项目 DESIGN.md 的主体（Step5 持久化的就是它），再补第 2 节的页面映射表。

**兜底**：没有贴合的套时，取场景最接近的一套，只按用户显式指令换色板；**禁止脱离范例从零自由设计**。

**唯一例外——用户上传了自己的 PPT 模板**：这条路径不走范例池，版式由 `DESIGN.template.md` 的底图 / 配色 / 字号档 / 版心尺寸自由设计（create-from-template.md T5）。此时本文件第 3 节的两条审美规则、以及 `DESIGN.template.md` §5 的锚点与填充率规则一起生效，替代范例骨架的兜底作用。

### 2. 页面映射表（DESIGN.md 必填）

为每一页给出：序号、源码文件、类型、角色（hero / supporting / transition）、版式、主视觉来源、字数估算、留白比例、色彩分配、关键约束。它是 STORY.md 与逐页源码之间的契约，逐页生成时严格对照。

```
| # | 源码文件 | 类型 | 角色 | 版式 | L1 来源 | 字数估算 | 留白% | 色彩分配 | 关键约束 |
| 01 | 01_cover.jsx | cover | hero | 范例 01_cover 骨架 | — | 30 | 35% | 按预设 §2 | 大字标题作主视觉（预设 §6），不加图 |
| 02 | 02_agenda.jsx | catalog | supporting | 范例 02 骨架 | — | 120 | 25% | 按预设 §2 | 每项 ≥ 30 字 |
```

「L1 来源」列必须标明：`材料图` / `SVG` / `版式替代` / `—`，决定该页配图走哪条获取路径（见 Step6 与 `pptx-api.md` §配图接口）。

### 3. 视觉节奏与一致性（跨套通用，唯二保留的审美规则）

- 10 页以上的 deck 至少 2–3 页 hero 页，其余 supporting；每页视觉重量不同，连续两页不得同版式。
- 同类内容页之间标题位置、页码位置、页眉页脚 chrome、页面 padding 全篇不漂移：所有内容页用所选套 `shell/content.jsx` 经 `scripts/build_page.py build` 拼装，只改 B 区内部（见 templates/README.md §4）。封面、章节页、结束页可自定义版式，但其中的页脚、页码、装饰条坐标须与内容页一致。

### 4. 配图来源

当前环境没有生图工具，配图只用过了门槛的用户材料图，按所选套预设 §4「图位」放（正方形、不加外框）；细则见 templates/README.md §5。页面映射表的「L1 来源」列据此填 `材料图` / `SVG` / `版式替代` / `—`。

材料图一律原图直传、零重绘、逐张核对；材料里没有可用位图的图位用 `<SVG>` / 色块 / 版式留白承载，并在配图清单标注「版式替代」。完整规则见 SKILL.md §特有规则与 create-from-scratch Step6，本文件不重复。

### 5. 企微文档可用字体白名单（硬约束）

pptx 由企微文档服务端渲染，`fontFamily` **只能取下表「写法」列**；白名单外的字体名（思源、方正、Noto、Inter、JetBrains Mono、各类西文展示字体等）会被替换成默认字体并改变行宽，一律禁止写入。范例套的预设也必须落在这张表内。

| 企微文档下拉名 | fontFamily 写法 | 类别 | 适用 |
| :-- | :-- | :-- | :-- |
| 雅黑 | `Microsoft YaHei` | 中文无衬线 | 正文默认、无衬线标题（范例池现用） |
| 苹方 | `PingFang SC` | 中文无衬线 | 正文、卡片说明 |
| 冬青黑简体中文 | `Hiragino Sans GB` | 中文无衬线 | 正文备选 |
| 黑体 | `SimHei` | 中文无衬线 | 政务套标题、表格 |
| 腾讯体 W3 / W7 | `TencentSans W3` / `TencentSans W7` | 中文无衬线 | 科技方向标题、锚点数字 |
| 华文宋体 | `STSong` | 中文衬线 | 衬线 display 标题与衬线正文（范例池现用） |
| 宋体 | `SimSun` | 中文衬线 | 政务套正文、引文 |
| 华文仿宋 | `STFangsong` | 中文仿宋 | 公文正文 |
| 华文楷体 | `STKaiti` | 中文楷体 | 引文、美术大字 |
| Arial | `Arial` | 西文无衬线 | 数字、西文标签，默认西文 |
| Calibri | `Calibri` | 西文无衬线 | 西文正文备选 |
| Times New Roman | `Times New Roman` | 西文衬线 | 衬线模板里的西文与数字 |
| Courier New | `Courier New` | 西文等宽 | 页码、元信息、kicker；禁止用于 ≥ 30px 的数字 |
| Comic Sans MS | — | 禁用 | 不得出现 |
| 默认字体 | 不写 `fontFamily` | — | 禁止依赖，每个 Text 都要落到明确字体 |

**衬线跟随模板**：所选范例套的原模板 display 或 body 是衬线的，中文就用 `STSong`、西文用 `Times New Roman`；是无衬线的，中文用 `Microsoft YaHei`、西文用 `Arial`。不要全篇雅黑。**中文文本绝不能落在纯西文字体上**（Arial / Times / Courier 无中文字形，含中文的 Text 必须写中文字体）。

腾讯体、冬青黑、华文系、黑体、宋体的写法取自标准家族名，首次使用前先提交一页测试确认服务端识别，不识别时改用下拉名原文（如 `华文宋体`）。
