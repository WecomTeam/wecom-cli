from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from io import StringIO
from pathlib import Path

from replace_text import main, replace_unique


class ReplaceUniqueTest(unittest.TestCase):
    def test_replaces_only_the_unique_target_and_preserves_neighbors(self) -> None:
        content = "# 一、背景\n保留前文 😀\n\n旧段落\\n字面量\n\n# 三、结论\n保留后文\n"
        old = "旧段落\\n字面量"
        new = "新段落\n包含真实换行"

        replaced, index, prefix, suffix = replace_unique(content, old, new)

        self.assertEqual(index, content.index(old))
        self.assertEqual(replaced, prefix + new + suffix)
        self.assertEqual(prefix, "# 一、背景\n保留前文 😀\n\n")
        self.assertEqual(suffix, "\n\n# 三、结论\n保留后文\n")
        self.assertNotIn(old, replaced)

    def test_rejects_a_missing_target(self) -> None:
        with self.assertRaisesRegex(ValueError, "实际出现 0 次"):
            replace_unique("正文", "不存在", "新内容")

    def test_rejects_an_ambiguous_target(self) -> None:
        with self.assertRaisesRegex(ValueError, "实际出现 2 次"):
            replace_unique("重复\n重复", "重复", "新内容")

    def test_rejects_overlapping_targets_as_ambiguous(self) -> None:
        with self.assertRaisesRegex(ValueError, "实际出现 3 次"):
            replace_unique("aaaaa", "aaa", "新内容")

    def test_rejects_an_empty_target(self) -> None:
        with self.assertRaisesRegex(ValueError, "旧片段不能为空"):
            replace_unique("正文", "", "新内容")

    def test_cli_does_not_create_output_when_target_is_ambiguous(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            input_path = root / "before.md"
            old_path = root / "old.md"
            new_path = root / "new.md"
            output_path = root / "after.md"
            input_path.write_text("重复\n重复", encoding="utf-8")
            old_path.write_text("重复", encoding="utf-8")
            new_path.write_text("新内容", encoding="utf-8")

            with redirect_stderr(StringIO()):
                exit_code = main(
                    [
                        "--input",
                        str(input_path),
                        "--old",
                        str(old_path),
                        "--new",
                        str(new_path),
                        "--output",
                        str(output_path),
                    ]
                )

            self.assertEqual(exit_code, 2)
            self.assertFalse(output_path.exists())

    def test_cli_does_not_overwrite_an_existing_output(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            input_path = root / "before.md"
            old_path = root / "old.md"
            new_path = root / "new.md"
            output_path = root / "after.md"
            input_path.write_text("旧内容", encoding="utf-8")
            old_path.write_text("旧内容", encoding="utf-8")
            new_path.write_text("新内容", encoding="utf-8")
            output_path.write_text("不得覆盖", encoding="utf-8")

            with redirect_stderr(StringIO()):
                exit_code = main(
                    [
                        "--input",
                        str(input_path),
                        "--old",
                        str(old_path),
                        "--new",
                        str(new_path),
                        "--output",
                        str(output_path),
                    ]
                )

            self.assertEqual(exit_code, 2)
            self.assertEqual(output_path.read_text(encoding="utf-8"), "不得覆盖")

    def test_cli_rejects_using_the_input_as_output(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            input_path = root / "before.md"
            old_path = root / "old.md"
            new_path = root / "new.md"
            input_path.write_text("旧内容", encoding="utf-8")
            old_path.write_text("旧内容", encoding="utf-8")
            new_path.write_text("新内容", encoding="utf-8")

            with redirect_stderr(StringIO()):
                exit_code = main(
                    [
                        "--input",
                        str(input_path),
                        "--old",
                        str(old_path),
                        "--new",
                        str(new_path),
                        "--output",
                        str(input_path),
                    ]
                )

            self.assertEqual(exit_code, 2)
            self.assertEqual(input_path.read_text(encoding="utf-8"), "旧内容")

    def test_cli_writes_replaced_content_and_hash_metadata(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            input_path = root / "before.md"
            old_path = root / "old.md"
            new_path = root / "new.md"
            output_path = root / "after.md"
            content = "前文\n旧内容\n后文"
            expected = "前文\n新内容\n后文"
            input_path.write_text(content, encoding="utf-8")
            old_path.write_text("旧内容", encoding="utf-8")
            new_path.write_text("新内容", encoding="utf-8")
            stdout = StringIO()

            with redirect_stdout(stdout):
                exit_code = main(
                    [
                        "--input",
                        str(input_path),
                        "--old",
                        str(old_path),
                        "--new",
                        str(new_path),
                        "--output",
                        str(output_path),
                    ]
                )

            metadata = json.loads(stdout.getvalue())
            self.assertEqual(exit_code, 0)
            self.assertEqual(output_path.read_text(encoding="utf-8"), expected)
            self.assertEqual(metadata["replacement_index"], content.index("旧内容"))
            self.assertEqual(
                metadata["input_sha256"],
                hashlib.sha256(content.encode("utf-8")).hexdigest(),
            )
            self.assertEqual(
                metadata["output_sha256"],
                hashlib.sha256(expected.encode("utf-8")).hexdigest(),
            )

    def test_cli_accepts_an_empty_new_file_to_delete_the_target(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            input_path = root / "before.md"
            old_path = root / "old.md"
            new_path = root / "new.md"
            output_path = root / "after.md"
            input_path.write_text("保留前文\n删除此段\n保留后文", encoding="utf-8")
            old_path.write_text("删除此段\n", encoding="utf-8")
            new_path.write_text("", encoding="utf-8")

            with redirect_stdout(StringIO()):
                exit_code = main(
                    [
                        "--input",
                        str(input_path),
                        "--old",
                        str(old_path),
                        "--new",
                        str(new_path),
                        "--output",
                        str(output_path),
                    ]
                )

            self.assertEqual(exit_code, 0)
            self.assertEqual(output_path.read_text(encoding="utf-8"), "保留前文\n保留后文")

    def test_cli_does_not_leave_output_for_invalid_utf8(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            input_path = root / "before.md"
            old_path = root / "old.md"
            new_path = root / "new.md"
            output_path = root / "after.md"
            input_path.write_bytes(b"\xff\xfe")
            old_path.write_text("旧内容", encoding="utf-8")
            new_path.write_text("新内容", encoding="utf-8")

            with redirect_stderr(StringIO()):
                exit_code = main(
                    [
                        "--input",
                        str(input_path),
                        "--old",
                        str(old_path),
                        "--new",
                        str(new_path),
                        "--output",
                        str(output_path),
                    ]
                )

            self.assertEqual(exit_code, 2)
            self.assertFalse(output_path.exists())

    def test_cli_does_not_leave_output_when_an_input_file_is_missing(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            old_path = root / "old.md"
            new_path = root / "new.md"
            output_path = root / "after.md"
            old_path.write_text("旧内容", encoding="utf-8")
            new_path.write_text("新内容", encoding="utf-8")

            with redirect_stderr(StringIO()):
                exit_code = main(
                    [
                        "--input",
                        str(root / "missing.md"),
                        "--old",
                        str(old_path),
                        "--new",
                        str(new_path),
                        "--output",
                        str(output_path),
                    ]
                )

            self.assertEqual(exit_code, 2)
            self.assertFalse(output_path.exists())


if __name__ == "__main__":
    unittest.main()
