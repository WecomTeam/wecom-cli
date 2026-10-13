from __future__ import annotations

import unittest
from pathlib import Path


SKILL_ROOT = Path(__file__).resolve().parents[1]


class DocReplacementContractTest(unittest.TestCase):
    def test_skill_routes_partial_and_full_replacements_explicitly(self) -> None:
        skill = (SKILL_ROOT / "SKILL.md").read_text(encoding="utf-8")

        self.assertIn("局部替换指定段落或章节", skill)
        self.assertIn("references/doc-contents-replace.md", skill)
        self.assertIn("先确认是局部还是整篇", skill)
        self.assertIn("禁止降级成 `append`", skill)
        self.assertIn("全量覆盖doc文档内容", skill)
        self.assertIn("仅当用户明确要求", skill)
        self.assertIn("references/doc-contents-overwrite.md", skill)

    def test_reference_requires_complete_source_and_rejects_lossy_rich_text(self) -> None:
        reference = (SKILL_ROOT / "references/doc-contents-replace.md").read_text(
            encoding="utf-8"
        )

        self.assertIn("短内容读取返回的 `content`", reference)
        self.assertIn("长内容使用返回的 `file_path`", reference)
        self.assertIn("读取完整文件", reference)
        self.assertIn("无法完整表达文档中的图片、表格或其他结构", reference)
        self.assertIn("不得用纯文本覆盖富文本文档", reference)

    def test_reference_persists_all_replacement_artifacts_and_hashes(self) -> None:
        reference = (SKILL_ROOT / "references/doc-contents-replace.md").read_text(
            encoding="utf-8"
        )

        for artifact in ["`before.md`", "`old.md`", "`new.md`", "`after.md`"]:
            self.assertIn(artifact, reference)
        self.assertIn("`prefix_sha256`", reference)
        self.assertIn("`suffix_sha256`", reference)
        self.assertIn("目标前后的正文未被本地替换过程改动", reference)

    def test_reference_restarts_when_the_prewrite_snapshot_changed(self) -> None:
        reference = (SKILL_ROOT / "references/doc-contents-replace.md").read_text(
            encoding="utf-8"
        )

        self.assertIn("最新完整 Markdown 与 `before.md` 逐字节比较", reference)
        self.assertIn("完全一致：可以继续写入", reference)
        self.assertIn("废弃本次 `after.md`", reference)
        self.assertIn("从第 1 步重新读取和定位", reference)

    def test_reference_never_treats_write_acknowledgement_as_final_success(self) -> None:
        reference = (SKILL_ROOT / "references/doc-contents-replace.md").read_text(
            encoding="utf-8"
        )

        self.assertIn("写接口返回成功只代表请求完成", reference)
        self.assertIn("不能据此向用户宣称段落已替换", reference)

    def test_reference_requires_all_postwrite_checks_and_forbids_patch_up_writes(self) -> None:
        reference = (SKILL_ROOT / "references/doc-contents-replace.md").read_text(
            encoding="utf-8"
        )

        required_checks = [
            "远端完整 Markdown 与 `after.md` 一致",
            "新片段位于原目标位置且只出现预期次数",
            "目标前后的内容分别与 `before.md` 的前缀、后缀一致",
            "旧片段已被替换",
        ]
        for check in required_checks:
            self.assertIn(check, reference)
        self.assertIn("只有四项全部通过时", reference)
        self.assertIn("不得继续追加补写", reference)

    def test_reference_preserves_the_safe_read_replace_verify_sequence(self) -> None:
        reference = (SKILL_ROOT / "references/doc-contents-replace.md").read_text(
            encoding="utf-8"
        )

        required_steps = [
            "### 1. 读取完整正文",
            "### 2. 唯一定位替换范围",
            "### 3. 生成完整新正文",
            "### 4. 写入前再次读取并比对",
            "### 5. 覆盖写入完整新正文",
            "### 6. 写后完整回读",
        ]
        positions = [reference.index(step) for step in required_steps]
        self.assertEqual(positions, sorted(positions))
        self.assertIn("不能只传 `new.md`", reference)
        self.assertIn("高并发编辑中的文档应停止操作", reference)
        self.assertIn("新内容可以为空", reference)


if __name__ == "__main__":
    unittest.main()
