# /// script
# requires-python = ">=3.10"
# dependencies = ["pypdf>=5"]
# ///
"""Split a large PDF (e.g. a textbook) into per-chapter PDFs using its bookmarks.

Each chapter becomes its own source in LLM Wiki, so ingest produces a summary
page per chapter instead of compressing the whole book into one long-source
digest. Import the output directory with Sources → 导入 → 文件夹.

Usage:
    uv run scripts/split_pdf_chapters.py book.pdf
    uv run scripts/split_pdf_chapters.py book.pdf --level 2 --out ch/
    uv run scripts/split_pdf_chapters.py book.pdf --list
    uv run scripts/split_pdf_chapters.py book.pdf --every 30   # no bookmarks
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path

from pypdf import PdfReader, PdfWriter


@dataclass
class Section:
    title: str
    start: int  # 0-based, inclusive
    end: int  # 0-based, exclusive


def collect_outline(reader: PdfReader, max_level: int) -> list[tuple[int, str, int]]:
    """Return (level, title, page) for bookmarks up to max_level, in document order."""
    entries: list[tuple[int, str, int]] = []

    def walk(items: list, level: int) -> None:
        for item in items:
            if isinstance(item, list):
                if level < max_level:
                    walk(item, level + 1)
                continue
            try:
                page = reader.get_destination_page_number(item)
            except Exception:
                continue
            if page is None or page < 0:
                continue
            entries.append((level, str(item.title).strip(), page))

    walk(reader.outline, 1)
    return entries


def sections_from_outline(
    reader: PdfReader, level: int, include_front: bool
) -> list[Section]:
    total = len(reader.pages)
    starts: list[tuple[str, int]] = []
    for _, title, page in collect_outline(reader, level):
        # Several bookmarks can point at the same page (chapter + its first
        # section); join the titles so each page range is emitted once
        # without losing the chapter name.
        if starts and starts[-1][1] == page:
            starts[-1] = (f"{starts[-1][0]} - {title}", page)
        elif not starts or page > starts[-1][1]:
            starts.append((title, page))

    if include_front and starts and starts[0][1] > 0:
        starts.insert(0, ("Front Matter", 0))

    return [
        Section(title, start, starts[i + 1][1] if i + 1 < len(starts) else total)
        for i, (title, start) in enumerate(starts)
    ]


def sections_every(total: int, size: int) -> list[Section]:
    return [
        Section(f"Pages {s + 1}-{min(s + size, total)}", s, min(s + size, total))
        for s in range(0, total, size)
    ]


def safe_filename(title: str, max_len: int = 80) -> str:
    # Keep CJK and other Unicode letters; drop characters illegal on
    # macOS/Windows file systems.
    name = re.sub(r'[\\/:*?"<>|\x00-\x1f]', " ", title)
    name = re.sub(r"\s+", " ", name).strip(" .")
    return (name[:max_len].rstrip(" .") or "untitled")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("pdf", type=Path)
    parser.add_argument("--out", type=Path, help="output dir (default: <pdf name>/)")
    parser.add_argument(
        "--level", type=int, default=1, help="bookmark depth to split on (default 1)"
    )
    parser.add_argument(
        "--every", type=int, help="ignore bookmarks; split every N pages"
    )
    parser.add_argument(
        "--skip-front", action="store_true", help="drop pages before the first bookmark"
    )
    parser.add_argument(
        "--min-pages", type=int, default=1,
        help="merge sections shorter than this into the previous one",
    )
    parser.add_argument("--list", action="store_true", help="print the plan only")
    args = parser.parse_args()

    reader = PdfReader(args.pdf)
    if reader.is_encrypted:
        try:
            reader.decrypt("")
        except Exception:
            print("PDF is encrypted; decrypt it first.", file=sys.stderr)
            return 1

    total = len(reader.pages)
    if args.every:
        sections = sections_every(total, args.every)
    else:
        sections = sections_from_outline(reader, args.level, not args.skip_front)
        if not sections:
            print(
                "No bookmarks found. Re-run with --every N to split by page count.",
                file=sys.stderr,
            )
            return 1

    if args.min_pages > 1:
        merged: list[Section] = []
        for sec in sections:
            if merged and sec.end - sec.start < args.min_pages:
                merged[-1].end = sec.end
            else:
                merged.append(sec)
        sections = merged

    width = max(2, len(str(len(sections))))
    out_dir = args.out or args.pdf.with_suffix("")
    for i, sec in enumerate(sections, 1):
        name = f"{i:0{width}d} - {safe_filename(sec.title)}.pdf"
        print(f"{name}  (p.{sec.start + 1}-{sec.end}, {sec.end - sec.start} pages)")
        if args.list:
            continue
        out_dir.mkdir(parents=True, exist_ok=True)
        writer = PdfWriter()
        for p in range(sec.start, sec.end):
            writer.add_page(reader.pages[p])
        writer.add_metadata({"/Title": sec.title})
        with open(out_dir / name, "wb") as f:
            writer.write(f)

    if not args.list:
        print(f"\nWrote {len(sections)} files to {out_dir}/")
    return 0


if __name__ == "__main__":
    sys.exit(main())
