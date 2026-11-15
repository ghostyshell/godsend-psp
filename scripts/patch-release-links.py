#!/usr/bin/env python3
"""Rewrite the README download table from an upload manifest.

Usage: python3 -I scripts/patch-release-links.py <links.txt>

Rows are `name | gofile-url | kiwi-url | size | sha256`. A row whose URL is
missing or EMPTY still appears with `pending`, so a failed mirror never blanks
out the half that worked.
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
README = ROOT / "README.md"
START = "<!-- downloads:start -->"
END = "<!-- downloads:end -->"


def label_for(name):
    low = name.lower()
    if low.endswith(".dmg"):
        return "macOS (Intel)" if "x64" in low else "macOS (Apple Silicon)"
    if low.endswith(".tar.gz"):
        if "linux" in low:
            return "Linux (x86_64, CLI)"
        return "macOS (Intel, CLI)" if "x64" in low else "macOS (Apple Silicon, CLI)"
    if low.endswith(".zip"):
        return "Windows (x86_64, CLI)"
    if low.endswith(".appimage"):
        return "Linux (x86_64, AppImage)"
    if low.endswith(".deb"):
        return "Linux (x86_64, deb)"
    if low.endswith(".exe"):
        return "Windows (x86_64, installer)"
    return name


def cell(url, text):
    return f"[{text}]({url})" if url and url != "EMPTY" else "pending"


def main():
    manifest = Path(sys.argv[1])
    rows = []
    for line in manifest.read_text().splitlines():
        parts = [p.strip() for p in line.split("|")]
        if len(parts) >= 3:
            rows.append(parts)

    def sort_key(row):
        name = row[0].lower()
        family = (
            "dmg" if name.endswith(".dmg")
            else "exe" if name.endswith(".exe")
            else "appimage" if name.endswith(".appimage")
            else "deb" if name.endswith(".deb")
            else "cli"
        )
        return (family, name)

    rows.sort(key=sort_key)
    table = [
        "| Build | GoFile (primary) | file.kiwi (backup) | Size |",
        "| --- | --- | --- | --- |",
    ]
    for name, gofile, kiwi, size, *_ in rows:
        table.append(
            f"| `{name}` | {cell(gofile, 'Download')} | {cell(kiwi, 'Mirror')} | {size or ''} |"
        )

    text = README.read_text()
    start = text.index(START) + len(START)
    end = text.index(END)
    new = text[:start] + "\n" + "\n".join(table) + "\n" + text[end:]
    README.write_text(new)
    print(f"patched {len(rows)} rows in README.md")


if __name__ == "__main__":
    main()
