---
name: 战略咨询
---

## 1. 气质一句话
两色 risograph：米纸上只有一种电光钴蓝墨，坐标纸永远垫在底下，宋体做陈述、黑体做说明、等宽做编目，像一本日系趋势年鉴。

## 2. 色彩池

> **本节是默认配色，不是限制。** 模板只定版式：外壳、分区、字号层级、图位、标志性处理。颜色可以整体替换——按下表的「角色」一一对应给出新色值，用 `build_page.py init --map` 换色，再过 `contrast`。换色时保持角色数量不变（不多加一种彩色），本预设 §6 / §7 里关于「只用某种颜色」「禁止某色相」的话只约束默认配色。
| 角色 | hex | 面积上限 |
|---|---|---|
| paper 纸面 | #F0EBDE | 整页底色，也是 QR 块与客户 chevron 的填充 |
| ink 钴蓝 | #1F2BE0 | 唯一墨色：全部文字、规则线、chevron、像素格、毛刺柱；≤35% 面积 |
| grid 坐标纸 | #DBD8DE（钴蓝 10% 预混） | 32px 方格线、像素柱「暗格」；不可关闭 |
| ink-faint 淡线 | #CAC8DE（钴蓝 18% 预混） | 账簿行间分隔线，只用于同层密排信息 |
| paper-2 深纸 | #E6E0CE | 备用区域微差，本套未用 |
| 禁止：第二种色相、渐变、阴影、圆角、纯白 #FFFFFF；SVG 内禁 rgba（会失色，用预混 hex） |

## 3. 字体系统（1280×720 已换算）
| 角色 | fontFamily | 字号 | 字重 | 行高 | 字距 |
|---|---|---|---|---|---|
| display-hero 封面 | STSong；年份 span 用 Times New Roman | 104 | 700 / 400 | 1.04 | 中文 0，西文 -1 |
| headline / Action Title | STSong | 34 | 700 | 1.3 | 0 |
| ed-callout 封面副题 | STSong | 30 | 400 | 1.2 | 0 |
| vbig-numeral 巨数 | Times New Roman | 104 / 72 | 400 | 0.95 | -1.5 / -1；单位 span STSong 40 |
| table-name 账簿行名 | STSong | 18 | 400 | 1 | 0 |
| body 说明正文 | Microsoft YaHei | 16 | 400 | 1.5 | 0 |
| chevron 节点名 | Microsoft YaHei | 18 | 700 | 1 | 0，纸色字 |
| micro 大写标签 | Microsoft YaHei 700 13；纯西文段 span Courier New letterSpacing 1.5–2.2 | 13 | 700 | 1 | 见左 |
| mono-tag / tick / 页码 | Courier New | 13–15 | 400 | 1 | 0.5–1.5 |
| Source / Footnote | Microsoft YaHei | 13 | 400 | 1.5 | 0 |

## 4. 母版三区
- 所有页 Slide style 逐字相同：`padding: '19px 48px 14px 48px', position: 'relative'`；Slide 内先铺 1280×719 坐标纸 SVG（absolute、top 1、zIndex 0），再放一个 `position: 'relative', zIndex: 1, width: 1184, height: 687` 的外壳 Box（x 48–1232，y 19–706），其余元素全部在外壳里。
- A 页眉区（y 19–164，高 145）= 顶部 1.5px 钴蓝发丝线（y 19–20.5）+ 间距 16 + topbar 127.5（y 36.5–164：Action Title 左、mono lab-tag 右、`paddingBottom: 14`、底部 1.5px 规则线，规则线底边固定在 y 164）。topbar 高度按双行标题写死，`alignItems: 'flex-end'` 让标题贴着规则线向上长：单行标题顶 ≈ y 95，双行标题顶 ≈ y 42，规则线与 B 区都不动。标题只有一种字号 STSong 34 / 700 / 行高 1.3，宽 940，每行最多 27 个汉字，最多 2 行（≤ 54 字）；lab-tag 纯西文大写，≤ 26 字符。
- B 内容区（y 188–624，宽 1184、高 436）：页面内容只能放在这个 Box 里。价值链 / 账簿 / 像素柱按 1184 全宽排。
- C 页脚区（y 633–706，高 73，全部 absolute、坐标写死）：Source + Footnote 两行 13px 左下 `bottom: 26`（宽 960，y 633–680，只写一行时贴底）；页码 `NN / 总页` mono 右下 `bottom: 30`（底边 y 676）；底部 1.5px 发丝线 `bottom: 0`（y 704.5–706）。封面省略 Source 与 topbar，顶 / 底发丝线和页码与内容页逐字相同。

### 固定骨架

纵向算账：19（padding-top）+ 145（A：1.5 发丝线 + 16 + 127.5 topbar）+ 24 + 436（B）+ 9 + 73（C：47 Source 两行 + 24.5 + 1.5 发丝线）+ 14（padding-bottom）= 720；外壳 Box 687 = 145 + 24 + 436 + 9 + 73。

内容页外壳：
内容页外壳以本套 `shell/content.jsx` 为准（占位 `〔…〕`，B 区标记 `〔B区内容〕`），用 `scripts/build_page.py build` 拼装，不照抄手打。

封面 / 章节页外壳（同一 Slide、坐标纸、双发丝线与页码，无 topbar 与 Source）：

```jsx
<Slide style={{ background: '#F0EBDE', padding: '19px 48px 14px 48px', position: 'relative' }}>
    <svg width={1280} height={719} viewBox='0 0 1280 719' style={{ position: 'absolute', top: 1, left: 0, zIndex: 0 }}>
        <defs>
            <pattern id='grid' width='32' height='32' patternUnits='userSpaceOnUse'>
                <path d='M 32 0 L 0 0 0 32' fill='none' stroke='#DBD8DE' strokeWidth='1' />
            </pattern>
        </defs>
        <rect x='0' y='0' width='1280' height='719' fill='#F0EBDE' />
        <rect x='0' y='0' width='1280' height='719' fill='url(#grid)' />
    </svg>
    <Box style={{ position: 'relative', zIndex: 1, width: 1184, height: 687 }}>
        <Box style={{ width: 1184, height: 1.5, background: '#1F2BE0' }} />
        〔封面主体：104px 双行 display → 大写标签 → 副题 → 双栏署名；毛刺柱 rect 加在同一张 SVG 里，QR 块 absolute top 40 / right 0〕
        <Text style={{ position: 'absolute', right: 0, bottom: 30, fontFamily: 'Courier New', fontSize: 13, color: '#1F2BE0', letterSpacing: 1 }}>〔NN / 总页〕</Text>
        <Box style={{ position: 'absolute', left: 0, bottom: 0, width: 1184, height: 1.5, background: '#1F2BE0' }} />
    </Box>
</Slide>
```

| 区 | x | y | w | h |
|---|---|---|---|---|
| 坐标纸 SVG | 0 | 1 | 1280 | 719 |
| 外壳 Box | 48 | 19 | 1184 | 687 |
| A1 顶部发丝线 | 48 | 19 | 1184 | 1.5 |
| A2 topbar（含 14 内边距与 1.5 规则线） | 48 | 36.5 | 1184 | 127.5 |
| A2 内标题 Text | 48 | 贴规则线向上，单行顶 ≈ 95 / 双行顶 ≈ 42 | 940 | ≤ 107 |
| B 内容区 | 48 | 188 | 1184 | 436 |
| C1 Source / Footnote | 48 | 633（底 680） | 960 | 47 |
| C2 页码 | 右缘 1232 | 底 676 | 自适应 | 约 16 |
| C3 底部发丝线 | 48 | 704.5 | 1184 | 1.5 |

所有内容页必须从此外壳复制，只替换占位文字和 B 区内部；不得改动外壳里的任何数值。B 区放不下就删内容或拆页，不得压缩页眉页脚或改 padding。

**封面标题容量（硬上限）**：104px 衬线标题（nowrap），版心 1184 宽：最多 2 行（短标题只写一行，不得为凑两行硬拆；空出的那一行保留原有占位高度，其余元素位置不变），每行最多 8 个汉字（数字按半个汉字计；右侧 x ≥ 932 是毛刺柱占位）；放不下就改写标题，不得加第三行。标题与页脚、底带之间必须留出空隙，任何情况下标题不得压到页脚上。

**装饰与文字不重叠（硬约束）**：毛刺柱占 x 932～1116、y 58～664，QR 马赛克占右上 58×58；封面、章节页、结束页的全部文字左对齐、`whiteSpace: nowrap`、右边界不超过 x 900：104px 标题每行最多 8 个汉字，30px 副题每行最多 28 个汉字；不得把标题居中铺满整页。结束页照封面外壳排（左对齐 + 毛刺柱），不另设居中版式。装饰性 SVG / Box 的矩形与任何文字容器的矩形不得相交；页型没有范例时，装饰只照本套封面或内容页范例的位置放，不另加。

### 图位（配图规则）
- 本套不放 AI 生成的配图（当前环境没有生图工具）：封面、章节页、内容页都以 §6 的标志性处理作主视觉。
- 只有用户提供的材料图可以上页，作为证据图，每页最多一张，图位 x 652 / y 188 / 436×436 正方形（左侧 580 宽放文字）；按真实比例居中放置（`<Image>` 宽高 = 图位边长按真实宽高比缩放），不写 objectFit，不加描边、相框、圆角、阴影，图注一行放图下方。
- 材料图上页的门槛：只有照片、真实图表、界面截图这类重画不出来的内容才放，且原图短边 ≥ 图位边长（放大会糊）。材料里的表格、流程图、对比图、文字截图一律不贴图，改用原生 `<Table>`、Box 排版或 SVG 重画；重画不了又不清晰的，宁可不放。
- 图位宽高不得改；没有材料图就不留图位，不得用色块或生成图冒充。

## 5. 版式清单
- `01_cover.slide` · cover · 左：104px 双行 display（年份 Times New Roman）→ 大写标签 → 30px 宋体副题 → 双栏署名；右：8 级阶梯扫描线毛刺柱（SVG pattern）+ 右上 8×8 QR 纸底马赛克；双发丝线 + 页码。
- `02_structure.slide` · structure（价值链）· 固定 topbar（双行 Action Title）→ mono 节号行 → 6 节 chevron SVG（第 3 节客户环节纸底钴边）+ 独立 Text 节名 → 每节 3 行说明 + mono 规模 → 利润池账簿（表头 1.5px 实线、3 行淡线、↑↓— 前缀）→ Source/Footnote/页码。
- `03_data.slide` · data · 固定 topbar（单行 Action Title，贴规则线）→ 左 380：Times New Roman 巨数 ×2 + 标签 + 16px 洞察 → 右 764：8 列像素堆叠柱（12 格 × 27.5px，亮格钴蓝、暗格 #DBD8DE，数值标签嵌在柱顶格）+ 1.5px 基线 + mono 年份刻度。

## 6. 标志性处理
1. 每页底下都有 32px 坐标纸（SVG pattern 预混 #DBD8DE），页面上下各一条 1.5px 钴蓝发丝线内缩 48px。
2. 只有一种墨：强调靠放大、换宋体、加 mono 箭头或降不透明度（0.6），绝不换色。
3. 像素毛刺柱（阶梯扫描线）与 8×8 QR 纸底马赛克是唯一装饰，只出现在封面 / 章节 / 引言等陈述页。
4. topbar = 宋体标题左 + mono lab-tag 右 + 1.5px 规则线，是所有内容页的统一标题处理。
5. 图表用「像素堆叠柱」：等高格子亮 / 暗两态表示数值，直接呼应毛刺柱；数值与刻度用 Courier New。
6. 零圆角、零阴影、零渐变；密排信息之间只用 18% 淡线（#CAC8DE）。

## 7. Do / Don't
Do：
- 所有文字、线、形状一律 #1F2BE0；底色一律 #F0EBDE；SVG 与 Box 里的淡色用预混 hex。
- 宋体标题用 700（brief CJK 表），字号 34；正文黑体 16，说明 ≥16，脚注 13。
- 纯西文标签（VOLUME、CAGR、lab-tag）用 Courier New 加 1.5–2.2 字距；中文标签字距归零。
- 结构图形状放 SVG，节名 / 说明 / 数字全部用独立 Text 叠层，坐标与 viewBox 1:1。
- 页码 `NN / 总页` 放在底部发丝线上方 30px，不压线。
Don't：
- 不引入第二色相（含涨红跌绿）；涨跌只用 ↑ / ↓ / — 前缀与 0.6 不透明度区分。
- 不给任何形状圆角，不加 boxShadow、渐变；不用生成配图，材料图只按 §4「图位」的证据位放。
- 不用 `backgroundImage` 重复渐变画格子（渲染为灰块），不在 SVG 里写 rgba 或中文。
- 不让陈述页塞满；索引 / 数据 / 账簿页才密排。
- 不把标题写成话题词，必须是完整结论句。
