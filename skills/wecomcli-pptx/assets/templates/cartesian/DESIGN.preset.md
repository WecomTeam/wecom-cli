---
name: 专题汇报
---

## 1. 气质一句话
纯白纸面上只有墨黑衬线标题、灰正文和 1px 浅灰细线，圆规环在角落漂着，像博物馆图录里夹着一张读书笔记。

## 2. 色彩池

> **本节是默认配色，不是限制。** 模板只定版式：外壳、分区、字号层级、图位、标志性处理。颜色可以整体替换——按下表的「角色」一一对应给出新色值，用 `build_page.py init --map` 换色，再过 `contrast`。换色时保持角色数量不变（不多加一种彩色），本预设 §6 / §7 里关于「只用某种颜色」「禁止某色相」的话只约束默认配色。
| 角色 | hex | 面积占比上限 |
|---|---|---|
| 画布 bg-primary | `#FFFFFF` | 全页底色；不用暖色纸底；中性色全部取企微 base_gray 色阶 |
| 次级面 bg-secondary | `#E7E7E8` | ≤15%：表头底、圆形头像框 |
| 斑马行 / 卡面 | `#F7F7F8`、`#FFFFFF` | 表格交替行用 `#F7F7F8`；描图纸卡为白底 + 1px 浅灰描边 |
| 墨黑 text-primary | `#10141A` | 标题、表体、结论、唯一一条 horizontal-accent |
| 浅灰线 line | `#B5B8BB` | 全部 1px 结构线、圆规环、卡片描边 |
| 中性 | 正文 `#565A5F`（5.6:1）；小标签 / 页码 `#6D7176`（4.9:1，用于 ≥12px 的 chrome 与 eyebrow） | 文字专用 |
| 强调色 | 无；不引入深蓝表头与红黄高亮，强调靠字号与衬线切换 | 0% |

中性色与企微 base_gray 的对应：墨 `#10141A` = base_gray_100，正文 `#565A5F` = base_gray_070，标签 `#6D7176` = base_gray_060（白底对比 4.9:1，12px 小字的下限，不得再浅），线 `#B5B8BB` = base_gray_030，表头 `#E7E7E8` = base_gray_010，斑马 `#F7F7F8` = base_gray_003。不使用专项语义色标码。

## 3. 字体系统（1280×720 换算）
| 角色 | fontFamily | 字号 | 字重 | 行高 | 字距 |
|---|---|---|---|---|---|
| 封面中文题 | STSong | 44 | 700 | 1.3 | 0 |
| 封面英文题 | Times New Roman | 18 | 400 italic | 1.55 | 0 |
| 页标题 h2（结论句） | STSong | 32 | 700 | 1.3 | 0 |
| 卡片标题 | STSong | 22 | 700 | 1.3 | 0 |
| 结论条 | STSong | 19 | 700 | 1.6 | 0 |
| 圆环内字母 / vs | Times New Roman | 15–17 | 400 | 1 | 0，颜色 #6D7176 |
| 正文 | Microsoft YaHei | 17 | 400 | 1.75 | 0 |
| 表格 | Microsoft YaHei | 17 | 400 | — | 0，表头 #565A5F、表体 #10141A |
| label / eyebrow / 卡内规格键 | Microsoft YaHei | 13–14 | 500 | 1 | 1.5–2.5px，颜色 #6D7176 |
| 规格值 / 元信息 | Microsoft YaHei | 16 | 400 | 1.4 | 0，墨黑 |
| 页码 counter | Courier New | 12 | 400 | 1 | 2px |

## 4. 母版三区
- 页面 padding：所有页（含封面）一律 `48px 52px 68px 52px`，Slide 为纵向 flex、`gap: 24`，内容宽 1176；不做垂直居中，页眉与内容区的 y 由固定数值决定；无顶栏。
- A 标题块 y 48–128（高 80）：label（13px，y 48–63）+ marginTop 12 的 h2（32px 衬线结论句，y 75–124）；h2 只允许一行，最多 32 个汉字（右上角固定有 140px 圆规环，x ≥ 1086）。
- B 内容区 y 152–652（宽 1176、高 500）：主体 388 高（双栏卡 / 表格）+ 间隔 22 + 结论条 90 高（1px 顶线 + paddingTop 16 + 两行 19px 衬线，最多约 100 个汉字）；单页只放一个主体。
- C 页脚条：左下 12px 灰色单位名（left 52, bottom 28，高 14，y 678–692），右下 `NN / 总页` 页码（right 52, bottom 28，高 14）。页码只有右下这一处，底部 68 留给它们。
- 封面沿用同一 Slide style 与同一 C 区；标题块固定在 x 52 / y 165–525（宽 720、高 360），圆规环固定在 left 824 / top 236，墨黑终止线固定在 left 52 / bottom 96。

### 固定骨架

内容页外壳：
内容页外壳以本套 `shell/content.jsx` 为准（占位 `〔…〕`，B 区标记 `〔B区内容〕`），用 `scripts/build_page.py build` 拼装，不照抄手打。

B 区内部的固定分配（主体 + 结论条，放进上面的空壳）：

```jsx
<Box style={{ width: 1176, height: 500 }}>
    <Box style={{ height: 388 }}>
    </Box>
    <Box style={{ flexDirection: 'row', alignItems: 'flex-start', gap: 28, borderTop: '1px solid #B5B8BB', paddingTop: 16, marginTop: 22, height: 90 }}>
        <Text style={{ fontSize: 13, fontWeight: 500, letterSpacing: 2.5, color: '#6D7176', lineHeight: 2.2, whiteSpace: 'nowrap' }}>结论</Text>
        <Text style={{ fontFamily: 'STSong', fontSize: 19, fontWeight: 700, color: '#10141A', lineHeight: 1.6 }}>
            〔结论，最多两行〕
        </Text>
    </Box>
</Box>
```

封面外壳（Slide 开标签、单位名、页码与内容页逐字相同）：

```jsx
<Slide style={{ width: 1280, height: 720, background: '#FFFFFF', position: 'relative', padding: '48px 52px 68px 52px', flexDirection: 'column', gap: 24, fontFamily: 'Microsoft YaHei' }}>
    <svg width={400} height={400} viewBox='0 0 400 400' style={{ position: 'absolute', left: 824, top: 236 }}>
        <rect x='0' y='0' width='400' height='400' fill='#FFFFFF' />
        <circle cx='200' cy='200' r='193' fill='none' stroke='#B5B8BB' strokeWidth='1' opacity='0.7' />
        <circle cx='200' cy='200' r='154' fill='none' stroke='#B5B8BB' strokeWidth='1' strokeDasharray='3 4' opacity='0.5' />
    </svg>

    <Box style={{ width: 720, height: 360, marginTop: 117 }}>
    </Box>

    <Box style={{ position: 'absolute', left: 52, bottom: 96, width: 256, height: 1, background: '#10141A' }} />

    <Box style={{ position: 'absolute', left: 52, bottom: 28, height: 14 }}>
        <Text style={{ fontSize: 12, fontWeight: 500, letterSpacing: 2, lineHeight: 1, color: '#6D7176', whiteSpace: 'nowrap' }}>〔单位名〕</Text>
    </Box>
    <Box style={{ position: 'absolute', right: 52, bottom: 28, height: 14 }}>
        <Text style={{ fontFamily: 'Courier New', fontSize: 12, fontWeight: 400, letterSpacing: 2, lineHeight: 1, color: '#6D7176' }}>〔NN / 总页〕</Text>
    </Box>
</Slide>
```

| 区 | x | y | w | h | 说明 |
|---|---|---|---|---|---|
| A 页眉 | 52 | 48 | 1176 | 80 | label y 48–63；h2 y 75–124，32px 一行，最多 32 个汉字 |
| B 内容区 | 52 | 152 | 1176 | 500 | 主体 y 152–540（高 388）+ 间隔 22 + 结论条 y 562–652（高 90） |
| C 单位名 | 52 | 678 | 随文字 | 14 | absolute，left 52 / bottom 28，YaHei 12 |
| C 页码 | 右缘 1228 | 678 | 随文字 | 14 | absolute，right 52 / bottom 28，Courier New 12 |
| 封面标题块 | 52 | 165 | 720 | 360 | 封面专用；圆规环 x 824–1224 / y 236–636；终止线 x 52–308 / y 623 |
| 角落圆规环（每张内容页都有，随外壳） | 1086 | 16 | 140 | 140 | 内容页装饰只许放这一处，absolute，不占流式高度 |

纵向算式：padding-top 48 + A 80 + gap 24 + B 500 + padding-bottom 68 = 720；C 区不占流式高度，绝对定位在底部 68 之内（y 678–692）。封面：48 + marginTop 117 + 标题块 360 + 余白 127 + padding-bottom 68 = 720。

硬规则：所有内容页必须从此外壳复制，只替换占位文字和 B 区内部；不得改动外壳里的任何数值。B 区放不下就删内容或拆页，不得压缩页眉页脚或改 padding。

**封面标题容量（硬上限）**：44px 衬线标题，标题块 720 宽、360 高：中文题最多 2 行，每行最多 16 个汉字；英文题最多 2 行。标题与页脚、底带之间必须留出空隙，任何情况下标题不得压到页脚上。

**装饰与文字不重叠（硬约束）**：内容页右上圆规环占 x 1086～1226、y 16～156，标题 32px 一行最多 32 个汉字即不会碰到；封面圆规环占 x 824～1224、y 236～636，封面文字全部在 x 52～800 之内。装饰性 SVG / Box 的矩形与任何文字容器的矩形不得相交；页型没有范例时，装饰只照本套封面或内容页范例的位置放，不另加。

### 图位（配图规则）
- 本套不放 AI 生成的配图（当前环境没有生图工具）：封面、章节页、内容页都以 §6 的标志性处理作主视觉。
- 只有用户提供的材料图可以上页，作为证据图，每页最多一张，图位 x 52 或 668 / y 152 / 388×388 正方形（下方结论条照常）；按真实比例居中放置（`<Image>` 宽高 = 图位边长按真实宽高比缩放），不写 objectFit，不加描边、相框、圆角、阴影，图注一行放图下方。
- 材料图上页的门槛：只有照片、真实图表、界面截图这类重画不出来的内容才放，且原图短边 ≥ 图位边长（放大会糊）。材料里的表格、流程图、对比图、文字截图一律不贴图，改用原生 `<Table>`、Box 排版或 SVG 重画；重画不了又不清晰的，宁可不放。
- 图位宽高不得改；没有材料图就不留图位，不得用色块或生成图冒充。

## 5. 版式清单
| 文件 | 类型 | 骨架 |
|---|---|---|
| `slides/01_cover.slide` | cover | 题名式：label → 两行 44px 衬线中文题 → 两行 Times New Roman italic 英文题 → 640 宽浅灰线 → 汇报人 / 单位 / 汇报时间；右侧 400px 圆规环；左下 256×1 墨黑终止线 |
| `slides/02_compare.slide` | compare | label + h2 → 两张描图纸卡（40px 圆环 A / B + 灰色类别标签 / 衬线标题 / 正文 / 发丝线上三行规格键值）+ 中央 1px 竖线与 44px「vs」圆环 → 1px 线 + 结论条 |
| `slides/03_table.slide` | data | label + h2 → 6 列 6 行 Table（灰表头 #E7E7E8、#F7F7F8 斑马、仅 1px 浅灰底线）高 388，占内容区约 78% → 1px 线 + 结论条；右上 140px 小圆规环 |

## 6. 标志性处理
1. 所有结构线都是 1px `#B5B8BB`：卡片描边、表格行线、结论条顶线、中央竖线；没有 2px 以上边框，也没有深色表头。
2. 圆规几何：实线外环 + 虚线内环（strokeDasharray '3 4'）成对出现，透明度 0.4–0.7，每页最多两处且不压文字。
3. 圆环字母是模板 card-icon：40–44px 1px 灰环里放 Times New Roman 400 的 A / B / vs，颜色 #6D7176。
4. 标题一律 STSong 墨黑，小标签一律 YaHei 13px 灰色加 2.5px 字距，两者对比构成层级；正文与表体不加粗。
5. 唯一墨黑线 horizontal-accent（256×1）只在封面出现一次。
6. 右下等宽页码 + 左下灰色单位名，无顶栏、无页脚横线。

## 7. Do / Don't
Do
- Slide background 一律 `#FFFFFF`，页眉与内容区按 §4 固定坐标摆放（不做垂直居中），留白 ≥40%。
- 每个内容页末尾必有结论条：`borderTop: '1px solid #B5B8BB'` + 灰色「结论」标签 + 19px 衬线结论，固定高 90、最多两行。
- 卡片用 `border: '1px solid #B5B8BB'` + `background: '#FFFFFF'`，卡尾规格键值先 `flex: 1` 占位再锚底。
- Table 只配 `border.bottom`，表头 `#E7E7E8`、斑马 `#F7F7F8`，首列左对齐其余居中。
Don't
- 不画右缘圆点页码指示器或任何随总页数变化的翻页控件；页码只用右下 `NN / 总页`。
- 不引入红、橙、蓝、绿等饱和色，不用深蓝表头，不用灰色渲染标题。
- 不给卡片或表格加圆角、阴影、渐变；圆角只允许真正的圆（50%）。
- 不把正文、表体或标签加粗；层级靠字号与衬线 / 无衬线切换。
- 不一页放超过两处几何装饰，不让圆环压到标题或表格。
- 不在中文标签上照搬 uppercase，只保留字距；`#6D7176` 不用于 ≥ 一句话的正文。
