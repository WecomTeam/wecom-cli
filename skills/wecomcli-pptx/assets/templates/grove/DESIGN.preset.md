---
name: 文献综述
parent: academic
sub_style: 文献综述
source: beautiful-html-templates/grove (MIT, Zara Zhang)
scheme: 深底为主，浅纸面逐页交替
formality: 中高
density: 中低
best_for: 组会文献综述、读书会、方法脉络梳理、开题前的调研汇报，以及任何想要安静克制、像一本装订好的专著那样有节奏的学术场合
avoid_for: 需要密集图表和大量数字的实验汇报、需要高饱和配色或活泼氛围的宣讲、每页要塞六个以上元素的内容
---

## 1. 气质一句话
深森绿纸面上只有暖纸色的细衬线字、一根 36px 珊瑚短线和 1px 发丝线把页面框成一页纸，像一本安静的文学期刊；衬线永不加粗，强调只靠珊瑚色。

## 2. 色彩池

> **本节是默认配色，不是限制。** 模板只定版式：外壳、分区、字号层级、图位、标志性处理。颜色可以整体替换——按下表的「角色」一一对应给出新色值，用 `build_page.py init --map` 换色，再过 `contrast`。换色时保持角色数量不变（不多加一种彩色），本预设 §6 / §7 里关于「只用某种颜色」「禁止某色相」的话只约束默认配色。
| 角色 | hex | 用途 |
|---|---|---|
| 森绿 forest | `#192B1B` | 深页整版底色；浅页的标题、正文、要点字色（在纸面上 11.8:1） |
| 深绿面 forest-alt | `#1E3221` | 深页次级表面：表头行填色（承 `#9C9E8E` 字 5.0:1）；材料图位的留边色 |
| 纸面 paper | `#E8E4D6` | 浅页整版底色 |
| 纸面次 paper-alt | `#DEDAD0` | 浅页表格斑马行、次级浅面 |
| 暖纸字 cream | `#D4CFBF` | 深页标题与正文（9.6:1）；永不用 `#FFFFFF` |
| 深页次级字 cream-muted | `#9C9E8E` | 深页 chrome、页脚、导语、表头字、来源注脚（5.5:1） |
| 浅页次级字 forest-muted | `#5A6253` | 浅页 chrome、页脚、正文段落、面板标签（5.0:1） |
| 珊瑚 coral | `#C8524A` | 只作 24px 以上：标题里的强调词、关键数字、36px 短线（深页 3.4:1 / 浅页 3.5:1，小字不够用） |
| 亮珊瑚 coral-light | `#E27871` | 深页 13～18px 的珊瑚小字：kicker、破折号项目符、表格高亮行（5.1:1） |
| 深珊瑚 coral-dark | `#9E3B34` | 浅页 13～18px 的珊瑚小字：kicker、破折号、面板标签、小结标签（5.3:1） |
| 深页发丝线 hairline-dark | `#3A4434` | 深页所有 1px 线：页眉底线、页脚顶线、表格行线、分栏竖线 |
| 浅页发丝线 hairline-light | `#CBCABC` | 浅页所有 1px 线 |
| 水印 watermark | `#2A3626` | 封面 / 章节页右下角 200px 水印数字，仅此一用 |
| 全部为实色 hex，不用 rgba；珊瑚是唯一彩色，深浅两页各只换四个颜色（底、字、次级字、发丝线）。不引入宝蓝、金色、黄色等第二彩色。 |

## 3. 字体系统（1280×720 换算后）
| 角色 | fontFamily | 字号 | 字重 | 行高 | 字距 |
|---|---|---|---|---|---|
| display 封面主标 | `STSong` | 60 | 400 | 1.2 | 0，强调词换 coral span，不斜体不加粗 |
| h1 章节标题 | `STSong` | 48 | 400 | 1.2 | 0 |
| headline 页标题 | `STSong` | 34（唯一字号，一行，≤ 28 个汉字） | 400 | 1.3 | 0，强调词换 coral |
| h3 面板 / 卡标题 | `STSong` | 24 | 400 | 1.25 | 0 |
| stat 关键数字 | `STSong` | 48（单位 span 24） | 400 | 1 | -1，coral |
| 小结 / 结论句 | `STSong` | 19 | 400 | 1.5 | 0 |
| lead 封面 / 章节导语 | `Microsoft YaHei` | 19 | 300 | 1.7 | 0 |
| body 正文段落 | `Microsoft YaHei` | 18 | 300 | 1.6 | 0 |
| bullet 要点行 | `Microsoft YaHei` | 18 | 300 | 1.35 | 0；破折号 `—` 用 `Courier New` 18 / 400 |
| table 表格 | `Microsoft YaHei` | 17 | 400 | — | 表头字 cream-muted，高亮行 coral-light |
| label chrome / kicker / 页脚 / 面板标签 | 中文 `Microsoft YaHei`，西文与数字 `Courier New`（span） | 13 | 400 | 1.15 | 2px |
| watermark 水印数字 | `STSong` | 200 | 400 | 1 | -6 |
模板的 Playfair Display 400 映射为 STSong 400（全篇没有任何 700 的衬线）；Jost 300 映射为 Microsoft YaHei 300，但只用在 18px 以上，13px 小字一律 400；JetBrains Mono 映射为 Courier New，只放西文和数字，中文标签用 Microsoft YaHei 加 2px 字距补足打字机质感。模板里的斜体强调在中文里改为「只换珊瑚色」，西文也不加 italic，全篇无斜体、无下划线。

## 4. 母版三区
- 所有页面（封面、章节页、深页、浅页）的 Slide style 除 background 外逐字相同：`padding: '40px 102px 48px', position: 'relative'`；版心 x 102～1178，宽 1076。
- A 页眉区（内容页 y 40～184，高 144）= A1 chrome 行 28（y 40～68：左章节标签、右 mono 页码，底部 1px 发丝线）+ 20 间距 + A2 标题块 96（y 88～184：13px 珊瑚 kicker → 36px 珊瑚短线 → 一行 34px 衬线标题）。标题只有 34px 一种字号，nowrap，最多 28 个汉字；放不下改写文案，不缩字号、不折行。kicker 可含 `Courier New` span 放西文与年份。
- B 内容区（y 208～620，宽 1076、高 412）：页面内容只能放在这个 Box 里。三栏对照页 = 351 面板 + 14 + 47 小结条；表格页 = 284 表格 + 16 + 112 结论条。
- C 页脚条（所有页面 y 640～672，高 32）：1px 发丝线 + 左「单位 · 会议名」+ 右 mono `NN / 总页`。每页必有，封面也有（左汇报人 / 导师，右日期 · 稿别）。
- 两种表面各一个外壳，数值完全相同、只差四个颜色：深页 `shell/content.jsx`（底 `#192B1B` / 字 `#D4CFBF` / 次级字 `#9C9E8E` / 发丝线 `#3A4434` / kicker `#E27871`），浅页 `shell/content_light.jsx`（底 `#E8E4D6` / 字 `#192B1B` / 次级字 `#5A6253` / 发丝线 `#CBCABC` / kicker `#9E3B34`）。深浅逐页交替，浅页用于对照、列表、材料图这类「读一页纸」的内容，深页用于封面、章节页、数据与表格。

### 固定骨架

内容页外壳以本套 `shell/content.jsx` 为准（占位 `〔…〕`，B 区标记 `〔B区内容〕`），用 `scripts/build_page.py build` 拼装，不照抄手打；浅页以 `shell/content_light.jsx` 为准，`check` 时浅页对 `content_light.jsx`、深页对 `content.jsx`。

封面外壳（无顶部 chrome；右下角 200px 水印数字；主体在 580 高的左栏里垂直居中；页脚与内容页同位）：
```jsx
<Slide style={{ background: '#192B1B', padding: '40px 102px 48px', position: 'relative' }}>
    <Box style={{ position: 'absolute', left: 938, top: 376, width: 240, height: 240, alignItems: 'flex-end' }}>
        <Text style={{ fontFamily: 'STSong', fontSize: 200, fontWeight: 400, color: '#2A3626', lineHeight: 1, letterSpacing: -6, whiteSpace: 'nowrap' }}>01</Text>
    </Box>
    <Box style={{ width: 720, height: 580, justifyContent: 'center' }}>
        <Text style={{ fontFamily: 'Microsoft YaHei', fontSize: 13, fontWeight: 400, color: '#E27871', letterSpacing: 2, lineHeight: 1.15, whiteSpace: 'nowrap' }}><>〔学院 · 组会名〕 · <span style={{ fontFamily: 'Courier New' }}>〔YYYY〕</span></></Text>
        <Box style={{ width: 36, height: 1, background: '#C8524A', marginTop: 12 }} />
        <Text style={{ fontFamily: 'STSong', fontSize: 60, fontWeight: 400, color: '#D4CFBF', lineHeight: 1.2, letterSpacing: 0, marginTop: 22, whiteSpace: 'nowrap' }}>〔主标第一行，≤12 字〕<br />〔主标第二行，≤12 字，强调词用 coral span〕</Text>
        <Text style={{ fontFamily: 'Microsoft YaHei', fontSize: 19, fontWeight: 300, color: '#9C9E8E', lineHeight: 1.7, marginTop: 22, width: 600 }}>〔导语，两行以内〕</Text>
    </Box>
    <Box style={{ flexDirection: 'row', justifyContent: 'space-between', alignItems: 'flex-start', height: 32, borderTop: '1px solid #3A4434', paddingTop: 13, marginTop: 20 }}>
        <Text style={{ fontFamily: 'Microsoft YaHei', fontSize: 13, fontWeight: 400, color: '#9C9E8E', letterSpacing: 2, lineHeight: 1.15, whiteSpace: 'nowrap' }}>〔汇报人 · 导师〕</Text>
        <Text style={{ fontFamily: 'Microsoft YaHei', fontSize: 13, fontWeight: 400, color: '#9C9E8E', letterSpacing: 2, lineHeight: 1.15, whiteSpace: 'nowrap' }}><><span style={{ fontFamily: 'Courier New' }}>〔YYYY.MM.DD〕</span> · 〔稿别〕</></Text>
    </Box>
</Slide>
```

章节页外壳（与封面同构：水印换成章节序号，kicker 写「NN / 章节名」，标题降到 48px 最多 3 行，页脚与内容页一致）：
```jsx
<Slide style={{ background: '#192B1B', padding: '40px 102px 48px', position: 'relative' }}>
    <Box style={{ position: 'absolute', left: 938, top: 376, width: 240, height: 240, alignItems: 'flex-end' }}>
        <Text style={{ fontFamily: 'STSong', fontSize: 200, fontWeight: 400, color: '#2A3626', lineHeight: 1, letterSpacing: -6, whiteSpace: 'nowrap' }}>〔NN〕</Text>
    </Box>
    <Box style={{ width: 720, height: 580, justifyContent: 'center' }}>
        <Text style={{ fontFamily: 'Microsoft YaHei', fontSize: 13, fontWeight: 400, color: '#E27871', letterSpacing: 2, lineHeight: 1.15, whiteSpace: 'nowrap' }}><><span style={{ fontFamily: 'Courier New' }}>〔NN〕</span> / 〔章节名〕</></Text>
        <Box style={{ width: 36, height: 1, background: '#C8524A', marginTop: 12 }} />
        <Text style={{ fontFamily: 'STSong', fontSize: 48, fontWeight: 400, color: '#D4CFBF', lineHeight: 1.2, letterSpacing: 0, marginTop: 22, whiteSpace: 'nowrap' }}>〔章节标题，最多 3 行，每行 ≤15 字〕</Text>
        <Text style={{ fontFamily: 'Microsoft YaHei', fontSize: 19, fontWeight: 300, color: '#9C9E8E', lineHeight: 1.7, marginTop: 22, width: 600 }}>〔章节导语，两行以内〕</Text>
    </Box>
    <Box style={{ flexDirection: 'row', justifyContent: 'space-between', alignItems: 'flex-start', height: 32, borderTop: '1px solid #3A4434', paddingTop: 13, marginTop: 20 }}>
        <Text style={{ fontFamily: 'Microsoft YaHei', fontSize: 13, fontWeight: 400, color: '#9C9E8E', letterSpacing: 2, lineHeight: 1.15, whiteSpace: 'nowrap' }}>〔单位 · 会议名〕</Text>
        <Text style={{ fontFamily: 'Courier New', fontSize: 13, fontWeight: 400, color: '#9C9E8E', letterSpacing: 2, lineHeight: 1.15, whiteSpace: 'nowrap' }}>〔NN / 总页〕</Text>
    </Box>
</Slide>
```

| 区 | 页面 | x | y | w | h |
|---|---|---|---|---|---|
| A1 chrome 行（含底部发丝线） | 内容页 | 102 | 40 | 1076 | 28 |
| A2 标题块（kicker + 短线 + 34px 标题） | 内容页 | 102 | 88 | 1076 | 96 |
| A2 · 珊瑚短线 | 内容页 | 102 | 116 | 36 | 1 |
| B 内容区 | 内容页 | 102 | 208 | 1076 | 412 |
| C 页脚条（含顶部发丝线） | 全部页面 | 102 | 640 | 1076 | 32 |
| 封面 / 章节页主体栏 | 封面类 | 102 | 40 | 720 | 580 |
| 水印数字 | 封面类 | 938 | 376 | 240 | 240 |

纵向算账：内容页 40（padding-top）+ 28（A1）+ 20 + 96（A2）+ 24 + 412（B）+ 20 + 32（C）+ 48（padding-bottom）= 720；封面类 40 + 580（主体栏）+ 20 + 32（C）+ 48 = 720。

硬规则：所有内容页必须从对应表面的外壳复制，只替换占位文字和 B 区内部；不得改动外壳里的任何数值。B 区放不下就删内容或拆页，不得压缩页眉页脚或改 padding。
- 13px 标签一律 `lineHeight: 1.15`，是为了在 28 / 32 高的 chrome 与页脚里装下；不得改成 1.4 以上。
- kicker 里没有西文时删掉 `Courier New` span，只留中文；不得改 Text 的 style。
- B 区内部另起一层 Box 排版，子项高度之和加间距必须等于 412（横排各栏写死 `height`，竖排写 `marginTop`）；文字高度按 字号 × 行高 × 1.2 × 行数 估。

**封面标题容量（硬上限）**：60px 衬线主标，nowrap，栏宽 720：最多 3 行，每行最多 12 个汉字；西文大写每行最多 16 个字母（含空格），西文不得自动折行，放不下就不写西文行。章节页 48px 标题最多 3 行、每行最多 15 个汉字。标题与导语、页脚之间必须留出空隙，任何情况下标题不得压到页脚上。

**装饰与文字不重叠（硬约束）**：水印数字占 x 938～1178、y 376～616，只出现在封面与章节页；封面 / 章节页的所有文字容器宽度 ≤ 720（x 102～822），不进入该区域；页脚在 y 640 以下，水印底边 616 不压页脚。36px 珊瑚短线只在 A2 标题块 y 116 一处（封面 / 章节页在 kicker 下 12px 处）。装饰性 SVG / Box 的矩形与任何文字容器的矩形不得相交；页型没有范例时，装饰只照本套封面或内容页范例的位置放，不另加。

### 图位（配图规则）
- 本套不放 AI 生成的配图（当前环境没有生图工具）：封面、章节页、内容页都以 §6 的标志性处理作主视觉。
- 只有用户提供的材料图可以上页，作为证据图，每页最多一张，图位 x 766 / y 208 / 412×412 正方形（B 区右侧，左侧 632 宽文字列 x 102～734），优先放在浅页；按真实比例居中放置（`<Image>` 宽高 = 图位边长按真实宽高比缩放），留边色深页 `#1E3221` / 浅页 `#DEDAD0`。`<Image>` 只写相等的 width / height，不写 objectFit，不加描边、相框、圆角、阴影，图注一行放图下方（13px 次级字色）。
- 材料图上页的门槛：只有照片、真实图表、界面截图这类重画不出来的内容才放，且原图短边 ≥ 412（放大会糊）。材料里的表格、流程图、对比图、文字截图一律不贴图，改用原生 `<Table>`、Box 排版或 SVG 重画；重画不了又不清晰的，宁可不放。
- 图位宽高不得改；没有材料图就不留图位，不得用色块或生成图冒充。

## 5. 版式清单
| 文件 | 类型 | 骨架 |
|---|---|---|
| `slides/01_cover.slide` | cover | 深页；无顶部 chrome；左栏 720 宽垂直居中：13px 珊瑚 kicker → 36px 短线 → 两行 60px 衬线主标（强调词 coral）→ 19px 导语；右下 200px 水印「01」；C 区左汇报人 / 导师、右日期 · 稿别 |
| `slides/02_compare.slide` | compare | 浅页外壳；B 区：三栏 324 宽面板（13px 标签行带底部发丝线 → 两行 24px 衬线小标 → 三行 18px 正文 → 三条破折号要点），栏间 52 宽、中置 1px 竖发丝线；底部 47 高发丝线小结条（「— 小结」+ 19px 衬线结论句） |
| `slides/03_table.slide` | table | 深页外壳；B 区：284 高原生 Table（表头 forest-alt 填色 + 次级字色，行线 1px 发丝线，最优行 coral-light）→ 112 高发丝线结论条（左 48px 珊瑚关键数字 + 右 18px 结论两行）；数据口径写在 C 区左端 |

## 6. 标志性处理
1. 衬线永不加粗：所有 STSong 一律字重 400，从 200px 水印到 19px 结论句没有一处 700；层级只靠字号和颜色。
2. kicker → 36px 珊瑚短线 → 标题，三件一组，每页只出现一次（内容页在 A2，封面 / 章节页在主体栏顶部）；B 区里不再放短线。
3. 强调只换色不换形：标题、小标里的强调词用 coral span；中文不斜体、西文也不斜体、没有下划线。
4. 全部 chrome 是 13px 字距 2 的小字：中文 Microsoft YaHei、西文数字 Courier New；页眉底、页脚顶各一条 1px 发丝线把页面框成「一页纸」，封面也保留页脚。
5. 破折号 `—` 是唯一的项目符，Courier New 珊瑚小字色，前置 22 宽；不用圆点、方块、图标。
6. 深森绿与暖纸面逐页交替，只换四个颜色；200px 水印数字只在封面与章节页右下角出现，颜色 `#2A3626`，不入内容页。

## 7. Do / Don't
Do
- 页标题一律 STSong 34 / 400 一行，写成结论句（「零样本基础模型首次…」），强调词换 coral。
- 正文 18px Microsoft YaHei 300、行高 1.6；要点行 1.35；13px 标签 400 字重、行高 1.15、字距 2。
- 小字珊瑚按表面选档：深页 `#E27871`、浅页 `#9E3B34`；`#C8524A` 只给 24px 以上的字和短线。
- 分隔只用 1px 发丝线（`borderTop / borderBottom / 1 宽竖 Box`），颜色随表面切换；表格行线也是同一根发丝线。
- 表格用原生 `Table`，表头填 forest-alt、字 cream-muted，最优行整行换 coral-light；表下必有关键数字 + 结论条。
- 每个内容页有三层：A2 标题、B 主体、B 底部小结 / 结论条；多栏页用竖发丝线做物理连接、用小结条做共享锚点。
Don't
- 不给任何 Box 加 borderRadius、boxShadow、渐变、rgba 表面色；不用色块做卡片，面板只靠发丝线和字号比例。
- 不用 `#FFFFFF`；不把 coral 用在段落、底色或大面积。
- 不引入第二彩色（宝蓝、金、黄、紫）；不用 2px 以上的结构线。
- 不用斜体、下划线、粗体衬线；不在中文上做大写转换。
- 不在内容页放水印数字；不把封面主标铺到 x 822 以右。
- 不用生成配图；材料图只按 §4「图位」的 412×412 证据位放，图上不叠字。
- 不省略 chrome 与页脚，不挪动它们的位置；13px 标签不得改行高。
