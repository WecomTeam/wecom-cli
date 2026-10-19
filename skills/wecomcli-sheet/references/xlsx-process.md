# 本地表格创建、编辑与分析

用户可能要求创建、编辑或分析 `.xlsx`、`.xlsm`、`.xls`、`.csv`、`.tsv` 文件。本流程依赖 `openpyxl`、`pandas`、`LibreOffice` 等环境组件，使用前须按「环境依赖检查」按需检测是否已安装，未安装时先安装再继续。

## 环境依赖检查

| 依赖 | 何时需要 |
|---|---|
| `openpyxl` | 编辑 xlsx、运行 `recalc.py` |
| `pandas` | 数据分析 |
| LibreOffice（`soffice`） | 运行 `recalc.py` 重算公式 |

- 按需检测与安装：只在即将用到某个依赖时检测它；纯分析只查 `pandas`，编辑文件查 `openpyxl`，需要重算公式时再查 `soffice`。不用到的依赖不检测、不安装。

## 执行前置门禁（处理已有表格时必须完成）

本节适用于已经存在的 `.xlsx`、`.xlsm`、`.xls`、`.csv`、`.tsv` 文件。

> **硬规则**：必须依次完成“来源规范化 → 结构探查 → 选择处理库”。在结构探查成功前，禁止使用 pandas、openpyxl 或其他方式读取业务数据、分析内容或修改文件。

### 得到统一工作文件 `working_path`

先按来源完成必要的读取副本或格式转换，并得到一个可供探查脚本读取的绝对路径 `working_path`。

| 来源 | 必须执行的操作 |
|---|---|
| 本地 `.xlsx` / `.xlsm` | 将原文件绝对路径记为 `working_path`；后续用 openpyxl 编辑 `.xlsm` 时须设置 `keep_vba=True` |
| 本地 `.xls` | 先用 `wecom-cli parser office convert` 转为 `.xlsx`，将转换结果的绝对路径记为 `working_path` |
| 本地 `.csv` / `.tsv` | 将原文件绝对路径记为 `working_path`；只读分析可用 pandas，编辑时必须保持用户要求的分隔符、编码和输出格式，不得无提示转为 `.xlsx` |

#### `.xls` 格式转换

- 判断“源文件是否已是用户要求的格式”时，优先结合用户可见文件名/附件展示名；若已一致，不调用转换工具，只简短回复“你发送的文件已经是 xls 了，无需进行转换。”，不要追加后台预转换、可转其他格式等说明。
- 若附件展示名/用户引用的文件名是 `.xls`，但本地处理路径是 `.xlsx`，这是后台预转换工作副本，仅用于内部处理；对用户格式判断仍按展示名，禁止说用户上传的是 `.xlsx`，也禁止据此回答 `.xlsx` 转 `.xls` 不支持。
- 严格按用户明确提出的目标格式处理，不要揣测或改写用户意图；意图不明确时先问清楚。若用户明确要求当前链路不支持的转换，直接说明不支持，不要自动改为 PDF或其他格式。

```bash
wecom-cli parser office convert --json '<JSON 参数>'
```

参数

| 字段          | 类型 | 必填 | 语义                         |
|-------------|---|---|----------------------------|
| `file_path` | string | 是 | 待转换文件的完整路径                 |
| `target_file_type`   | string | 是 | 目标文件格式，可选 `xlsx` 和 `xls` |

### 强制执行结构探查

取得 `working_path` 后，必须立即运行探查脚本：

```bash
python3 "{baseDir}/scripts/collect.py" "<working_path>"
```

- `{baseDir}` 表示当前 Skill 所在目录（`wecomcli-sheet`）。执行时必须替换为实际绝对路径。
- 脚本已经存在，直接运行；禁止先做脚本存在性检测。
- 脚本支持 `.xlsx`、`.xlsm`、`.csv`、`.tsv`；`.xls` 必须先转换为 `.xlsx`
- 此步骤是结构采样，不等于读取全部业务数据。后续正式读取仍须限定在用户要求的子表和范围内。

### 根据探查结果选择处理库

- 仅做数据分析且不生成交付文件：可以使用 pandas。
- 编辑数据、公式、格式、合并单元格，或生成需要保留原工作簿对象的交付文件：使用 openpyxl。
- 任一子表 `image_count > 0`：必须使用 openpyxl，并显式制定浮动图片的保留、删除或移动方案。

## 读取与分析数据

### 用 pandas 做数据分析

数据分析、可视化和基础操作优先用 **pandas**，它提供了完整的数据处理能力：

```python
import pandas as pd

# 读取 Excel
df = pd.read_excel('file.xlsx')                           # 默认读第一个子表
all_sheets = pd.read_excel('file.xlsx', sheet_name=None)  # 读全部子表，返回 dict

# 分析
df.head()      # 预览数据
df.info()      # 列信息
df.describe()  # 描述统计

# 写出 Excel
df.to_excel('output.xlsx', index=False)
```

## Excel 文件工作流

### 日期规范化

按日期筛选、排序、分组或比较后，交付列必须显示为可读日期，不能保留 Excel 序列号（如 `46327`）

```python
from datetime import datetime, timedelta
c.value = datetime(1899, 12, 30) + timedelta(days=c.value)
c.number_format = "yyyy/mm/dd"
```

转换值和设置日期格式缺一不可。

### 关键规则：写公式，不写硬编码值

**一律用 Excel 公式，不要在 Python 里算好结果再把数字写进单元格。** 只有写公式，表格才是动态可更新的。

#### 错误做法 —— 硬编码计算结果

```python
# 差：在 Python 里算完把结果写死
total = df['Sales'].sum()
sheet['B10'] = total  # 写死成 5000

# 差：在 Python 里算增长率
growth = (df.iloc[-1]['Revenue'] - df.iloc[0]['Revenue']) / df.iloc[0]['Revenue']
sheet['C5'] = growth  # 写死成 0.15

# 差：在 Python 里算平均值
avg = sum(values) / len(values)
sheet['D20'] = avg  # 写死成 42.5
```

#### 正确做法 —— 使用 Excel 公式

```python
# 好：让 Excel 自己求和
sheet['B10'] = '=SUM(B2:B9)'

# 好：增长率写成 Excel 公式
sheet['C5'] = '=(C4-C2)/C2'

# 好：用 Excel 函数求平均
sheet['D20'] = '=AVERAGE(D2:D19)'
```

这条规则适用于**所有**计算 —— 合计、百分比、比率、差值等等。源数据变化时，表格应当能自行重算。

### 通用流程

1. **选库**：纯数据分析且不生成文件：可使用 pandas。编辑数据、公式/格式，或任一子表 `image_count > 0`：必须使用 openpyxl，并显式处理图片。
2. **新建 / 加载**：新建工作簿或加载既有文件。
3. **修改**：写入或编辑数据、公式与格式；涉及增删行列时，先检查是否有浮动图片，含图片则按「处理浮动图片」规则操作。
4. **保存**：写出文件。
5. **重算公式（写了公式则必做）**：见下方「重算公式」节，使用当前 Skill 自带的 `scripts/recalc.py`。
6. **核查并修复错误**：见下方「解读 `recalc()` 的返回」节，按返回 dict 判断成功 / 有错 / 失败。

### 新建 Excel 文件

```python
# 用 openpyxl 处理公式与格式
from openpyxl import Workbook
from openpyxl.styles import Font, PatternFill, Alignment

wb = Workbook()
sheet = wb.active

# 写入数据
sheet['A1'] = 'Hello'
sheet['B1'] = 'World'
sheet.append(['Row', 'of', 'data'])

# 写入公式
sheet['B2'] = '=SUM(A1:A10)'

# 设置格式
sheet['A1'].font = Font(bold=True, color='FF0000')
sheet['A1'].fill = PatternFill('solid', start_color='FFFF00')
sheet['A1'].alignment = Alignment(horizontal='center')

# 列宽
sheet.column_dimensions['A'].width = 20

wb.save('output.xlsx')
```

### 编辑已有 Excel 文件

涉及到编辑请求时，要求严格遵守用户的要求，不允许自行扩大或者缩小修改内容和范围。

```python
# 用 openpyxl 以保留公式与格式
from openpyxl import load_workbook

# 加载既有文件
wb = load_workbook('existing.xlsx')
sheet = wb.active            # 指定子表用 wb['SheetName']

# 遍历多个子表
for sheet_name in wb.sheetnames:
    sheet = wb[sheet_name]
    print(f"Sheet: {sheet_name}")

# 修改单元格
sheet['A1'] = 'New Value'
sheet.insert_rows(2)   # 在第 2 行插入一行
sheet.delete_cols(3)   # 删除第 3 列

# 新增子表
new_sheet = wb.create_sheet('NewSheet')
new_sheet['A1'] = 'Data'

wb.save('modified.xlsx')
```

## 处理浮动图片

- **图片不属于单元格内容**：`ws.delete_rows()` 只删除和移动单元格，不会自动删除浮动图片，也不会更新图片锚点，必须单独处理；
- **定位图片所在行列**：遍历 `ws._images`，通过 `image.anchor._from.row` / `.col` 获取左上角锚点；锚点使用 **0-based** 编号，换算为 Excel 行列时需要 `+1`；
- **删除行内图片**：图片锚点行落在待删除区间时，从 `ws._images` 中删除。批量删除应先收集索引，再按索引**倒序删除**，避免列表下标变化；
- **移动后续图片**：图片位于待删除区间之后时，将 `anchor._from.row` 减去删除行数；如果是 `TwoCellAnchor`，还要同步修改 `anchor.to.row`；
- **正确操作顺序**：先删除目标行对应的图片并调整后续图片锚点，再调用 `ws.delete_rows()`；
- **锚点类型注意**：`OneCellAnchor` 只需调整 `_from`；`TwoCellAnchor` 需要同时调整 `_from` 和 `to`，右下角属性是 `anchor.to`，不是 `_to`；`AbsoluteAnchor` 无法按单元格行号可靠移动；
- **跨簿复制图片保留 ext**：`add_image(img, "F2")` 传字符串锚点会走 anchor setter 丢弃原 `ext` 并按图片像素重算显示尺寸，应手动构造 `OneCellAnchor(_from=AnchorMarker(col=原列, row=目标行-1), ext=原ext)` 后 `add_image(img)` 不传锚点；

## 重算公式

openpyxl 创建或修改出的 Excel 文件，公式只是字符串，**没有计算值**。用 `recalc.py` 重算（原地覆盖写缓存值）：

```bash
python3 "{baseDir}/scripts/recalc.py" "<xlsx文件绝对路径>"
```

- 其中 `{baseDir}` 表示当前 Skill 所在的目录（wecomcli-sheet）。执行命令时，必须将其替换为实际绝对路径。脚本文件已经存在，直接按路径运行即可，禁止对文件做存在性检测。
- `recalc.py` 会原地覆盖目标工作簿，必须对已保存的待交付副本执行，禁止直接重算唯一原文件；对 `.xlsm` 或包含高级对象的工作簿，重算后还必须复核 VBA 和对象保真。

该脚本会：

- 重算所有子表的全部公式
- 扫描**所有**单元格里的 Excel 错误（`#REF!` / `#DIV/0!` 等）
- 返回 dict，含错误的具体位置与数量

## 公式校验清单

用于快速确认公式是否正确：

### 必查项

- [ ] **抽查 2~3 个引用**：铺开整张表之前，先确认这几个引用取到的值正确
- [ ] **列号映射**：确认 Excel 列号对得上（例如第 64 列是 BL，不是 BK）
- [ ] **行号偏移**：Excel 行号从 1 开始（DataFrame 第 5 行 = Excel 第 6 行）

### 常见坑

- [ ] **空值处理**：用 `pd.notna()` 判断 null
- [ ] **靠右的列**：财年数据常在第 50 列之后
- [ ] **多处匹配**：要搜出所有出现位置，别只取第一个
- [ ] **除零**：公式里用 `/` 之前先检查分母（`#DIV/0!`）
- [ ] **引用错位**：确认每个引用都指向预期单元格（`#REF!`）
- [ ] **跨子表引用**：跨表链接要用正确写法（`Sheet1!A1`）

### 公式测试策略

- [ ] **从小做起**：先在 2~3 个单元格上验证公式，再大面积套用
- [ ] **核对依赖**：确认公式引用到的单元格都存在
- [ ] **测边界值**：覆盖 0、负数和极大值

### 解读 `recalc()` 的返回

函数返回带错误明细的 dict：

```python
# 成功无错
{ "status": "success", "total_errors": 0, "total_formulas": 42, "error_summary": {} }

# 成功有错
{
  "status": "errors_found",
  "total_errors": 3,
  "total_formulas": 42,
  "error_summary": {                 # 仅在发现错误时非空
    "#REF!":   { "count": 2, "locations": ["Sheet1!B5", "Sheet1!C10"] },   # locations 每类最多 20
    "#DIV/0!": { "count": 1, "locations": ["Sheet1!D3"] }
  }
}

# 失败
{ "error": "检测到外链(externalLinks)；先把外链单元格值拷出再重算" }
```

判断写法：

```python
if "error" in result:
    ...  # 失败,暴露给agent
elif result["status"] == "success":
    ...  # 重算成功,无 Excel 错误
else:   # errors_found
    for err_type, info in result["error_summary"].items():
        ...  # info = {"count": N, "locations": [...]}
```

## 最佳实践

### 选库

- **pandas**：适合数据分析、批量操作和简单的数据导出
- **openpyxl**：适合复杂格式、公式以及 Excel 特有能力

### 使用 openpyxl

- 单元格下标从 1 开始（`row=1, column=1` 即 A1）
- 读计算值要用 `data_only=True`：`load_workbook('file.xlsx', data_only=True)`
- **警告**：以 `data_only=True` 打开后再保存，公式会被值替换且**永久丢失**
- 大文件：读用 `read_only=True`，写用 `write_only=True`
- 公式会被保留但不会求值 —— 使用当前 Skill 自带的 `scripts/recalc.py` 重算并检查错误

### 使用 pandas

- 显式指定数据类型，避免类型推断出错：`pd.read_excel('file.xlsx', dtype={'id': str})`
- 大文件只读需要的列：`pd.read_excel('file.xlsx', usecols=['A', 'C', 'E'])`
- 正确处理日期：`pd.read_excel('file.xlsx', parse_dates=['date_column'])`

## 代码风格要求

**重要**：生成 Excel 相关 Python 代码时：

- 代码精简，不写多余注释
- 不用冗长的变量名，不做重复操作
- 不加无必要的 print

**对 Excel 文件本身**：

- 给含复杂公式或重要假设的单元格加批注
- 硬编码值要标注数据来源
