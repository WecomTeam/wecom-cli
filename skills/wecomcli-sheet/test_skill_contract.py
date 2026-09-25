from __future__ import annotations

import unittest
from pathlib import Path


SKILL_ROOT = Path(__file__).resolve().parent


class SheetVerificationContractTest(unittest.TestCase):
    def test_create_flow_requires_complete_remote_read_back(self) -> None:
        skill = (SKILL_ROOT / "SKILL.md").read_text(encoding="utf-8")

        self.assertIn("远端读回基线", skill)
        self.assertIn("task_status", skill)
        self.assertIn("不能据此宣称表格内容完整", skill)
        self.assertIn("第一条数据", skill)
        self.assertIn("最后一条数据", skill)
        self.assertIn("远端读回与本地基线完全一致", skill)

    def test_create_flow_records_a_complete_local_baseline(self) -> None:
        skill = (SKILL_ROOT / "SKILL.md").read_text(encoding="utf-8")

        self.assertIn("每个工作表的有效区域", skill)
        self.assertIn("行列数", skill)
        self.assertIn("非空单元格矩阵", skill)
        self.assertIn("作为远端读回基线", skill)

    def test_create_flow_reads_every_remote_sheet_and_its_full_range(self) -> None:
        skill = (SKILL_ROOT / "SKILL.md").read_text(encoding="utf-8")

        self.assertIn("`wecom-cli sheet get`", skill)
        self.assertIn("`sheet_id` / `data_range`", skill)
        self.assertIn("读取每个工作表的完整有效区域", skill)
        self.assertIn("references/sheet-ranges-get.md", skill)

    def test_create_flow_compares_dimensions_matrix_and_boundary_rows(self) -> None:
        skill = (SKILL_ROOT / "SKILL.md").read_text(encoding="utf-8")

        self.assertIn("逐表比较远端与第 2 步基线的行列数和非空单元格矩阵", skill)
        self.assertIn("表头后的第一条数据", skill)
        self.assertIn("最后一条数据", skill)
        self.assertIn("少行、少列、单元格错位或内容不一致", skill)

    def test_create_flow_keeps_header_and_first_business_row_distinct(self) -> None:
        skill = (SKILL_ROOT / "SKILL.md").read_text(encoding="utf-8")

        self.assertIn("表头是本地工作表的第一行", skill)
        self.assertIn("第一条业务数据是第二行", skill)
        self.assertIn("必须保留两者", skill)
        self.assertIn("禁止把第一条业务数据误当作表头辅助数据而跳过", skill)

    def test_update_flow_keeps_zero_based_rows_and_verifies_every_cell(self) -> None:
        reference = (SKILL_ROOT / "references/sheet-contents-update.md").read_text(
            encoding="utf-8"
        )

        self.assertIn("`start_row` / `start_column` 均从 0 起", reference)
        self.assertIn("禁止因目标区域含表头而跳过 `rows[0]`", reference)
        self.assertIn("读取刚写入的完整目标区域", reference)
        self.assertIn("逐行逐列比较单元格值与预期", reference)


if __name__ == "__main__":
    unittest.main()
