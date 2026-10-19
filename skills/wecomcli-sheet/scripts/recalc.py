#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""recalc.py — Excel 公式重算器。

由 xlsx_recalc wheel（src/xlsx_recalc 的 _recalc.py + _soffice.py）还原而来，
两个模块内联为单文件脚本，恢复 CLI 入口，agent 直接以子进程方式调用：

    python3 recalc.py <file.xlsx> [timeout_seconds] [--skip-external-link-check]

基于 LibreOffice 宏（RecalculateAndSave: calculateAll → store → close）显式重算
xlsx 全部公式并写入缓存值，**原地覆盖文件**；随后扫描全部单元格的 Excel 错误
（#REF!/#DIV/0!/#VALUE!/#NAME?/#NULL!/#NUM!/#N/A），按错误类型给出计数与位置。

内置沙箱适配层：AF_UNIX socket 被封禁时自动编译并挂 LD_PRELOAD C shim
（lo_socket_shim.so），让 LibreOffice UNO socket 在受限沙箱里跑通。需要 gcc
（仅首次编译 shim；正常环境 socket 没被封则不需要 gcc）。

前置条件：
  - LibreOffice (soffice) 已安装且在 PATH。
  - openpyxl 已安装（沙箱默认装）。
  - 文件必须可写（宏 store() 原地覆盖）。agent 应先把 openpyxl 产出保存到
    沙箱可写目录（output/ 或 temp/），再调本脚本。
  - 默认开外链闸门：检测到 externalLinks 直接返回 error，防止 store()
    静默剥缓存值致公式变 #NAME?。确认无外链或已拷值时传
    --skip-external-link-check 跳过。

输出 JSON 到 stdout；含 "error" 键时退出码 1，否则 0。
"""

import argparse
import json
import os
import platform
import socket
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

from openpyxl import load_workbook

MACRO_DIR_MACOS = "~/Library/Application Support/LibreOffice/4/user/basic/Standard"
MACRO_DIR_LINUX = "~/.config/libreoffice/4/user/basic/Standard"
MACRO_FILENAME = "Module1.xba"

# LibreOffice 宏:强制重算全部公式 → 原地保存 → 关闭。
# 比 soffice --convert-to xlsx 的"加载时重算"更显式可靠。
RECALCULATE_MACRO = """<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE script:module PUBLIC "-//OpenOffice.org//DTD OfficeDocument 1.0//EN" "module.dtd">
<script:module xmlns:script="http://openoffice.org/2000/script" script:name="Module1" script:language="StarBasic">
    Sub RecalculateAndSave()
      ThisComponent.calculateAll()
      ThisComponent.store()
      ThisComponent.close(True)
    End Sub
</script:module>"""

EXCEL_ERRORS = [
    "#VALUE!", "#DIV/0!", "#REF!", "#NAME?", "#NULL!", "#NUM!", "#N/A",
]


# --------------------------------------------------------------------------- #
# soffice 沙箱适配层(原 _soffice.py)
# --------------------------------------------------------------------------- #

_SHIM_SO = Path(tempfile.gettempdir()) / "lo_socket_shim.so"


def get_soffice_env() -> dict:
    """返回跑 soffice 所需的 env dict。

    - 固定设 SAL_USE_VCLPLUGIN=svp(无头渲染)。
    - 若 AF_UNIX socket 被封禁(沙箱),编译并挂 lo_socket_shim.so 到 LD_PRELOAD。
    """
    env = os.environ.copy()
    env["SAL_USE_VCLPLUGIN"] = "svp"

    if _needs_shim():
        shim = _ensure_shim()
        env["LD_PRELOAD"] = str(shim)

    return env


def _needs_shim() -> bool:
    """AF_UNIX socket 能创建就不需要 shim。"""
    try:
        s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        s.close()
        return False
    except OSError:
        return True


def _ensure_shim() -> Path:
    """编译(仅首次)并返回 shim .so 路径。需要 gcc。"""
    if _SHIM_SO.exists():
        return _SHIM_SO

    src = Path(tempfile.gettempdir()) / "lo_socket_shim.c"
    src.write_text(_SHIM_SOURCE)
    subprocess.run(
        ["gcc", "-shared", "-fPIC", "-o", str(_SHIM_SO), str(src), "-ldl"],
        check=True,
        capture_output=True,
    )
    src.unlink()
    return _SHIM_SO


# LD_PRELOAD 挂的这个 C shim 把 LibreOffice 内部的 socket(AF_UNIX) 调用
# 重定向到 socketpair+pipe,绕开沙箱对 AF_UNIX 的封禁。仅 FD<1024 生效。
# 源自 xlsx 插件 office/soffice.py,逻辑未改动。
_SHIM_SOURCE = r"""
#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/socket.h>
#include <unistd.h>

static int (*real_socket)(int, int, int);
static int (*real_socketpair)(int, int, int, int[2]);
static int (*real_listen)(int, int);
static int (*real_accept)(int, struct sockaddr *, socklen_t *);
static int (*real_close)(int);
static int (*real_read)(int, void *, size_t);

/* Per-FD bookkeeping (FDs >= 1024 are passed through unshimmed). */
static int is_shimmed[1024];
static int peer_of[1024];
static int wake_r[1024];            /* accept() blocks reading this */
static int wake_w[1024];            /* close()  writes to this      */
static int listener_fd = -1;        /* FD that received listen()    */

__attribute__((constructor))
static void init(void) {
    real_socket     = dlsym(RTLD_NEXT, "socket");
    real_socketpair = dlsym(RTLD_NEXT, "socketpair");
    real_listen     = dlsym(RTLD_NEXT, "listen");
    real_accept     = dlsym(RTLD_NEXT, "accept");
    real_close      = dlsym(RTLD_NEXT, "close");
    real_read       = dlsym(RTLD_NEXT, "read");
    for (int i = 0; i < 1024; i++) {
        peer_of[i] = -1;
        wake_r[i]  = -1;
        wake_w[i]  = -1;
    }
}

/* ---- socket ---------------------------------------------------------- */
int socket(int domain, int type, int protocol) {
    if (domain == AF_UNIX) {
        int fd = real_socket(domain, type, protocol);
        if (fd >= 0) return fd;
        /* socket(AF_UNIX) blocked - fall back to socketpair(). */
        int sv[2];
        if (real_socketpair(domain, type, protocol, sv) == 0) {
            if (sv[0] >= 0 && sv[0] < 1024) {
                is_shimmed[sv[0]] = 1;
                peer_of[sv[0]]    = sv[1];
                int wp[2];
                if (pipe(wp) == 0) {
                    wake_r[sv[0]] = wp[0];
                    wake_w[sv[0]] = wp[1];
                }
            }
            return sv[0];
        }
        errno = EPERM;
        return -1;
    }
    return real_socket(domain, type, protocol);
}

/* ---- listen ---------------------------------------------------------- */
int listen(int sockfd, int backlog) {
    if (sockfd >= 0 && sockfd < 1024 && is_shimmed[sockfd]) {
        listener_fd = sockfd;
        return 0;
    }
    return real_listen(sockfd, backlog);
}

/* ---- accept ---------------------------------------------------------- */
int accept(int sockfd, struct sockaddr *addr, socklen_t *addrlen) {
    if (sockfd >= 0 && sockfd < 1024 && is_shimmed[sockfd]) {
        /* Block until close() writes to the wake pipe. */
        if (wake_r[sockfd] >= 0) {
            char buf;
            real_read(wake_r[sockfd], &buf, 1);
        }
        errno = ECONNABORTED;
        return -1;
    }
    return real_accept(sockfd, addr, addrlen);
}

/* ---- close ----------------------------------------------------------- */
int close(int fd) {
    if (fd >= 0 && fd < 1024 && is_shimmed[fd]) {
        int was_listener = (fd == listener_fd);
        is_shimmed[fd] = 0;

        if (wake_w[fd] >= 0) {              /* unblock accept() */
            char c = 0;
            write(wake_w[fd], &c, 1);
            real_close(wake_w[fd]);
            wake_w[fd] = -1;
        }
        if (wake_r[fd] >= 0) { real_close(wake_r[fd]); wake_r[fd]  = -1; }
        if (peer_of[fd] >= 0) { real_close(peer_of[fd]); peer_of[fd] = -1; }

        if (was_listener)
            _exit(0);                        /* conversion done - exit */
    }
    return real_close(fd);
}
"""


# --------------------------------------------------------------------------- #
# 重算核心(原 _recalc.py)
# --------------------------------------------------------------------------- #

def has_gtimeout() -> bool:
    try:
        subprocess.run(
            ["gtimeout", "--version"], capture_output=True, timeout=1, check=False
        )
        return True
    except (FileNotFoundError, subprocess.TimeoutExpired):
        return False


def setup_libreoffice_macro() -> bool:
    """确保 LibreOffice 用户配置里有 RecalculateAndSave 宏。"""
    macro_dir = os.path.expanduser(
        MACRO_DIR_MACOS if platform.system() == "Darwin" else MACRO_DIR_LINUX
    )
    macro_file = os.path.join(macro_dir, MACRO_FILENAME)

    if os.path.exists(macro_file) and "RecalculateAndSave" in Path(macro_file).read_text():
        return True

    if not os.path.exists(macro_dir):
        # 首次跑 soffice 让它生成用户配置目录树
        subprocess.run(
            ["soffice", "--headless", "--terminate_after_init"],
            capture_output=True, timeout=10, check=False, env=get_soffice_env(),
        )
        os.makedirs(macro_dir, exist_ok=True)

    try:
        Path(macro_file).write_text(RECALCULATE_MACRO)
        return True
    except Exception:
        return False


def _has_external_links(path: str) -> bool:
    """检测 xlsx 内部是否存在外链(externalLinks)部件。"""
    try:
        with zipfile.ZipFile(path) as z:
            return any("externallink" in n.lower() for n in z.namelist())
    except Exception:
        return False


def recalc(filename, timeout=30, check_external_links=True):
    """用 LibreOffice 宏重算 xlsx 全部公式,原地保存并扫描错误。

    参数:
        filename (str): xlsx 文件路径(必须可写——宏会 store() 原地覆盖)。
        timeout (int): soffice 超时秒数,默认 30。
        check_external_links (bool): 默认 True。检测到外链直接返回 error,
            防止 LibreOffice store() 静默剥掉外链缓存值致公式变 #NAME?。
            确认无外链依赖或已手动拷出值时可传 False 跳过。

    返回 dict:
        成功无错:  {"status":"success","total_errors":0,"total_formulas":N}
        成功有错:  {"status":"errors_found","total_errors":N,"total_formulas":N,
                    "error_summary":{"#REF!":{"count":2,"locations":[...]}, ...}}
        失败:      {"error":"..."}  (文件不存在/soffice 缺失/宏配置失败/超时等)
    """
    if not Path(filename).exists():
        return {"error": f"File {filename} does not exist"}

    abs_path = str(Path(filename).absolute())

    # 外链前置闸门(默认开)。='[1]xxx'!$B$2 这类外链,store() 会剥缓存值。
    if check_external_links and _has_external_links(abs_path):
        return {"error": "检测到外链(externalLinks)；先把外链单元格值拷出再重算"}

    if not setup_libreoffice_macro():
        return {"error": "Failed to setup LibreOffice macro"}

    cmd = [
        "soffice",
        "--headless",
        "--norestore",
        "vnd.sun.star.script:Standard.Module1.RecalculateAndSave?language=Basic&location=application",
        abs_path,
    ]

    if platform.system() == "Linux":
        cmd = ["timeout", str(timeout)] + cmd
    elif platform.system() == "Darwin" and has_gtimeout():
        cmd = ["gtimeout", str(timeout)] + cmd

    try:
        result = subprocess.run(cmd, capture_output=True, text=True, env=get_soffice_env())
    except FileNotFoundError:
        return {"error": "soffice 未安装或不在 PATH；请先安装 LibreOffice"}

    # returncode 124 = timeout 超时退出码(Linux timeout / macOS gtimeout)
    if result.returncode != 0 and result.returncode != 124:
        error_msg = result.stderr or "Unknown error during recalculation"
        if "Module1" in error_msg or "RecalculateAndSave" not in error_msg:
            return {"error": "LibreOffice macro not configured properly"}
        return {"error": error_msg}

    try:
        wb = load_workbook(filename, data_only=True)

        error_details = {err: [] for err in EXCEL_ERRORS}
        total_errors = 0

        for sheet_name in wb.sheetnames:
            ws = wb[sheet_name]
            for row in ws.iter_rows():
                for cell in row:
                    if cell.value is not None and isinstance(cell.value, str):
                        for err in EXCEL_ERRORS:
                            if err in cell.value:
                                error_details[err].append(
                                    f"{sheet_name}!{cell.coordinate}"
                                )
                                total_errors += 1
                                break

        wb.close()

        out = {
            "status": "success" if total_errors == 0 else "errors_found",
            "total_errors": total_errors,
            "error_summary": {},
        }

        for err_type, locations in error_details.items():
            if locations:
                out["error_summary"][err_type] = {
                    "count": len(locations),
                    "locations": locations[:20],  # 每类最多 20 个位置
                }

        wb_formulas = load_workbook(filename, data_only=False)
        formula_count = 0
        for sheet_name in wb_formulas.sheetnames:
            ws = wb_formulas[sheet_name]
            for row in ws.iter_rows():
                for cell in row:
                    if (cell.value and isinstance(cell.value, str)
                            and cell.value.startswith("=")):
                        formula_count += 1
        wb_formulas.close()

        out["total_formulas"] = formula_count

        return out

    except Exception as e:
        return {"error": str(e)}


# --------------------------------------------------------------------------- #
# CLI
# --------------------------------------------------------------------------- #

def main():
    parser = argparse.ArgumentParser(
        description="用 LibreOffice 宏重算 xlsx 全部公式,原地保存并扫描 Excel 错误。"
                    "输出 JSON 到 stdout。")
    parser.add_argument("file", help="xlsx 文件路径(必须可写,宏 store() 原地覆盖)")
    parser.add_argument("timeout", nargs="?", type=int, default=30,
                        help="soffice 超时秒数(默认 30)")
    parser.add_argument("--skip-external-link-check", action="store_true",
                        help="跳过外链(externalLinks)前置闸门。"
                             "仅在确认无外链依赖或已手动拷出值时使用")
    args = parser.parse_args()

    result = recalc(args.file, timeout=args.timeout,
                    check_external_links=not args.skip_external_link_check)
    print(json.dumps(result, indent=2, ensure_ascii=False))
    sys.exit(1 if "error" in result else 0)


if __name__ == "__main__":
    main()
