#!/usr/bin/env python3
"""Create 6-grid or 9-grid visual contact sheets from a PPTX.

Pipeline:
1. LibreOffice converts PPTX to PDF.
2. `pypdfium2` (pip 包) renders PDF pages to PNG.
3. Pillow composes every 6 or 9 slides into one contact-sheet PNG.

依赖：LibreOffice（系统二进制，pptx→pdf）+ pypdfium2 / Pillow（pip 包）。

Safety: this script never performs wildcard deletion. It writes into a new
run-specific output directory and overwrites only explicit files inside it.
"""

from __future__ import annotations

import argparse
import datetime as _dt
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Iterable, List, Sequence, Tuple


def _die(message: str, code: int = 1) -> None:
    print(f"[pptx-style] ERROR: {message}", file=sys.stderr)
    raise SystemExit(code)


def _run(cmd: Sequence[str], cwd: Path | None = None) -> None:
    printable = " ".join(str(x) for x in cmd)
    print(f"[pptx-style] RUN: {printable}")
    proc = subprocess.run(
        list(cmd),
        cwd=str(cwd) if cwd else None,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if proc.stdout.strip():
        print(proc.stdout.strip())
    if proc.stderr.strip():
        print(proc.stderr.strip(), file=sys.stderr)
    if proc.returncode != 0:
        _die(f"command failed with exit code {proc.returncode}: {printable}")


def _find_executable(name_or_path: str | None, candidates: Iterable[str]) -> str | None:
    if name_or_path:
        p = Path(name_or_path).expanduser()
        if p.exists():
            return str(p)
        found = shutil.which(name_or_path)
        if found:
            return found
    for item in candidates:
        p = Path(item).expanduser()
        if p.exists():
            return str(p)
        found = shutil.which(item)
        if found:
            return found
    return None


def _natural_key(path: Path) -> Tuple[int, str]:
    match = re.search(r"(\d+)(?=\.png$)", path.name)
    return (int(match.group(1)) if match else 10**9, path.name)


def _import_pillow():
    try:
        from PIL import Image, ImageDraw, ImageFont
    except Exception as exc:  # pragma: no cover - dependency message path
        _die(
            "Pillow is required for contact-sheet composition. "
            "若缺失则拼图不可用，请直接查看逐页 PNG 或走 xml 兜底。\n"
            f"Original import error: {exc}"
        )
    return Image, ImageDraw, ImageFont


def convert_pptx_to_pdf(pptx: Path, pdf_dir: Path, soffice: str) -> Path:
    pdf_dir.mkdir(parents=True, exist_ok=True)
    before = {p.resolve() for p in pdf_dir.glob("*.pdf")}
    _run([soffice, "--headless", "--convert-to", "pdf", "--outdir", str(pdf_dir), str(pptx)])
    after = sorted(pdf_dir.glob("*.pdf"), key=lambda p: p.stat().st_mtime, reverse=True)
    new_files = [p for p in after if p.resolve() not in before]
    if new_files:
        return new_files[0]
    if after:
        return after[0]
    _die("LibreOffice did not produce a PDF file")


def _import_pdfium():
    try:
        import pypdfium2 as pdfium
    except Exception as exc:  # pragma: no cover - dependency message path
        _die(
            "pypdfium2 is required to render PDF pages to PNG. "
            "若缺失则 PDF 渲染不可用，请走 xml 解析兜底。\n"
            f"Original import error: {exc}"
        )
    return pdfium


def render_pdf_to_pngs(pdf: Path, slides_dir: Path, dpi: int) -> List[Path]:
    """用 pypdfium2 把 PDF 各页渲染为 slide-NN.png。"""
    pdfium = _import_pdfium()
    slides_dir.mkdir(parents=True, exist_ok=True)
    scale = dpi / 72.0
    doc = pdfium.PdfDocument(str(pdf))
    try:
        for i in range(len(doc)):
            page = doc[i]
            bitmap = page.render(scale=scale)
            pil_image = bitmap.to_pil()      # 直接产 PIL Image，无需中间文件
            pil_image.save(slides_dir / f"slide-{i + 1:02d}.png")
            page.close()
    finally:
        doc.close()
    slide_pngs = sorted(slides_dir.glob("slide-*.png"), key=_natural_key)
    if not slide_pngs:
        _die("pypdfium2 did not produce slide PNG files")
    return slide_pngs


def make_contact_sheets(slide_pngs: Sequence[Path], sheets_dir: Path, grid: int) -> List[Path]:
    Image, ImageDraw, ImageFont = _import_pillow()
    sheets_dir.mkdir(parents=True, exist_ok=True)

    if grid == 6:
        cols, rows = 3, 2
    elif grid == 9:
        cols, rows = 3, 3
    else:
        _die("grid must be 6 or 9")

    thumb_w, thumb_h = 360, 203  # 16:9 thumbnail box
    label_h = 26
    gap = 18
    margin = 28
    cell_w = thumb_w
    cell_h = thumb_h + label_h
    canvas_w = margin * 2 + cols * cell_w + (cols - 1) * gap
    canvas_h = margin * 2 + rows * cell_h + (rows - 1) * gap

    try:
        font = ImageFont.truetype("Arial.ttf", 15)
    except Exception:
        font = ImageFont.load_default()

    outputs: List[Path] = []
    for sheet_idx, start in enumerate(range(0, len(slide_pngs), grid), start=1):
        group = slide_pngs[start : start + grid]
        first_page = start + 1
        last_page = start + len(group)
        canvas = Image.new("RGB", (canvas_w, canvas_h), "#F4F4F2")
        draw = ImageDraw.Draw(canvas)

        for i, slide_path in enumerate(group):
            row = i // cols
            col = i % cols
            x = margin + col * (cell_w + gap)
            y = margin + row * (cell_h + gap)
            with Image.open(slide_path) as img:
                img = img.convert("RGB")
                img.thumbnail((thumb_w, thumb_h), Image.Resampling.LANCZOS)
                frame = Image.new("RGB", (thumb_w, thumb_h), "#FFFFFF")
                px = (thumb_w - img.width) // 2
                py = (thumb_h - img.height) // 2
                frame.paste(img, (px, py))
            canvas.paste(frame, (x, y))
            draw.rectangle([x, y, x + thumb_w, y + thumb_h], outline="#D0D0D0", width=1)
            draw.text((x + 8, y + thumb_h + 6), f"Slide {start + i + 1}", fill="#333333", font=font)

        out = sheets_dir / f"style_sheet_{sheet_idx:02d}_pages_{first_page:03d}-{last_page:03d}.png"
        canvas.save(out, quality=95)
        outputs.append(out)
    return outputs


def write_manifest(run_dir: Path, pptx: Path, pdf: Path, slide_pngs: Sequence[Path], sheets: Sequence[Path], grid: int, dpi: int) -> Path:
    manifest = run_dir / "STYLE_PREVIEW.md"
    sheet_list = "\n".join(f"- `{p.relative_to(run_dir)}`" for p in sheets)
    slide_list = "\n".join(f"- `{p.relative_to(run_dir)}`" for p in slide_pngs)
    manifest.write_text(
        "# PPTX Style Preview\n\n"
        f"- Source PPTX: `{pptx}`\n"
        f"- Render DPI: `{dpi}`\n"
        f"- Grid: `{grid}` slides per contact sheet\n"
        f"- PDF: `{pdf.relative_to(run_dir)}`\n\n"
        "## Contact sheets for style reading\n\n"
        f"{sheet_list}\n\n"
        "## Individual slide PNGs\n\n"
        f"{slide_list}\n\n"
        "## How to use\n\n"
        "Read the contact-sheet PNGs first to infer the global visual language: "
        "canvas rhythm, background, color palette, typography hierarchy, recurring motifs, "
        "page-type patterns, chart/table style, image treatment, and whitespace density. "
        "Only read individual slide PNGs when a contact sheet is too small for local details. "
        "Then write the project DESIGN.md from these observations.\n",
        encoding="utf-8",
    )
    return manifest


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Convert a PPTX into 6/9-grid style contact-sheet PNGs.")
    parser.add_argument("pptx", help="Path to the source .pptx file")
    parser.add_argument("--outdir", default="style_preview", help="Output root directory; a run-specific subdirectory is created inside it")
    parser.add_argument("--grid", type=int, choices=(6, 9), default=9, help="Slides per contact sheet: 6 or 9")
    parser.add_argument("--dpi", type=int, default=120, help="PDF render DPI for individual slide PNGs")
    parser.add_argument("--soffice", default=os.environ.get("SOFFICE"), help="Path to LibreOffice soffice executable")
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv or sys.argv[1:])
    pptx = Path(args.pptx).expanduser().resolve()
    if not pptx.exists():
        _die(f"PPTX file not found: {pptx}")
    if pptx.suffix.lower() != ".pptx":
        _die(f"expected a .pptx file, got: {pptx.name}")

    soffice = _find_executable(
        args.soffice,
        [
            "soffice",
            "/Applications/LibreOffice.app/Contents/MacOS/soffice",
            "/opt/libreoffice/program/soffice",
            "/usr/bin/libreoffice",
        ],
    )
    if not soffice:
        _die(
            "LibreOffice soffice executable not found. Install LibreOffice and/or pass --soffice. "
            "macOS path is usually /Applications/LibreOffice.app/Contents/MacOS/soffice"
        )

    out_root = Path(args.outdir).expanduser().resolve()
    run_id = _dt.datetime.now().strftime("%Y%m%d_%H%M%S")
    safe_stem = re.sub(r"[^A-Za-z0-9._-]+", "_", pptx.stem).strip("_") or "pptx"
    run_dir = out_root / f"{safe_stem}_style_preview_{run_id}"
    pdf_dir = run_dir / "pdf"
    slides_dir = run_dir / "slides"
    sheets_dir = run_dir / "sheets"
    run_dir.mkdir(parents=True, exist_ok=False)

    pdf = convert_pptx_to_pdf(pptx, pdf_dir, soffice)
    slide_pngs = render_pdf_to_pngs(pdf, slides_dir, args.dpi)
    sheets = make_contact_sheets(slide_pngs, sheets_dir, args.grid)
    manifest = write_manifest(run_dir, pptx, pdf, slide_pngs, sheets, args.grid, args.dpi)

    print("[pptx-style] DONE")
    print(f"[pptx-style] run_dir: {run_dir}")
    print(f"[pptx-style] manifest: {manifest}")
    for sheet in sheets:
        print(f"[pptx-style] sheet: {sheet}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
