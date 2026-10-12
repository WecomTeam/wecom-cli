---
name: 经营分析
---

## 1. 气质一句话
藏青与米白双表面交替的情报简报式经营分析：衬线标题、mono 元数据、唯一的古金落在强调词、序号与关键数字上，每页以引导问题或小结收口。

## 2. 色彩池

> **本节是默认配色，不是限制。** 模板只定版式：外壳、分区、字号层级、图位、标志性处理。颜色可以整体替换——按下表的「角色」一一对应给出新色值，用 `build_page.py init --map` 换色，再过 `contrast`。换色时保持角色数量不变（不多加一种彩色），本预设 §6 / §7 里关于「只用某种颜色」「禁止某色相」的话只约束默认配色。
| 角色 | hex | 面积上限 |
|---|---|---|
| 暗表面 navy | #1C2644 | 整页底色（封面、数据页），叠 53px 网格纹理 |
| 亮表面 cream | #F0ECE3 | 整页底色（方法卡页），与 navy 逐页交替 |
| 唯一强调 gold | 暗页 #C8A870；亮页 #8A6A2E（同一支金的深档，#C8A870 在米白上对比只有 1.9，看不清） | ≤3%：标题强调词、金短线、卡片 2px 顶线、序号、关键数字、图表主系列、kicker |
| 暗页正文 text-warm | #E2DCD0 | 标题与正文；永不使用 #FFFFFF |
| 中性：暗页次级 #8A96A8（对比度 5.0:1，可作正文）、暗页装饰 #4E5A6E（只作图表对照系列 / 箭头，不作文字）；亮页主 #1A2030 / 次 #5A6270；发丝线 #1A4785（暗）/ #CAC4B4（亮）。宝蓝、警示红、数据黄一律不引入。 |

## 3. 字体系统（1280×720 已换算）
| 角色 | fontFamily | 字号 | 字重 | 行高 | 字距 |
|---|---|---|---|---|---|
| 封面中文主标 | STSong | 52 | 700 | 1.28 | 0，强调词换 gold |
| 封面英文长题 | Times New Roman | 21 | 400 | 1.55 | 强调短语 gold + italic（Signal moment） |
| h2 页标题 | STSong | 32 | 700 | 1.3 | 0，强调词换 gold |
| 引导问题 / 小结 | STSong | 19–23 | 600 | 1.35–1.6 | 0 |
| 卡片标题 | STSong | 22 | 600 | 1.4 | 0 |
| 序号 / 卡内锚定数字 | STSong | 30–34 | 600 | 1.0 | gold |
| 关键数字 | STSong | 72 | 600 | 1.0 | -1px，单位 30px，gold |
| 正文 / 洞察 | Microsoft YaHei | 18 | 400 | 1.7 | 0 |
| 说明 / 元信息 | Microsoft YaHei | 15 | 400 | 1.6 | 0 |
| kicker / chrome / 注脚 | Courier New | 12 | 500 | 1.4 | 西文 1–2px，中文 0 |

## 4. 母版三区
- 所有页（封面、亮页、暗页）的 Slide style 除 background 外逐字相同：`padding: '40px 96px', position: 'relative'`；Slide 内只放一个 `position: 'relative', width: 1088, height: 640` 的外壳 Box（x 96–1184，y 40–680），暗页另在 Slide 上 absolute 铺一张 1280×720 的网格 SVG，亮页不铺。
- A 页眉区（y 40–132，高 92）= 顶部 chrome 24（左章节标签、右 mono 页码，底部 1px 发丝线，y 40–64）+ 间距 8 + 标题槽 60（y 72–132，row、垂直居中、底部 1px 发丝线）。标题槽只有两种填法，字号都不可改：① h2 标题 STSong 32 / 700，单行，最多 30 个汉字；② 引导问题：12px 金色「引导问题」kicker + STSong 23 / 600 问句，单行，最多 38 个汉字。放不下就改写文案，不缩字号、不换行。
- B 内容区（y 152–628，宽 1088、高 476）：页面内容只能放在这个 Box 里。三栏卡用 2px 金顶线分隔（卡高 402 + 间距 18 + 小结条 56 = 476）；数据页左图 660×476 / 右文 392×476。
- C 页脚条（y 647–680，高 33）：1px 发丝线 + 左「单位 · 会议名」+ 右 mono `NN / 总页`，`marginTop: 19` 固定；封面无页脚，机构名放顶行 kicker（与内容页 chrome 同高 24、同基线，但不压发丝线），底部元信息条贴外壳底 y 632–680。
- 两种表面只换四个颜色，数值不变：亮页 发丝线 #CAC4B4 / 元数据 #5A6270 / 标题 #1A2030 / 底 #F0ECE3；暗页 发丝线 #1A4785 / 元数据 #8A96A8 / 标题 #E2DCD0 / 底 #1C2644。

### 固定骨架

纵向算账：40（padding-top）+ 24（chrome）+ 8 + 60（标题槽）+ 20 + 476（B）+ 19 + 33（C）+ 40（padding-bottom）= 720。

亮页外壳（标题槽放 h2）：
内容页外壳以本套 `shell/content.jsx` 为准（占位 `〔…〕`，B 区标记 `〔B区内容〕`），用 `scripts/build_page.py build` 拼装，不照抄手打。

暗页外壳（多一张网格 SVG，标题槽放引导问题；暗页放 h2 或亮页放引导问题时，只互换标题槽里的 Text，并把文字色换成所在表面的标题色）：

暗页外壳以本套 `shell/content_dark.jsx` 为准。

封面外壳（无 chrome 发丝线、无页脚；顶行 kicker 与内容页 chrome 同位，元信息条贴底）：

```jsx
<Slide style={{ background: '#1C2644', padding: '40px 96px', position: 'relative' }}>
    <svg width={1280} height={720} viewBox='0 0 1280 720' style={{ position: 'absolute', top: 0, left: 0 }}>
        <rect x='0' y='0' width='1280' height='720' fill='#1C2644' />
        <path d='M53.3 0V720M106.7 0V720M160.0 0V720M213.3 0V720M266.7 0V720M320.0 0V720M373.3 0V720M426.7 0V720M480.0 0V720M533.3 0V720M586.7 0V720M640.0 0V720M693.3 0V720M746.7 0V720M800.0 0V720M853.3 0V720M906.7 0V720M960.0 0V720M1013.3 0V720M1066.7 0V720M1120.0 0V720M1173.3 0V720M1226.7 0V720M0 53.3H1280M0 106.7H1280M0 160.0H1280M0 213.3H1280M0 266.7H1280M0 320.0H1280M0 373.3H1280M0 426.7H1280M0 480.0H1280M0 533.3H1280M0 586.7H1280M0 640.0H1280M0 693.3H1280' fill='none' stroke='#FFFFFF' strokeOpacity='0.035' strokeWidth='1' />
    </svg>
    <Box style={{ position: 'relative', width: 1088, height: 640 }}>
        <Box style={{ flexDirection: 'row', justifyContent: 'space-between', alignItems: 'flex-end', height: 24, paddingBottom: 9 }}>
            <Text style={{ fontFamily: 'Microsoft YaHei', fontSize: 12, fontWeight: 500, color: '#C8A870', whiteSpace: 'nowrap' }}>〔机构 · 部门〕</Text>
            <Text style={{ fontFamily: 'Courier New', fontSize: 12, fontWeight: 500, color: '#8A96A8', letterSpacing: 1.5, whiteSpace: 'nowrap' }}>〔ENGLISH STAMP · YYYY〕</Text>
        </Box>
        <Box style={{ flex: 1, justifyContent: 'flex-end' }}>
            〔封面主体：场次 kicker → 24px 金短线 → 两行主标 → 两行英文长题〕
            <Box style={{ flexDirection: 'row', justifyContent: 'space-between', alignItems: 'flex-start', borderTop: '1px solid #1A4785', paddingTop: 14, marginTop: 44, height: 48 }}>
                〔汇报人 / 审阅人 / 时间〕
            </Box>
        </Box>
    </Box>
</Slide>
```

| 区 | x | y | w | h |
|---|---|---|---|---|
| 外壳 Box | 96 | 40 | 1088 | 640 |
| A1 顶部 chrome（含底部发丝线） | 96 | 40 | 1088 | 24 |
| A2 标题槽（含底部发丝线） | 96 | 72 | 1088 | 60 |
| B 内容区 | 96 | 152 | 1088 | 476 |
| C 页脚条（含顶部发丝线） | 96 | 647 | 1088 | 33 |
| 暗页网格 SVG | 0 | 0 | 1280 | 720 |
| 封面顶行 kicker | 96 | 40 | 1088 | 24 |
| 封面元信息条 | 96 | 632 | 1088 | 48 |

所有内容页必须从此外壳复制，只替换占位文字和 B 区内部；不得改动外壳里的任何数值。B 区放不下就删内容或拆页，不得压缩页眉页脚或改 padding。

**封面标题容量（硬上限）**：两个槽都是 nowrap，版心 1088 宽，分开算——① 52px 衬线主标：最多 2 行，每行最多 20 个汉字；主标本身是英文时每行最多 38 个字符（小写约 0.55 × 52px），中英混排按汉字 52px、西文 29px 累加不超过 1088。② 主标下方 21px Times New Roman 英文长题：最多 2 行，每行最多 70 个字符（这个 70 只适用于 21px 这一槽，不得套用到 52px 主标）。标题与页脚、底带之间必须留出空隙，任何情况下标题不得压到页脚上。

**装饰与文字不重叠（硬约束）**：暗页网格 SVG 是背景，不算装饰占位；金短线只在标题上方 24px 处。装饰性 SVG / Box 的矩形与任何文字容器的矩形不得相交；页型没有范例时，装饰只照本套封面或内容页范例的位置放，不另加。

### 图位（配图规则）
- 本套不放 AI 生成的配图（当前环境没有生图工具）：封面、章节页、内容页都以 §6 的标志性处理作主视觉。
- 只有用户提供的材料图可以上页，作为证据图，每页最多一张，图位 x 96 或 664 / y 152 / 476×476 正方形，只放在亮页；按真实比例居中放置（`<Image>` 宽高 = 图位边长按真实宽高比缩放），不写 objectFit，不加描边、相框、圆角、阴影，图注一行放图下方。
- 材料图上页的门槛：只有照片、真实图表、界面截图这类重画不出来的内容才放，且原图短边 ≥ 图位边长（放大会糊）。材料里的表格、流程图、对比图、文字截图一律不贴图，改用原生 `<Table>`、Box 排版或 SVG 重画；重画不了又不清晰的，宁可不放。
- 图位宽高不得改；没有材料图就不留图位，不得用色块或生成图冒充。

## 5. 版式清单
- `01_cover.slide` · cover · 暗页：顶行机构 kicker（金）+ 英文场次戳 → 主体坐底：mono「年度经营分析会」→ 24px 金短线 → 两行衬线中文主标 → 两行 Times New Roman 英文长题（强调短语金色 italic）→ 发丝线元信息条（汇报人 / 审阅人 / 时间）。
- `02_cards.slide` · cards · 亮页：chrome → 标题槽 h2 → 三栏并列卡（2px 金顶线 / 金序号 / 衬线标题 / ≥110 字正文 / 锚底金色数据 + mono 注脚）→ 发丝线小结条（mono「— 小结」+ 衬线结论）→ 页脚。
- `03_data.slide` · data · 暗页：chrome → L0 引导问题条 → 左 660×476 原生簇状柱图（金 = 高峰、#4E5A6E = 平峰，navy 底）+ 右列关键数字 72px 金 / 说明 / 发丝线 / 洞察 ≥100 字 / 锚底 mono 数据口径 → 页脚。

## 6. 标志性处理
1. Signal moment：标题里的强调词换 gold；西文短语再加 italic，中文只换色不倾斜。
2. 所有 kicker / chrome / 页码 / 数据口径注脚一律 Courier New 12px，顶部 chrome 下必有 1px 发丝线。
3. 关键数字与序号都是 STSong 600 金色，永不用无衬线或正文色。
4. 24px 金短线只出现在封面 kicker 与主标之间；多卡页的物理连接用每卡 2px 金顶线，共享锚点用发丝线小结条。
5. 暗页整幅 53px 网格纹理（3.5% 白），亮页不加；两表面逐页交替。
6. 图表只用两色：金 = 本文 / 关注序列，#4E5A6E = 对照序列；图表背景与页面同色，坐标与图例文字 #8A96A8。

## 7. Do / Don't
Do：
- 每个内容页必须有结论或数据锚点：卡片页用底部小结条，数据页用引导问题条 + 关键数字。
- 正文 ≥18px、注脚与 chrome ≥12px；暗页正文用 #E2DCD0 或 #8A96A8，不用 #4E5A6E 写字。
- 卡片 ≥110 字，S4 数据锚底：先 `flex: 1` 占位再放金色数字与 mono 注脚。
- 区块分隔只用 `borderTop/borderBottom: '1px solid'`，颜色随表面切换。
Don't：
- 不给 Box 加 borderRadius、boxShadow、渐变底，不用色块做卡片。
- 不引入宝蓝 #1D5DB0 / 藏蓝顶条 / 红黄警示色，gold 是唯一彩色。
- 不用 #FFFFFF，不把 gold 用在段落或大面积底。
- 不在中文上加 letterSpacing、不做中文斜体；mono 只做元数据。
- 不用生成配图；材料图只按 §4「图位」的证据位放；徽标位用 mono 机构名代替。
