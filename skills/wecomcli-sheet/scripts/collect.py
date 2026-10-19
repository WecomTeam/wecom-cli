#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""collect.py — 表格事实采集器（只测量、不判断）。

由 wecom_sheet_collector wheel（src/wecom_sheet_collector）还原而来：
__init__.py 的 collect() 与 _core.py 的全部采集逻辑内联为单文件脚本，
恢复 CLI 入口（argparse），agent 直接以子进程方式调用：

    python3 collect.py <file.xlsx>                 # 采集所有子表，紧凑 JSON 打 stdout
    python3 collect.py <file.xlsx> --sheet 汇总    # 只采指定子表
    python3 collect.py <file.xlsx> --range A1:D20 [--sheet 汇总]   # 定向采区域
    python3 collect.py <file.xlsx> --column B [--sheet 汇总]       # 定向采整列

设计原则（石蕊测试）：
  允许报告："B列第5行以下97%是数值"、"A4:A5是合并区"、"第15行含 =SUM(B7:B14)"
  禁止结论："所以表头在第5行"、"第15行是小计"、"这是二维表"
  —— 任何带"所以/因此/这是"的判断都留给模型，脚本不碰。

支持 .xlsx/.xlsm（全事实）；.csv/.tsv（无合并/样式，给类型剖面）；.xls 提示先转换。
出错打印 {"error": "..."} 到 stdout 并以退出码 1 结束（正常为 0）。
依赖：openpyxl（沙箱默认预装）。
"""

import argparse
import csv
import json
import os
import sys
from collections import Counter
from contextlib import contextmanager
from datetime import datetime, date, time
from pathlib import Path
from threading import RLock

MAX_COLS_DISPLAY = 60          # 列剖面/快照「数据行取值」最多展示列数(表头列名不受此限,始终给全)
CELL_VALUE_TRUNC = 60          # 单元格值展示截断
NUM_DECIMALS = 4               # 快照里浮点数展示的最大小数位(探查阶段不需要全精度,降噪省体积)
MAX_CSV_BYTES = 200 * 1024 * 1024  # CSV 全量读入内存上限(200 MiB),防超大文件 OOM 拖垮同机其他 skill
MERGE_DISPLAY_CAP = 50         # 合并区展示上限,超出只给前 N 个 + total;全量用 --range 定向采时带 merge 状态
IMAGE_DETAIL_CAP = 10          # 浮动图片详情展示上限,超出只给前 N 张 + total;编辑需 offset_emu 等细粒度字段时用 --range 定向采
ROW_RUNS_CAP = 15              # 行游程段数展示上限,超出只给前 N 段 + total_runs;后段用 --range 看实际值

# openpyxl 的 Fill.from_tree 是进程级类方法。当前脚本通常以独立子进程运行，
# 但仍用锁保护，避免未来被同一进程多线程调用时发生临时补丁交叉覆盖。
_OPENPYXL_LOAD_LOCK = RLock()


# --------------------------------------------------------------------------
# openpyxl 空 Fill 兼容加载（只影响解析过程，不修改原文件）
# --------------------------------------------------------------------------
def _is_empty_fill_error(exc):
    """判断异常链是否为 openpyxl 解析空 <fill/> 时产生的已知错误。"""
    messages = []
    visited = set()
    current = exc
    while current is not None and id(current) not in visited:
        visited.add(id(current))
        messages.append(str(current))
        current = current.__cause__ or current.__context__
    combined = "\n".join(messages)
    return ("openpyxl.styles.fills.Fill" in combined
            and "Fill() takes no arguments" in combined)


@contextmanager
def _tolerate_empty_fills():
    """临时将空 <fill/> 解释为无填充；退出时恢复 openpyxl 原始解析器。"""
    from openpyxl.styles import PatternFill
    from openpyxl.styles.fills import Fill

    original_descriptor = Fill.__dict__["from_tree"]
    state = {"fill_index": 0}
    normalized_fill_ids = []

    def tolerant_from_tree(cls, element):
        fill_id = state["fill_index"]
        state["fill_index"] += 1
        if len(element) == 0:
            normalized_fill_ids.append(fill_id)
            return PatternFill(patternType="none")
        return original_descriptor.__func__(cls, element)

    Fill.from_tree = classmethod(tolerant_from_tree)
    try:
        yield normalized_fill_ids
    finally:
        Fill.from_tree = original_descriptor


def load_workbook_compat(path, **kwargs):
    """优先标准加载；仅命中空 Fill 错误时临时容错并重试。

    返回 (workbook, warnings)。warnings 只在发生兼容处理时出现，便于模型知道
    源文件存在 OOXML 兼容问题，且本次只在内存解析阶段按无填充处理。
    """
    import openpyxl

    with _OPENPYXL_LOAD_LOCK:
        try:
            return openpyxl.load_workbook(path, **kwargs), []
        except Exception as exc:
            if not _is_empty_fill_error(exc):
                raise

        with _tolerate_empty_fills() as fill_ids:
            workbook = openpyxl.load_workbook(path, **kwargs)

    warnings = [{
        "code": "empty_fill_normalized",
        "fill_ids": list(fill_ids),
        "message": ("检测到 openpyxl 不兼容的空 fill，采集时已临时按无填充处理;"
                    "原始文件未修改"),
    }]
    return workbook, warnings


# 顶层 _fields:子表字段含义说明,只出现一次,不在每个 sheet 里重复
_FIELDS = {
    "title": "当前子表名称,与sheets对象中的键相同",
    "freeze_panes": "冻结窗格起点;例如A2表示第1行被冻结,可作为表头末行的辅助证据,不能单独据此判定表头",
    "top_snapshot": "顶部样本{columns,rows,notes};columns是全部Excel列字母,rows是前N行的全宽值,notes记录前60列中的合并锚点/加粗/数字格式;用于识别标题、表头和首批数据",
    "bottom_snapshot": "底部样本{rows,notes},与顶部样本去重;rows是末N行的全宽值,用于发现合计、备注、来源等尾部内容",
    "column_profile": "各列剖面[{col,type_counts,number_formats}];统计前max_rows行、覆盖全部列,type_counts不计空单元格,用于判断列的数据类型和显示格式",
    "row_runs": "连续相同行结构的压缩段[{start_row,end_row,count,nonempty_cols,dominant_type,has_formula,any_bold}];最多返回前15段,用于定位表头/数据区转换和多个数据块",
    "merged": "合并区域坐标列表,如A1:B2;最多返回前50个,用于还原多级表头和区分合并覆盖格与普通空值",
    "formulas": "公式位置与文本[{cell,formula}];扫描前max_rows行和前60列,最多返回200个,用于定位计算列、合计和小计",
    "blanks": "扫描范围内的全空行列{rows,cols};扫描前max_rows行和前60列,用于发现同一子表内不同表格之间的分隔",
    "image_count": "成功识别到的浮动图片数量;提示编辑时注意图片及其锚点可能受影响",
    "charts": "图表列表[{type,type_name,title,anchor,series_count,series:[{title,val_ref,cat_ref}]}];val_ref/cat_ref是图表数值和分类标签的单元格引用,可据此定向追采",
    "trunc": "完整性标记{rows,formulas,merged,row_runs,scan_cols};true表示对应扫描或输出未覆盖全部内容;scan_cols仅表示公式、空行空列和快照notes只检查前60列,顶部/底部值、列剖面和行游程仍覆盖全部列",
}


# --------------------------------------------------------------------------
# 沙箱路径闸门(软失败,与 wecomagent WECOMAGENT_*_DIRS 白名单口径对齐)
# --------------------------------------------------------------------------
def _parse_dirs_env(env_name):
    """解析 WECOMAGENT_*_DIRS,支持 ["/abs", ...] 或 [{"path":"/abs"}, ...];
    缺失/非法一律返回空列表(-> 视为未配置,由调用方决定是否放行)。"""
    raw = os.environ.get(env_name, "")
    if not raw:
        return []
    try:
        items = json.loads(raw)
    except (json.JSONDecodeError, TypeError):
        return []
    if not isinstance(items, list):
        return []
    roots = []
    for d in items:
        p = None
        if isinstance(d, dict) and isinstance(d.get("path"), str) and d["path"]:
            p = d["path"]
        elif isinstance(d, str) and d:
            p = d
        if p:
            roots.append(os.path.realpath(p))
    return roots


def ensure_readable(path_str):
    """读路径闸门(软失败):字面校验(空字节/相对路径/".."段) + realpath 前缀归属。

    与 deep-research/utils.py 口径一致:仅当 WECOMAGENT_READABLE_DIRS /
    WECOMAGENT_WRITABLE_DIRS 至少配置了一个时才启用白名单归属校验;两者都
    未配置时按"非沙箱/开发环境"放行,不误伤本地调用(纵深防御,不替代上层沙箱)。
    校验失败返回错误信息字符串;通过返回 None。"""
    if not isinstance(path_str, str) or not path_str:
        return "路径不能为空"
    if "\x00" in path_str:
        return "路径包含非法空字节"
    # ".." 段必须在 realpath 折叠前检查(realpath 会把 /a/b/../c 静默折叠为 /a/c)
    if any(seg == ".." for seg in path_str.replace("\\", "/").split("/")):
        return "路径不能包含 '..' 段"

    readable = _parse_dirs_env("WECOMAGENT_READABLE_DIRS")
    writable = _parse_dirs_env("WECOMAGENT_WRITABLE_DIRS")
    roots = readable + writable
    if not roots:
        return None  # 未配置白名单 -> 放行(依赖上层沙箱兜底)

    real = os.path.realpath(path_str)
    for root in roots:
        try:
            if os.path.commonpath([real, root]) == root:
                return None
        except ValueError:
            continue  # 跨盘符/相对绝对混合
    return f"路径不在可读白名单内(解析为 {real})"


# --------------------------------------------------------------------------
# 类型分类(纯测量)
# --------------------------------------------------------------------------
def classify(cell):
    v = cell.value
    if v is None or v == "":
        return "empty"
    dt = getattr(cell, "data_type", None)
    if dt == "f" or (isinstance(v, str) and v.startswith("=")):
        return "formula"
    if dt == "e":
        return "error"
    if isinstance(v, bool):            # 必须在 int 之前判断
        return "bool"
    if isinstance(v, (datetime, date, time)):
        return "date"
    if isinstance(v, (int, float)):
        return "number"
    if isinstance(v, str):
        return "text"
    return "other"


def cell_is_bold(cell):
    try:
        return bool(cell.font and cell.font.bold)
    except Exception:
        return False


def trunc(v):
    s = str(v)
    return s if len(s) <= CELL_VALUE_TRUNC else s[:CELL_VALUE_TRUNC] + "…"


def emit_value(v):
    """值输出统一入口:截断时回带 truncated + full_len 信号,让消费方区分'天然以…结尾'与'被截断'。"""
    s = "" if v is None else str(v)
    if len(s) <= CELL_VALUE_TRUNC:
        return {"value": s or None}
    return {"value": s[:CELL_VALUE_TRUNC] + "…", "truncated": True, "full_len": len(s)}


def cell_display(v):
    """head 形态里单个数据格的展示值(标量,非 dict):
    - None/空 -> None
    - float -> 降到 NUM_DECIMALS 位小数(去掉全精度长串,探查不需要),整数值不带小数
    - 其它 -> 字符串并做长度截断(超长补 …)
    """
    if v is None or v == "":
        return None
    if isinstance(v, bool):
        return v
    if isinstance(v, float):
        if v == int(v):
            return int(v)
        return round(v, NUM_DECIMALS)
    if isinstance(v, int):
        return v
    s = str(v)
    return s if len(s) <= CELL_VALUE_TRUNC else s[:CELL_VALUE_TRUNC] + "…"


def col_letter(idx):
    from openpyxl.utils import get_column_letter
    return get_column_letter(idx)


# --------------------------------------------------------------------------
# 合并区索引(事实)
# --------------------------------------------------------------------------
def build_merge_index(ws):
    from openpyxl.utils import range_boundaries
    anchors = {}    # (r,c) -> {"span_rows":.., "span_cols":..}
    covered = set() # 被合并覆盖的非锚点单元格
    ranges = []
    for rng in ws.merged_cells.ranges:
        c1, r1, c2, r2 = range_boundaries(str(rng))
        ranges.append(str(rng))
        anchors[(r1, c1)] = {"span_rows": r2 - r1 + 1, "span_cols": c2 - c1 + 1}
        for r in range(r1, r2 + 1):
            for c in range(c1, c2 + 1):
                if (r, c) != (r1, c1):
                    covered.add((r, c))
    return ranges, anchors, covered


def merge_status(r, c, anchors, covered):
    if (r, c) in anchors:
        a = anchors[(r, c)]
        return {"role": "anchor", "span_rows": a["span_rows"], "span_cols": a["span_cols"]}
    if (r, c) in covered:
        return {"role": "covered"}
    return None


# --------------------------------------------------------------------------
# 图表元信息采集(事实)—— 不只判断有无,采出类型/标题/锚点/系列引用区域
# --------------------------------------------------------------------------
_CHART_TYPE_NAME = {
    "BarChart": "柱形/条形图", "BarChart3D": "3D柱形/条形图",
    "LineChart": "折线图", "LineChart3D": "3D折线图",
    "PieChart": "饼图", "PieChart3D": "3D饼图",
    "AreaChart": "面积图", "AreaChart3D": "3D面积图",
    "ScatterChart": "散点图", "RadarChart": "雷达图",
    "DoughnutChart": "圆环图", "BubbleChart": "气泡图",
    "SurfaceChart": "曲面图", "SurfaceChart3D": "3D曲面图",
}


def _chart_title(ch):
    """尽量稳健地取出图表标题文本(标题可能是 str / 富文本 / 引用)。"""
    t = ch.title
    if t is None:
        return None
    if isinstance(t, str):
        return t
    tx = getattr(t, "tx", None)
    rich = getattr(tx, "rich", None) if tx else None
    if not rich:
        return None
    parts = []
    for p in (rich.p or []):
        for r in (p.r or []):
            if getattr(r, "t", None):
                parts.append(r.t)
    return "".join(parts) if parts else None


def _ref_of(ds):
    """从 NumDataSource / AxDataSource 取引用区域 f,如 'Sheet1!$B$2:$B$10'。"""
    if ds is None:
        return None
    for attr in ("numRef", "strRef"):
        ref = getattr(ds, attr, None)
        if ref is not None and getattr(ref, "f", None):
            return ref.f
    return None


def _series_info(s):
    title = None
    if s.title is not None:
        sl = s.title
        if getattr(sl, "strRef", None) and sl.strRef.f:
            title = sl.strRef.f            # 引用式标题
        elif getattr(sl, "v", None):
            title = sl.v                    # 字面量标题
    return {
        "title": title,
        "val_ref": _ref_of(s.val),          # 数值区域
        "cat_ref": _ref_of(s.cat),          # 分类标签区域
    }


def collect_charts(ws):
    out = []
    for ch in getattr(ws, "_charts", []) or []:
        cls = type(ch).__name__
        anchor = None
        a = getattr(ch, "anchor", None)
        frm = getattr(a, "_from", None) if a else None
        if frm is not None:                 # openpyxl anchor 的 col/row 是 0-based
            anchor = f"{col_letter(frm.col + 1)}{frm.row + 1}"
        out.append({
            "type": cls,
            "type_name": _CHART_TYPE_NAME.get(cls, cls),
            "title": _chart_title(ch),
            "anchor": anchor,
            "series_count": len(ch.series) if ch.series else 0,
            "series": [_series_info(s) for s in (ch.series or [])],
        })
    return out


# --------------------------------------------------------------------------
# 浮动图片事实采集(事实)—— 不只判断有无,采出数量/锚点/位置/尺寸/格式
# --------------------------------------------------------------------------
def collect_images(ws):
    """浮动图片元数据事实(不碰二进制,只测定位信息)。

    图片不在单元格内(iter_rows/values_only 读不到),靠锚点定位,是编辑时
    最容易整体丢失的隐性数据。单张图解析失败只跳过并记 skipped,不拖垮采集。

    探查阶段只保留核心字段(cell/width_px/height_px/format),超出 IMAGE_DETAIL_CAP
    张时只给前 N 张详情 + total;编辑需 offset_emu 等细粒度字段时用 --range 定向采。
    """
    out = []
    skipped = []
    for idx, img in enumerate(getattr(ws, "_images", []) or []):
        info = {"index": idx}
        try:
            a = img.anchor
            frm = getattr(a, "_from", None)
            if frm is None:
                skipped.append({"index": idx, "reason": "无锚点(_from 缺失)"})
                continue
            kind = type(a).__name__          # OneCellAnchor / TwoCellAnchor
            info["anchor_type"] = kind
            info["cell"] = f"{col_letter(frm.col + 1)}{frm.row + 1}"
            info["width_px"] = getattr(img, "width", None)
            info["height_px"] = getattr(img, "height", None)
            fmt = getattr(img, "format", None)
            info["format"] = fmt if isinstance(fmt, str) else (
                str(fmt) if fmt is not None else None)
            # 探查阶段不输出 offset_emu / to_cell 等编辑定位字段;超出 cap 的图也不展开
            out.append(info)
        except Exception as e:
            skipped.append({"index": idx, "reason": f"{type(e).__name__}: {e}"})
    total = len(out)
    truncated = total > IMAGE_DETAIL_CAP
    shown = out[:IMAGE_DETAIL_CAP]
    result = {"count": total, "images": shown}
    if truncated:
        result["images_truncated"] = True
        result["images_shown"] = len(shown)
        result["images_total"] = total
    if skipped:
        result["skipped"] = skipped
    return result


# --------------------------------------------------------------------------
# 逐行精细快照(事实)—— top / bottom 共用
# 形态:类似 pandas df.head() 的二维表(columns + rows),而非逐格 {col,type,value}
# 对象。关键:columns 给「全部列名」始终保真、绝不因 MAX_COLS_DISPLAY 截断
# (年份/字段名是结构核心,这里截断会让模型看不到尾部列 -> 曾致范围误判)。
# 数据行的取值列数仍受 ncols 限,超出以 cols_truncated 标注。
# --------------------------------------------------------------------------
def _row_values(ws, r, ncols):
    """取第 r 行前 ncols 列的展示值(标量数组,None 表示空格)。"""
    return [cell_display(ws.cell(r, c).value) for c in range(1, ncols + 1)]


def _all_column_letters(ws):
    """全部列字母(A..最后一列),不受 MAX_COLS_DISPLAY 限制 —— 表头保真的关键。"""
    return [col_letter(c) for c in range(1, ws.max_column + 1)]


def _struct_notes(ws, r_start, r_end, anchors, covered, ncols):
    """稀疏结构线索:合并锚点 / 加粗格 / 非常规数字格式,只在存在时逐条给,
    不再逐格铺满。格式如 {"cell":"A4","merge":{...}} / {"cell":"B5","bold":true}。"""
    notes = []
    for r in range(r_start, r_end + 1):
        for c in range(1, ncols + 1):
            cell = ws.cell(r, c)
            ref = f"{col_letter(c)}{r}"
            ms = merge_status(r, c, anchors, covered)
            if ms and ms.get("role") == "anchor":
                notes.append({"cell": ref, "merge": ms})
            nf = getattr(cell, "number_format", None)
            if nf and nf != "General" and classify(cell) != "empty":
                notes.append({"cell": ref, "number_format": nf})
            if cell_is_bold(cell) and classify(cell) != "empty":
                notes.append({"cell": ref, "bold": True})
    return notes


def top_snapshot(ws, anchors, covered, top_rows, ncols):
    """顶部 head:columns 给全部列名(保真),rows 给前 top_rows 行的值。
    取值列用全宽(ws.max_column):前几行含表头文字,是结构核心,绝不截列
    (只有 top_rows 行,全宽取值成本极小);notes 扫描列仍用 ncols 控成本。"""
    snap_cols = ws.max_column
    last = min(ws.max_row, top_rows)
    rows = [{"row": r, "values": _row_values(ws, r, snap_cols)} for r in range(1, last + 1)]
    return {
        "columns": _all_column_letters(ws),
        "rows": rows,
        "notes": _struct_notes(ws, 1, last, anchors, covered, ncols),
    }


def bottom_snapshot(ws, anchors, covered, top_rows, bottom_rows, ncols):
    """底部 head:采最后 bottom_rows 行,与 top 去重(start=max(top_rows+1,max_row-bottom+1))。
    取值列同 top 用全宽;文件太短(已被 top 覆盖)时 rows 为空。columns 不重复给。"""
    snap_cols = ws.max_column
    max_row = ws.max_row
    start = max(top_rows + 1, max_row - bottom_rows + 1)
    if start > max_row:
        return {"rows": [], "notes": []}
    rows = [{"row": r, "values": _row_values(ws, r, snap_cols)} for r in range(start, max_row + 1)]
    return {
        "rows": rows,
        "notes": _struct_notes(ws, start, max_row, anchors, covered, ncols),
    }


# --------------------------------------------------------------------------
# 全列类型剖面 + 逐行签名(游程压缩)(事实)
# --------------------------------------------------------------------------
def scan_profile(ws, ncols, max_rows):
    scan_to = min(ws.max_row, max_rows)
    truncated = ws.max_row > scan_to

    col_types = {c: Counter() for c in range(1, ncols + 1)}
    col_formats = {c: set() for c in range(1, ncols + 1)}

    row_runs = []      # 游程压缩后的行签名
    prev_key = None
    run = None

    for r in range(1, scan_to + 1):
        nonempty_cols = []
        types = Counter()
        has_formula = False
        any_bold = False
        for c in range(1, ncols + 1):
            cell = ws.cell(r, c)
            t = classify(cell)
            if t != "empty":
                nonempty_cols.append(c)
                types[t] += 1
                col_types[c][t] += 1
                nf = getattr(cell, "number_format", None)
                if nf and nf != "General":
                    col_formats[c].add(nf)
                if t == "formula":
                    has_formula = True
            if cell_is_bold(cell):
                any_bold = True
        dom = types.most_common(1)[0][0] if types else "empty"
        key = (tuple(nonempty_cols), dom, has_formula, any_bold)
        if key == prev_key:
            run["end_row"] = r
            run["count"] += 1
        else:
            if run:
                row_runs.append(run)
            run = {
                "start_row": r, "end_row": r, "count": 1,
                "nonempty_cols": [col_letter(c) for c in nonempty_cols],
                "nonempty_count": len(nonempty_cols),
                "dominant_type": dom,
                "has_formula": has_formula,
                "any_bold": any_bold,
            }
            prev_key = key
    if run:
        row_runs.append(run)

    col_profile = []
    for c in range(1, ncols + 1):
        counts = dict(col_types[c])
        col_profile.append({
            "col": col_letter(c),
            "type_counts": counts,
            "number_formats": sorted(col_formats[c])[:5],
        })
    return col_profile, row_runs, scan_to, truncated


# --------------------------------------------------------------------------
# 公式清单(事实)
# --------------------------------------------------------------------------
def collect_formulas(ws, ncols, max_rows, cap=200):
    out = []
    for r in range(1, min(ws.max_row, max_rows) + 1):
        for c in range(1, ncols + 1):
            cell = ws.cell(r, c)
            v = cell.value
            if isinstance(v, str) and v.startswith("="):
                entry = {"cell": f"{col_letter(c)}{r}"}
                entry.update(emit_value(v))
                # 公式文本若被截断,补一个 untruncated_formula 仅供定位(不带则不出现)
                entry["formula"] = entry.pop("value")
                out.append(entry)
                if len(out) >= cap:
                    return out, True
    return out, False


# --------------------------------------------------------------------------
# 空行/空列图(事实)—— 多表分隔线索
# --------------------------------------------------------------------------
def blank_map(ws, ncols, max_rows):
    scan_to = min(ws.max_row, max_rows)
    blank_rows = []
    for r in range(1, scan_to + 1):
        if all(ws.cell(r, c).value in (None, "") for c in range(1, ncols + 1)):
            blank_rows.append(r)
    blank_cols = []
    for c in range(1, ncols + 1):
        if all(ws.cell(r, c).value in (None, "") for r in range(1, scan_to + 1)):
            blank_cols.append(col_letter(c))
    return blank_rows, blank_cols


# --------------------------------------------------------------------------
# 单个 sheet 的完整事实
# --------------------------------------------------------------------------
def collect_sheet(ws, top_rows, max_rows, bottom_rows=5):
    ncols = min(ws.max_column, MAX_COLS_DISPLAY)   # 快照数据行/游程/公式的取值列上限
    cols_truncated = ws.max_column > ncols
    # 列类型剖面单独放开到全部列:每列只是一个小对象,成本可控,且能覆盖
    # 尾部列(如靠后的年份列),避免模型漏采表尾字段。
    prof_cols = ws.max_column
    ranges, anchors, covered = build_merge_index(ws)
    col_profile, row_runs, scanned, row_trunc = scan_profile(ws, prof_cols, max_rows)
    formulas, fx_trunc = collect_formulas(ws, ncols, max_rows)
    blank_rows, blank_cols = blank_map(ws, ncols, max_rows)

    charts = collect_charts(ws)
    images = collect_images(ws)

    # 合并区:精简成 A1:B1 字符串列表,超 cap 截断
    merge_truncated = len(ranges) > MERGE_DISPLAY_CAP
    shown_merged = ranges[:MERGE_DISPLAY_CAP]

    # 行游程:超 cap 只给前 N 段
    runs_total = len(row_runs)
    runs_truncated = runs_total > ROW_RUNS_CAP
    shown_runs = row_runs[:ROW_RUNS_CAP]

    return {
        "title": ws.title,
        "freeze_panes": ws.freeze_panes,
        "top_snapshot": top_snapshot(ws, anchors, covered, top_rows, ncols),
        "bottom_snapshot": bottom_snapshot(ws, anchors, covered, top_rows, bottom_rows, ncols),
        "column_profile": col_profile,
        "row_runs": shown_runs,
        "merged": shown_merged,
        "formulas": formulas,
        "blanks": {"rows": blank_rows, "cols": blank_cols},
        "image_count": images["count"],
        "charts": charts,
        "trunc": {
            "rows": row_trunc,
            "formulas": fx_trunc,
            "merged": merge_truncated,
            "row_runs": runs_truncated,
            "scan_cols": cols_truncated,
        },
    }


# --------------------------------------------------------------------------
# 定向采集
# --------------------------------------------------------------------------
def collect_range(ws, rng_str):
    from openpyxl.utils import range_boundaries
    c1, r1, c2, r2 = range_boundaries(rng_str)
    _, anchors, covered = build_merge_index(ws)
    out = []
    for r in range(r1, r2 + 1):
        row = []
        for c in range(c1, c2 + 1):
            cell = ws.cell(r, c)
            t = classify(cell)
            e = {"ref": f"{col_letter(c)}{r}", "type": t}
            if t != "empty":
                e.update(emit_value(cell.value))
            ms = merge_status(r, c, anchors, covered)
            if ms:
                e["merge"] = ms
            nf = getattr(cell, "number_format", None)
            if nf and nf != "General":
                e["number_format"] = nf
            row.append(e)
        out.append(row)
    return {"range": rng_str, "cells": out}


def collect_column(ws, letter, max_rows):
    from openpyxl.utils import column_index_from_string
    c = column_index_from_string(letter.upper())
    scan_to = min(ws.max_row, max_rows)
    vals = []
    for r in range(1, scan_to + 1):
        cell = ws.cell(r, c)
        t = classify(cell)
        entry = {"row": r, "type": t}
        if t != "empty":
            entry.update(emit_value(cell.value))
        vals.append(entry)
    return {"column": letter.upper(), "values": vals,
            "scanned_rows": scan_to, "truncated": ws.max_row > scan_to}


# --------------------------------------------------------------------------
# CSV 轻量路径(无合并/样式)
# --------------------------------------------------------------------------
def collect_csv(path, sep, top_rows, max_rows, bottom_rows=5):
    # 全量 list(csv.reader) 会把整文件读进内存,超大文件可能 OOM;读前先卡文件大小
    try:
        size = os.path.getsize(path)
    except OSError as e:
        return {"error": f"无法读取 CSV 文件属性: {type(e).__name__}"}
    if size > MAX_CSV_BYTES:
        return {"error": f"CSV 文件过大({size}字节 > 上限{MAX_CSV_BYTES}字节),"
                         f"请先拆分或用 --range/--column 定向采集"}

    encs = ["utf-8-sig", "gbk", "gb18030", "utf-8", "latin-1"]
    rows = None
    used_enc = None
    for enc in encs:
        try:
            with open(path, newline="", encoding=enc) as f:
                rows = list(csv.reader(f, delimiter=sep))
            used_enc = enc
            break
        except (UnicodeDecodeError, UnicodeError):
            # 只吞编码类异常继续换下一种编码;PermissionError/OSError/MemoryError
            # 等真实错误不吞,交由上层 except 报出准确类型,便于定位
            continue
    if rows is None:
        return {"error": f"无法解码 CSV: {path}"}

    def cls(s):
        if s is None or s == "":
            return "empty"
        try:
            float(s.replace(",", ""))
            return "number"
        except ValueError:
            return "text"

    def csv_cell(s):
        """CSV 单格展示值:数值转 number 降精度,其余截断字符串,空 -> None。"""
        if s is None or s == "":
            return None
        if cls(s) == "number":
            try:
                f = float(s.replace(",", ""))
                return int(f) if f == int(f) else round(f, NUM_DECIMALS)
            except ValueError:
                pass
        return s if len(s) <= CELL_VALUE_TRUNC else s[:CELL_VALUE_TRUNC] + "…"

    def row_values(i):
        return {"row": i + 1,
                "values": [csv_cell(rows[i][j]) if j < len(rows[i]) else None
                           for j in range(full_width)]}

    full_width = max((len(r) for r in rows[:max_rows]), default=0)   # 真实全宽
    ncols = min(full_width, MAX_COLS_DISPLAY)                        # 列类型剖面列上限
    all_cols = [col_letter(j + 1) for j in range(full_width)]        # 全部列字母,不截断
    top = [row_values(i) for i in range(min(len(rows), top_rows))]
    total = len(rows)
    start_i = max(top_rows, total - bottom_rows)  # 0-based, 已含 top 覆盖区间
    bottom = [row_values(i) for i in range(start_i, total)] if start_i < total else []
    col_types = [{"col": col_letter(j + 1),
                  "type_counts": dict(Counter(cls(r[j] if j < len(r) else "") for r in rows[:max_rows]))}
                 for j in range(ncols)]
    return {
        "note": "CSV/TSV 无合并单元格与样式信息,仅提供值类型剖面。",
        "encoding": used_enc,
        "dimensions": {"rows": len(rows), "cols": full_width},
        "top_snapshot": {"columns": all_cols, "rows": top},
        "bottom_snapshot": {"rows": bottom},
        "column_profile": col_types,
        "merged": [],
        "trunc": {"column_profile_cols": full_width > ncols},
    }


# --------------------------------------------------------------------------
# 主采集入口(对应原 wheel 的 collect())
# --------------------------------------------------------------------------
def collect(path, sheet=None, rng=None, column=None, top_rows=5, bottom_rows=5, max_rows=20000):
    """采集表格事实(只测量、不判断),返回 dict。

    参数:
        path (str): 表格文件路径。支持 .xlsx/.xlsm/.csv/.tsv。
        sheet (str|None): 只采指定子表;None 采所有子表。
        rng (str|None): 定向采区域,如 "A1:D20"(需 xlsx/xlsm,作用于 sheet 或首个子表)。
        column (str|None): 定向采整列,如 "B"(同上)。
        top_rows/bottom_rows/max_rows: 快照行数与扫描行上限。

    返回 dict;出错返回 {"error": "..."},不抛异常。
    """
    path_str = str(path)

    # 沙箱路径闸门(软失败)
    path_err = ensure_readable(path_str)
    if path_err:
        return {"error": path_err}

    p = Path(path_str)
    if not p.exists():
        return {"error": f"文件不存在: {path_str}"}

    suffix = p.suffix.lower()
    try:
        if suffix in (".csv", ".tsv"):
            if rng or column or sheet:
                return {"error": "CSV/TSV 不支持 --sheet/--range/--column 定向采集"}
            sep = "\t" if suffix == ".tsv" else ","
            csv_result = collect_csv(str(p), sep, top_rows, max_rows, bottom_rows)
            return {"file": path_str, **csv_result}

        if suffix == ".xls":
            return {"error": ".xls 为老二进制格式,openpyxl 无法读取合并/样式。请先把 .xls 转成 .xlsx 再采集。"}

        if suffix not in (".xlsx", ".xlsm"):
            return {"error": f"不支持的格式: {suffix}"}

        wb, load_warnings = load_workbook_compat(
            str(p), data_only=False, keep_vba=(suffix == ".xlsm"))

        # 定向采集:--range / --column(作用于 --sheet 指定子表,缺省首个子表)
        if rng or column:
            target_name = sheet or wb.sheetnames[0]
            if target_name not in wb.sheetnames:
                return {"error": f"子表不存在: {target_name}(可选: {wb.sheetnames})"}
            ws = wb[target_name]
            result = {"file": path_str, "sheet": target_name}
            if load_warnings:
                result["_warnings"] = load_warnings
            if rng:
                result.update(collect_range(ws, rng))
            else:
                result.update(collect_column(ws, column, max_rows))
            return result

        # 整体采集:--sheet 只采一个子表,缺省采所有子表
        if sheet:
            if sheet not in wb.sheetnames:
                return {"error": f"子表不存在: {sheet}(可选: {wb.sheetnames})"}
            names = [sheet]
        else:
            names = wb.sheetnames

        sheets_meta = [{"name": n,
                        "max_row": wb[n].max_row, "max_col": wb[n].max_column}
                       for n in names]
        detail = {n: collect_sheet(wb[n], top_rows, max_rows, bottom_rows) for n in names}

        result = {
            "_fields": _FIELDS,
            "file": path_str,
            "sheet_names": wb.sheetnames,
            "sheets_overview": sheets_meta,
            "sheets": detail,
            "_note": "以上均为测量事实。表头位置/小计判定/表型推断请由模型完成;"
                     "图表的 series[].val_ref/cat_ref 是追采真实数值的入口;"
        }
        if load_warnings:
            result["_warnings"] = load_warnings
        return result

    except Exception as e:
        return {"error": f"{type(e).__name__}: {e}"}


# --------------------------------------------------------------------------
# CLI
# --------------------------------------------------------------------------
def main():
    parser = argparse.ArgumentParser(
        description="表格事实采集器(只测量、不判断),紧凑 JSON 输出到 stdout。")
    parser.add_argument("file", help="表格文件路径(.xlsx/.xlsm/.csv/.tsv)")
    parser.add_argument("--sheet", help="只采指定子表(缺省采所有子表)")
    parser.add_argument("--range", dest="range",
                        help="定向采区域,如 A1:D20(配合 --sheet,缺省首个子表)")
    parser.add_argument("--column", help="定向采整列,如 B(配合 --sheet,缺省首个子表)")
    parser.add_argument("--top", type=int, default=5, help="顶部快照行数(默认 5)")
    parser.add_argument("--bottom", type=int, default=5, help="底部快照行数(默认 5)")
    parser.add_argument("--max-rows", type=int, default=20000,
                        help="行扫描上限(默认 20000)")
    parser.add_argument("--pretty", action="store_true",
                        help="缩进美化输出(默认紧凑单行,省 token)")
    args = parser.parse_args()

    result = collect(args.file, sheet=args.sheet, rng=args.range,
                     column=args.column, top_rows=args.top,
                     bottom_rows=args.bottom, max_rows=args.max_rows)

    if args.pretty:
        print(json.dumps(result, ensure_ascii=False, indent=2, default=str))
    else:
        print(json.dumps(result, ensure_ascii=False, separators=(",", ":"), default=str))

    sys.exit(1 if "error" in result else 0)


if __name__ == "__main__":
    main()
