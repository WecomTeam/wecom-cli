#!/usr/bin/env python3
"""在完整 UTF-8 文本中安全替换唯一片段。"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path


def replace_unique(content: str, old: str, new: str) -> tuple[str, int, str, str]:
    """替换唯一旧片段，并返回新文本、位置及未改动的前后文。"""
    if not old:
        raise ValueError("旧片段不能为空")

    positions: list[int] = []
    search_from = 0
    while (position := content.find(old, search_from)) != -1:
        positions.append(position)
        search_from = position + 1

    occurrences = len(positions)
    if occurrences != 1:
        raise ValueError(f"旧片段必须恰好出现一次，实际出现 {occurrences} 次")

    index = positions[0]
    prefix = content[:index]
    suffix = content[index + len(old) :]
    return prefix + new + suffix, index, prefix, suffix


def sha256_text(value: str) -> str:
    return hashlib.sha256(value.encode("utf-8")).hexdigest()


def read_utf8(path: Path) -> str:
    return path.read_bytes().decode("utf-8")


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="在完整 UTF-8 文本中替换唯一片段")
    parser.add_argument("--input", required=True, type=Path, help="替换前的完整正文")
    parser.add_argument("--old", required=True, type=Path, help="待替换的唯一旧片段")
    parser.add_argument("--new", required=True, type=Path, help="替换后的新片段，可为空文件")
    parser.add_argument("--output", required=True, type=Path, help="替换后的完整正文")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)

    try:
        if args.input.resolve() == args.output.resolve():
            raise ValueError("输出文件不能覆盖输入基线")
        if args.output.exists():
            raise ValueError("输出文件已存在，请使用新的输出路径")

        content = read_utf8(args.input)
        old = read_utf8(args.old)
        new = read_utf8(args.new)
        replaced, index, prefix, suffix = replace_unique(content, old, new)

        with args.output.open("xb") as output_file:
            output_file.write(replaced.encode("utf-8"))
        print(
            json.dumps(
                {
                    "replacement_index": index,
                    "input_sha256": sha256_text(content),
                    "output_sha256": sha256_text(replaced),
                    "prefix_sha256": sha256_text(prefix),
                    "suffix_sha256": sha256_text(suffix),
                },
                ensure_ascii=False,
            )
        )
        return 0
    except (OSError, UnicodeError, ValueError) as error:
        print(f"替换失败：{error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
