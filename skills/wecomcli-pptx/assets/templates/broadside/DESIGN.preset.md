---
name: 政务工作
---

## 1. 气质一句话
海报式的巨型陈述标题撑满版面：封面朱红满版上两行金渐变大字，内页白底黑色粗体结论句，红只做卡头、胶囊和标记，金只做点睛。

## 2. 色彩池

> **本节是默认配色，不是限制。** 模板只定版式：外壳、分区、字号层级、图位、标志性处理。颜色可以整体替换——按下表的「角色」一一对应给出新色值，用 `build_page.py init --map` 换色，再过 `contrast`。换色时保持角色数量不变（不多加一种彩色），本预设 §6 / §7 里关于「只用某种颜色」「禁止某色相」的话只约束默认配色。
| 角色 | hex | 面积上限 |
|---|---|---|
| 朱红（封面满版底） | `#E63F26` | 封面 ≥50%，内页不用作底 |
| 深朱红（红卡头、PART 胶囊、标题短竖条、目录顶线、封面底带与胶囊底） | `#C42819` | 内页 8–20% |
| 高光红 / 粉红橙（仅底部三段装饰条后两段） | `#EE5530` / `#F26B40` | 仅装饰条 |
| 金（飘带、短条、方点、胶囊、粒子、细线） | `#E19825` / `#FFC000` | 内页 ≤5%，封面 15–30% |
| 浅金（封面小字、红卡头上的编号） | `#FFE79B` | 文字 |
| 深金（飘带尾翼） / 飘带字 | `#B8791A` / `#A81810` | 仅飘带 |
| 金胶囊上的字（章节页） | `#7A1710` | 文字 |
| 白（内页底、卡片底、红底上的字） | `#FFFFFF` | 内页 ≥60% |
| 正文黑 / 次级灰 / 弱灰 / 线灰 | `#1A1616` / `#4B5563` / `#6B7280` / `#E5E7EB` | 文字与 1px 线 |

## 3. 字体系统（1280×720）
| 角色 | fontFamily | 字号 | 字重 | 行高 / 字距 |
|---|---|---|---|---|
| 封面巨型陈述标题（金渐变） | Microsoft YaHei | 132 | 900 | 1 / 0，单行 Box 高 160 |
| 内页主标（结论句） | Microsoft YaHei | 内容页 42 / 目录页 48 | 900 | 1.2，单行不换行 |
| 目录章节名 | Microsoft YaHei | 40 | 900 | 1.2 |
| 红卡头名称 / 卡内小标 | Microsoft YaHei | 26 / 22 | 900 / bold | 1 / 1.3 |
| 正文 / 目录说明 | Microsoft YaHei | 18–19 | normal | 1.75 / 1.5 |
| 飘带副标 / 胶囊字 | Microsoft YaHei | 22 / 18 | bold / normal | 1 / 字距 3 |
| 中文页眉页脚 | Microsoft YaHei | 14 | normal | 字距 2 |
| PART 胶囊字 | Arial | 12–16 | bold | 字距 1 |
| CONTENTS、`/ 0N`、页码、目录页号 | Courier New | 14–22 | 700 | 字距 2–3 |

## 4. 母版三区（页面 padding 40 / 70，封面、目录、内容页相同）
- A 页眉区：y 40–152，高 112。上半是 30 高的页眉细线条（y 40–70，左 PART 胶囊 + 章节名，右页名，底部 1px `#E5E7EB`），下半是 60 高的标题行（y 92–152，红短竖条 12×44 + 42px 结论句）。标题只用 42px / 900 一种字号，单行不换行，最多 24 个汉字（含标点）。
- B 内容区：x 70 / y 176 / 1140×452；页面内容只放在这个 Box 里。三栏红头白卡 364×452，gap 24。图文页为左图 704×396 + 右红头白卡 412×452，gap 24（见「图位」）。
- C 页脚：绝对定位。y 660–669 红橙三段装饰条 678 / 333 / 269（x 0，通栏 1280）；y 678 左「年度党课宣讲」（left 70）、右 Courier New 页码（right 70），字号 14。
- 目录页：页眉细线条与页脚同上；标题块 y 94–210（CONTENTS + 金短条 + 48px 主标，单行，最多 16 个汉字），目录区 y 230–640（1140×410，4 行，每行 100）。
- 封面：顶行 y 40–70（左上编号 + 右上栏目名，无底线）+ 两条金细线；底部 y 658 金线 + y 660–720 深朱红底带（与内页装饰条同起于 y 660），页码同在 right 70 / top 678，改浅金 `#FFE79B`；栏目名已在右上，页脚左侧不再重复。

### 固定骨架

内容页外壳：
内容页外壳以本套 `shell/content.jsx` 为准（占位 `〔…〕`，B 区标记 `〔B区内容〕`），用 `scripts/build_page.py build` 拼装，不照抄手打。

目录页外壳（页眉细线条与页脚同内容页，标题块与目录区用自己的固定值）：

```jsx
<Slide style={{ width: 1280, height: 720, background: '#FFFFFF', position: 'relative', padding: '40px 70px 0 70px', fontFamily: 'Microsoft YaHei' }}>
    <Box style={{ flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center', height: 30, borderBottom: '1px solid #E5E7EB' }}>
        <Text style={{ fontSize: 14, letterSpacing: 2, color: '#6B7280' }}>〔整份标题〕</Text>
        <Text style={{ fontSize: 14, letterSpacing: 2, color: '#6B7280' }}>目录</Text>
    </Box>

    <Box style={{ marginTop: 24, flexDirection: 'row', justifyContent: 'space-between', alignItems: 'flex-end', height: 116 }}>
        <Box style={{ gap: 10 }}>
            <Text style={{ fontFamily: 'Courier New', fontSize: 15, fontWeight: 700, letterSpacing: 3, color: '#C42819', lineHeight: 1.2 }}>CONTENTS</Text>
            <Box style={{ width: 36, height: 3, background: '#E19825' }} />
            <Box style={{ flexDirection: 'row', alignItems: 'center', gap: 16, height: 72 }}>
                <Box style={{ width: 12, height: 46, background: '#C42819' }} />
                <Text style={{ fontSize: 48, fontWeight: 900, lineHeight: 1.2, color: '#1A1616', whiteSpace: 'nowrap' }}>〔目录标题〕</Text>
            </Box>
        </Box>
        <Box style={{ flexDirection: 'row', alignItems: 'center', gap: 16, marginBottom: 12 }}>
            <Box style={{ flexDirection: 'row', gap: 8 }}>
                <Box style={{ width: 10, height: 10, background: '#E19825' }} /><Box style={{ width: 10, height: 10, background: '#E19825', opacity: 0.6 }} /><Box style={{ width: 10, height: 10, background: '#E19825', opacity: 0.3 }} />
            </Box>
            <Box style={{ height: 34, padding: '0 20px', borderRadius: 17, background: '#E19825', alignItems: 'center', justifyContent: 'center' }}>
                <Text style={{ fontSize: 16, fontWeight: 'bold', color: '#7A1710', lineHeight: 1 }}>〔栏目名 · 共几讲〕</Text>
            </Box>
        </Box>
    </Box>

    <Box style={{ marginTop: 20, width: 1140, height: 410 }}>
    </Box>

    <Box style={{ position: 'absolute', left: 0, top: 660, width: 1280, height: 9, flexDirection: 'row' }}>
        <Box style={{ width: 678, height: 9, background: '#C42819' }} />
        <Box style={{ width: 333, height: 9, background: '#EE5530' }} />
        <Box style={{ width: 269, height: 9, background: '#F26B40' }} />
    </Box>
    <Text style={{ position: 'absolute', left: 70, top: 678, fontSize: 14, letterSpacing: 2, color: '#6B7280', lineHeight: 1 }}>〔单位或栏目名〕</Text>
    <Text style={{ position: 'absolute', right: 70, top: 678, fontFamily: 'Courier New', fontSize: 14, color: '#6B7280', lineHeight: 1 }}>〔NN / 总页〕</Text>
</Slide>
```

封面外壳（光晕、粒子等装饰照 `slides/01_cover.slide` 写在顶行之前；这里只列与内页对齐的固定件）：

```jsx
<Slide style={{ width: 1280, height: 720, background: '#E63F26', position: 'relative', padding: '40px 70px 0 70px', fontFamily: 'Microsoft YaHei' }}>
    <Box style={{ position: 'absolute', right: 70, top: 78, width: 220, height: 2, background: '#E19825' }} />
    <Box style={{ position: 'absolute', right: 70, top: 90, width: 120, height: 2, background: '#FFE79B', opacity: 0.8 }} />
    <Box style={{ position: 'absolute', left: 0, top: 660, width: 1280, height: 60, background: '#C42819' }} />
    <Box style={{ position: 'absolute', left: 0, top: 658, width: 1280, height: 2, background: '#E19825' }} />
    <Text style={{ position: 'absolute', right: 70, top: 678, fontFamily: 'Courier New', fontSize: 14, color: '#FFE79B', lineHeight: 1 }}>〔01 / 总页〕</Text>

    <Box style={{ position: 'relative', flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center', height: 30 }}>
        <Text style={{ fontFamily: 'Courier New', fontSize: 16, fontWeight: 700, letterSpacing: 2, color: '#FFE79B' }}>01</Text>
        <Text style={{ fontSize: 14, letterSpacing: 2, color: '#FFE79B' }}>〔单位或栏目名〕</Text>
    </Box>

    <Box style={{ position: 'relative', marginTop: 74, width: 1140, height: 320 }}>
    </Box>
</Slide>
```

| 区 | 页型 | x | y | w | h |
|---|---|---|---|---|---|
| 页面 padding | 全部 | 左右 70 | 上 40 / 下 0 | — | — |
| A 页眉区 | 内容页 | 70 | 40 | 1140 | 112 |
| A 内 · 页眉细线条 | 内容页 / 目录页 | 70 | 40 | 1140 | 30 |
| A 内 · 标题行 | 内容页 | 70 | 92 | 1140 | 60 |
| B 内容区 | 内容页 | 70 | 176 | 1140 | 452 |
| 标题块 | 目录页 | 70 | 94 | 1140 | 116 |
| 目录区 | 目录页 | 70 | 230 | 1140 | 410 |
| C 三段装饰条 | 内容页 / 目录页 | 0 | 660 | 1280 | 9 |
| C 单位名 / 页码 | 内容页 / 目录页 | left 70 / right 70 | 678 | — | 14 |
| 顶行 | 封面 | 70 | 40 | 1140 | 30 |
| 标题区（两行各 160） | 封面 | 70 | 144 | 1140 | 320 |
| 飘带 / 胶囊 | 封面 | 70 / 92 | 494 / 570 | — | 56 / 40 |
| 金线 + 深朱红底带 | 封面 | 0 | 658 / 660 | 1280 | 2 / 60 |
| 页码 | 封面 | right 70 | 678 | — | 14 |

纵向算账：
- 内容页：40（padding-top）+ 112（A）+ 24（gap）+ 452（B）+ 32（gap）+ 32（C：装饰条 9 + 间隔 9 + 文字 14）+ 28（底部留白）= 720。
- 目录页：40 + 30（页眉细线条）+ 24 + 116（标题块）+ 20 + 410（目录区）+ 20 + 32（C）+ 28 = 720。
- 封面：40 + 30（顶行）+ 74 + 320（标题区）+ 30 + 56（飘带）+ 20 + 40（胶囊）+ 48 + 62（金线 2 + 底带 60）= 720。

所有内容页必须从此外壳复制，只替换占位文字和 B 区内部；不得改动外壳里的任何数值。B 区放不下就删内容或拆页，不得压缩页眉页脚或改 padding。目录页、封面同理：只替换占位文字和空壳 Box 的内部。

**封面标题容量（硬上限）**：132px 金渐变标题，标题区 1140×320：最多 2 行（短标题只写一行，不得为凑两行硬拆；空出的那一行保留原有占位高度，其余元素位置不变），每行一个 Text，每行最多 8 个汉字；放不下就改写标题，不得加第三行或缩小字号到 100px 以下。标题与页脚、底带之间必须留出空隙，任何情况下标题不得压到页脚上。

**装饰与文字不重叠（硬约束）**：封面金飘带、胶囊、金粒子与标题的相对位置照 `slides/01_cover.slide`，标题区 1140×320 之外不放文字；底部三段装饰条 y 660～669 之上 20px 内不放文字。装饰性 SVG / Box 的矩形与任何文字容器的矩形不得相交；页型没有范例时，装饰只照本套封面或内容页范例的位置放，不另加。

### 图位（配图规则）
- 图位（唯一，正方形）：x 70 / y 178 / 396×396，左，上方 2px 红线、下方一行图注；右侧 720 宽红头白卡。范例见 `slides/04_image.slide`；封面、章节页及其他内容页不放图。
- 没有生图工具，不放 AI 配图；图位只放合格的用户材料图（门槛见 README §5）。`<Image>` 只写相等的 width / height，不写 objectFit；图不加描边、相框、包边线、圆角、阴影；图注一行放图下方。原因与材料图写法见 README §5。
- 有合格材料图时图文页最多 2～4 张、不连续；没有就不用图文页，改用本套其他版式，不留空图位。

## 5. 版式清单
| 文件 | 类型 | 骨架 |
|---|---|---|
| `slides/01_cover.slide` | cover | 朱红满版 + 左侧正方形径向光晕；132px 两行金渐变巨型标题撑满 1140 宽；金色水平飘带副标（主带 + 深金尾翼）；两枚金边红胶囊（宣讲单位 / 时间）；右下空白区金色粒子，右上两条金细线；底部 y 658 金线 + y 660–720 深朱红带，带内右侧浅金页码 |
| `slides/02_section.slide` | section（目录） | 白底；页眉线；CONTENTS 红 kicker + 36×3 金短条 + 红竖条 + 48px 主标，右侧金方点 + 金胶囊；2px 深朱红顶线下 4 行「PART 0N 红胶囊 ｜ 40px 章节名 ｜ 金短条 + 一句说明 ｜ 红色页号」，行间 1px 灰线；三段装饰条 + 页码 |
| `slides/03_cards.slide` | cards（三栏要点） | 白底；固定骨架 A（页眉线 PART 胶囊 + 红竖条 + 42px 结论句）；B 区内3 张红头白卡：56px 红卡头「/ 0N + 名称」→ 22px 红色小标 → 18px 正文 ≥100 字 → 1px 线 + 金短竖条要点钉底；三段装饰条 + 页码 |
| `slides/04_image.slide` | image（图文内容页） | 白底；固定骨架 A；B 区左图右文：左栏 2px 深朱红顶线 + 704×396 的 16:9 纪实照片 + 金短条引出的 14px 图注；右栏 412×452 红头白卡：56px 红卡头「/ 0N + 名称」→ 22px 红色小标 → 18px 正文 → 1px 线 + 3 条金短竖条要点；三段装饰条 + 页码 |

## 6. 标志性处理
1. 巨型陈述标题：封面 132px 两行撑满版面，内页主标一律写成结论句、900 字重、左带红短竖条。
2. 封面四件：金渐变大字（`backgroundClip: 'text'` + `textShadow`）、径向光晕 Box、金色水平飘带、金边红胶囊；另加金色粒子与两条金细线。
3. PART 编号只用深朱红胶囊一种载体（Arial bold 反白）；卡片编号沿用模板的「/ 0N」斜杠标记，放在红卡头内用浅金。
4. 内页底部固定红橙三段装饰条（约 53 / 26 / 21，不等宽）加右下页码。
5. 红头白卡：56px 深朱红卡头 + 白底正文 + 卡尾金色短竖条要点；目录行内用 28×4 金短条引出说明。
6. 层级仍靠 1px 细线与字重字号，一页只有一个 display 时刻。

## 7. Do / Don't
Do
- 封面标题每行 ≤8 字，每行单独一个 Text，外包 Box 高度取字号 ×1.2。
- 径向光晕 Box 用正方形，保证渐变在盒内衰减到透明，不露方形边。
- 装饰 SVG（粒子）只放在无文字无卡片的空白区，绝对定位元素写在内容之前。
- 目录用 CONTENTS + PART 0N，章节名中文；页眉页脚位置页间不漂移。
- 正文用 `#1A1616`，次级说明 `#4B5563`，小字 ≥14px。
- 配图只用 `<Image>` 放进 §4「图位」表里的固定图位，上压 2px 深朱红顶线，图注放在图下方。
Don't
- 不在内页铺红底或通栏红块，红只进卡头、胶囊、竖条、顶线和装饰条。
- 不把金用于正文文字或大面积色块；金渐变大字只在封面与章节页出现。
- 不旋转任何元素，不放印章、红绸、整屏暗蒙版。
- 不在封面、目录、章节页放图；不在图上叠文字、色块或渐变；不给图加圆角、阴影；不用卡通、插画、3D 配图。
- 除 PART / CONTENTS / KPI 与数字单位外页面不出现英文；不写口号、人名、机构名。
- 不用清单外字体，含中文的 Text 必须写 Microsoft YaHei。
