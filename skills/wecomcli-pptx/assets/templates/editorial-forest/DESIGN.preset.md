---
name: 教学课件
parent: academic
sub_style: 教学课件
---

## 1. 气质一句话
像一份安静的讲义：白底上由蓝色衬线大字撑住整页，等宽小标签当页眉页脚，全篇只有一个蓝色色阶，没有阴影、渐变和装饰线；适合大学课程的单讲课件，一页讲透一个知识点。

## 2. 色彩池

> **本节是默认配色，不是限制。** 模板只定版式：外壳、分区、字号层级、图位、标志性处理。颜色可以整体替换——按下表的「角色」一一对应给出新色值，用 `build_page.py init --map` 换色，再过 `contrast`。换色时保持角色数量不变（不多加一种彩色），本预设 §6 / §7 里关于「只用某种颜色」「禁止某色相」的话只约束默认配色。
| 角色 | hex | 用途 | 面积上限 |
|---|---|---|---|
| 白 white | `#FFFFFF` | 所有页面的整版底色；蓝色 tile 上的文字色；描边 tile 的填色 | 整版页 100% |
| 主蓝 blue | `#267EF0` | 33px 以上的衬线大标题；图表主系列；图例色块；monogram 圆章边线 | 只作大字与图形，不铺大面积底 |
| 面蓝 blue-surface | `#2373DA` | 承载白色小字的 tile 填色（白字对比度 4.6:1）；描边 tile 的 2px 边线 | ≤ 30% |
| 标签蓝 blue-label | `#1F68C5` | 白底上的 15～17px 标签、脚注、序号、坐标轴与数据标签 | 仅作文字 |
| 深字蓝 blue-ink | `#1B539A` | 浅蓝 tile 上的文字色 | 仅作文字 |
| 浅蓝 `#DCEBFD` / 次浅蓝 `#ACCFFA`：tile 填色轮换与图表次系列；墨 `#10141A`：白底正文。全部取自企微 accent_blue 色阶，禁止第二色相，禁止粉色、绿色，禁止 rgba 表面色。 |

## 3. 字体系统（1280×720 换算后）
| 角色 | fontFamily | 字号 | 字重 | 行高 | 字距 |
|---|---|---|---|---|---|
| display-hero 封面 / 收尾大标题 | `STSong` | 中文 112（西文原 147） | 500 | 1.1 | 0 |
| display 金句 / 大数字 | `STSong` | 中文 84～96；数字 120～147 | 500（数字可 700） | 1.05 | 0 |
| headline 页面主标题 | `STSong` | 50（唯一字号，一行，≤ 20 个汉字） | 500 | 1.15 | 0 |
| title-card tile 标题 | `STSong` | hero tile 50 / 普通 33 | 500 | 1.15～1.2 | 0 |
| body 正文 | `STSong` | 20 / 卡片内 17 | 400 | 1.6 | 0 |
| label 页眉标签 | 中文 `Microsoft YaHei`，西文 `Courier New`（span） | 16 | 500 | 1.4 | 西文 3px / 中文 4px |
| caption 页脚 / tile 序号 / tile 脚标 | 同上 | 页脚 15；tile 序号 16；tile 脚标 15 | 500 | 1.4 | 西文 3px / 中文 4px |
| monogram 圆章字 | `Courier New` | 19 | 500 | 1 | 2px |
西文标签全大写；中文标签不做大写转换，用 4px 字距补足「排字机」质感。

## 4. 母版三区
- 页面 padding：所有页面（含封面、金句页、总结页）一律 `60px 80px 56px`，Slide 主轴 `gap: 20`；版心 x 80～1200，宽 1120。
- A 页眉区：内容页 y 60～166（高 106）= topbar 行（y 60～84，两端各一条 16px mono 标签）+ 12 间距 + 一行 50px 主蓝衬线标题（y 96 起）。标题只有 50px 一种字号，只许一行、最多 20 个汉字，写成结论句；超长就改写，不得缩字号或折行。封面 A 区高 88（y 60～148），左标签仍在 y 60～84，右侧放 88px 圆章。
- B 内容区：内容页 y 186～620，1120×434，所有页面内容只能放在这个 Box 里（tile grid 整体 434 高，2×2 小 tile 各 209 高；图表页 = 24 图例行 + 16 + 394 图表）。封面 B 区 y 168～620，1120×452，三行 112px 标题在其中垂直居中。
- C 页脚条：所有页面 y 640～664（高 24），左端一条 15px mono 说明（来源 / 院系名），右端 15px「课程名 · NN/总页」页码；封面右端改放主讲人与日期。每页必有，不得省略。
- 页码只出现在 C 区右端；topbar 右侧只放说明性标签，不放页码。monogram 圆章只出现在封面与总结页。
- 金句页、总结页沿用封面外壳（需要时删去圆章，左标签保留）；stats、framework 等其余内容页一律用内容页外壳。

### 固定骨架

内容页外壳以本套 `shell/content.jsx` 为准（占位 `〔…〕`，B 区标记 `〔B区内容〕`），用 `scripts/build_page.py build` 拼装，不照抄手打。
封面 / 金句 / 总结页外壳：
```jsx
<Slide style={{ background: '#FFFFFF', padding: '60px 80px 56px', gap: 20 }}>
    <Box style={{ flexDirection: 'row', justifyContent: 'space-between', alignItems: 'flex-start', height: 88 }}>
        <Box style={{ justifyContent: 'center', height: 24 }}>
            <Text style={{ fontFamily: 'Microsoft YaHei', fontSize: 16, fontWeight: 500, color: '#1F68C5', letterSpacing: 4 }}><><span style={{ fontFamily: 'Courier New', letterSpacing: 3 }}>〔年份或讲次〕</span> 〔课程名〕</></Text>
        </Box>
        <Box style={{ width: 88, height: 88, borderRadius: 44, border: '2px solid #267EF0', justifyContent: 'center', alignItems: 'center' }}>
            <Text style={{ fontFamily: 'Courier New', fontSize: 19, fontWeight: 500, color: '#1F68C5', letterSpacing: 2 }}>〔讲次〕</Text>
        </Box>
    </Box>

    <Box style={{ width: 1120, height: 452, justifyContent: 'center' }}>
        <Text style={{ fontFamily: 'STSong', fontSize: 112, fontWeight: 500, color: '#267EF0', lineHeight: 1.1, letterSpacing: 0, whiteSpace: 'nowrap' }}>
            〔标题第一行，≤9 字〕<br />
            〔标题第二行，≤9 字〕<br />
            〔标题第三行，可删〕
        </Text>
    </Box>

    <Box style={{ flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center', height: 24 }}>
        <Text style={{ fontFamily: 'Microsoft YaHei', fontSize: 15, fontWeight: 500, color: '#1F68C5', letterSpacing: 4 }}>〔院系名〕</Text>
        <Text style={{ fontFamily: 'Microsoft YaHei', fontSize: 15, fontWeight: 500, color: '#1F68C5', letterSpacing: 4 }}><>〔主讲人〕 <span style={{ fontFamily: 'Courier New', letterSpacing: 3 }}>/ 〔YYYY.MM.DD〕</span></></Text>
    </Box>
</Slide>
```
| 区 | 页面 | x | y | w | h |
|---|---|---|---|---|---|
| A 页眉区 | 内容页 | 80 | 60 | 1120 | 106 |
| A · topbar 行（左右标签） | 全部页面 | 80 | 60 | 1120 | 24 |
| A · 标题（50px 一行） | 内容页 | 80 | 96 | 1120 | 70 |
| A 页眉区 | 封面类 | 80 | 60 | 1120 | 88 |
| A · 圆章 | 封面类 | 1112 | 60 | 88 | 88 |
| B 内容区 | 内容页 | 80 | 186 | 1120 | 434 |
| B 内容区 | 封面类 | 80 | 168 | 1120 | 452 |
| C 页脚条 | 全部页面 | 80 | 640 | 1120 | 24 |

封面两处占位按内容填，不要默认带出季度字样：`〔年份或讲次〕` 与圆章 `〔讲次〕` 都落在 `Courier New` 上，只能填西文和数字（如 `2026`、`LECTURE 06`、`WEEK 12`）；圆章最多 4 个字符（如 `06`、`L6`、`W12`、`26`），内容没有讲次概念（专题讲座、导论课等）时填年份后两位，或按上文删去圆章。

纵向算账：内容页 60（padding-top）+ 106（A）+ 20（gap）+ 434（B）+ 20（gap）+ 24（C）+ 56（padding-bottom）= 720；封面类 60 + 88 + 20 + 452 + 20 + 24 + 56 = 720。

硬规则：所有内容页必须从此外壳复制，只替换占位文字和 B 区内部；不得改动外壳里的任何数值。B 区放不下就删内容或拆页，不得压缩页眉页脚或改 padding。
- 标签里没有西文时删掉对应的 `span`，只留中文；不得改 Text 的 style。
- B 区内部另起一层 Box 排版（横排写 `flexDirection: 'row', height: 434`，竖排写 `height: 434, gap: 16`），子项高度之和加 gap 必须等于 434。
- 112px 三行标题实测占 444 高，只放得进封面类的 452 高 B 区；内容页 B 区里不放 84px 以上的多行大字。

**封面标题容量（硬上限）**：112px 衬线标题，B 区 1120×452：最多 3 行，每行最多 9 个汉字；西文大写每行最多 13 个字母（含空格），西文不得自动折行，放不下就不写西文行。确需 4 行时整段降到 84px（每行最多 12 个汉字），不得保持 112px 写 4 行。标题与页脚、底带之间必须留出空隙，任何情况下标题不得压到页脚上。

**装饰与文字不重叠（硬约束）**：封面圆章占 x 1112～1200、y 60～148，封面左标签与标题不进入该区域。装饰性 SVG / Box 的矩形与任何文字容器的矩形不得相交；页型没有范例时，装饰只照本套封面或内容页范例的位置放，不另加。

### 图位（配图规则）
- 图位（唯一，正方形）：x 80 / y 186 / 434×434，左；右侧 670 宽文字列（两块 209 高 tile）。写法见 `slides/04_image.slide`；封面、章节页及其他内容页不放图。
- 本套不放 AI 生成的配图（当前环境没有生图工具）；图位只放合格的用户材料图，即课堂 / 实验室 / 机房照片、真实实验截图、仪器图这类重画不出来的内容（门槛见 README §5）；材料里的表格、流程、对比图、公式截图一律不贴图，改用原生 `<Table>`、Box 排版或 SVG 重画。
- `<Image>` 只写相等的 width / height，不写 objectFit；图不加描边、相框、包边线、圆角、阴影；图注一行放图下方（C 区左端）。材料图写法见 README §5。
- 有合格材料图时图文页最多 2～4 张、不连续；没有就不用图文页，改用本套其他版式，不留空图位。

## 5. 版式清单
| 文件 | 类型 | 骨架 |
|---|---|---|
| `slides/01_cover.slide` | cover | 讲次封面：封面外壳；A 区 88 高（左「讲次 + 课程名」标签 + 右 88px 蓝框讲次圆章）→ 452 高 B 区内三行 112px 衬线标题（课程 / 本讲主题 / 西文短句）左对齐、垂直居中 → C 区两端 footline（院系名 / 主讲人与日期） |
| `slides/02_cards.slide` | cards | 本讲提纲：内容页外壳；B 区：左 466×434 hero tile（回顾 / 引入）+ 右 2×2（311×209）知识点 tile，填色面蓝 / 浅蓝 / 白底蓝框 / 次浅蓝轮换，每 tile 序号在上、脚标钉底 |
| `slides/03_data.slide` | data | 实验数据：内容页外壳；B 区：右对齐的 24 高自绘 swatch 图例行 → 16 → 394px 原生双系列柱状图（主蓝 / 次浅蓝，如两种算法在不同规模上的用时对比）；标题写成一句可讲的结论；数据来源写在 C 区左端 |
| `slides/04_image.slide` | image | 课堂 / 实验照片：内容页外壳；B 区：左 434×434 正方形材料图位（只放用户材料图，图上不叠字）+ 右 670 宽一列两块 209 高文字 tile（上浅蓝标题 tile：标签 + 两行 33px 标题 + 钉底脚标；下白底蓝框要点 tile：标签 + 三条 17px 步骤）；图注写在 C 区左端 |

## 6. 标志性处理
1. 衬线字重 500、零阴影、超大显示字号（封面 112、数字可到 147）——大字本身就是主视觉。
2. 等宽字 chrome：所有标签、脚注、序号、图例、坐标轴都是 mono 小字 + 宽字距，绝不出现在标题尺度。
3. 每页 topbar：左 mono 标签，右侧说明标签或圆章；每页底部都有 footline，页码固定在 footline 右端。
4. 白底为主：内容页一律白底，蓝色只出现在大字、tile 与图形上；需要整版重色时只用面蓝，且全篇不超过两页。
5. tile 填色轮换：面蓝 + 浅蓝 + 次浅蓝 + 白底蓝框 混排，绝不整排同色。
6. 2px 发丝线是唯一分隔语言（KPI 行上方、meta 行上方、描边 tile 边框、圆章边线）。

## 7. Do / Don't
Do
- 显示标题一律 `STSong` 字重 500，中文字距 0、行高 1.1～1.2；西文部分可用 span 收 -2px 字距。
- 标签和脚注一律小写字号（15～17）+ 字距 3～4px；西文用 `Courier New` 全大写，中文用 `Microsoft YaHei` span。
- tile 用 `borderRadius: 6`，step tile 用 8；填色 Box 直接承载文字，不加边框；只有白底 tile 加 `2px solid #2373DA`。
- 图表用原生 `Chart`，`colors` 只给主蓝 / 次浅蓝 / 浅蓝，`background` 等于页面底色，`showLegend={false}` 后用 17px swatch Box 自绘图例。
- 一页一个知识点；标题写成学生能带走的结论句（如「两种遍历用时相近，都随边数线性增长」），不写话题标签。
- 配图只用 `Image` 放进「图位」小节规定的 434×434 正方形图位，只放用户材料图（课堂、实验室、机房照片或真实实验截图），不写 objectFit、不加圆角，全篇不出现 AI 生成的图。
- 版心 padding 固定 `60px 80px 56px`，留白要围绕大字而不是均匀撒开。
Don't
- 不加 `boxShadow`、渐变、rgba 表面色、圆角 > 8。
- 不引入第三种字体家族，不用斜体、下划线。
- 不用 1px 或 ≥ 3px 的结构线；边框只有 2px / 2.5px。
- 不引入粉、绿、黄、紫等第二色相；白字只放在面蓝 `#2373DA` 上，不放在主蓝或更浅的蓝上。
- 不在一页里叠两个竞争主体（例如图表 + 卡片组）；图文页里照片就是主体，不再加图表或大数字。
- 不在图位之外放图，不放任何 AI 生成的配图：不用图片做整版背景、tile 底纹或页眉装饰，图上不叠文字，不给图片加边框、阴影、滤色蒙层。
- 不在每页都放 monogram 圆章。
- 不省略 topbar 和 footline，不挪动它们的位置。
