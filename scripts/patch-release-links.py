#!/usr/bin/env python3
"""Rewrite the README download table from an R2 upload manifest.

Usage: python3 -I scripts/patch-release-links.py <links.txt> [version]

Rows are `name | url | size | sha256 | bytes`. Writes docs/downloads.json with
the R2 URL in both `r2` (what the in-app updater reads) and `url` (what the
site reads), plus the exact byte count and sha256. A row whose URL is missing or
EMPTY still appears with `pending`, so a failed upload never blanks out a row
that worked.
"""
import datetime
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
README = ROOT / "README.md"
MANIFEST = ROOT / "docs" / "downloads.json"
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


def cell(url):
    return f"[Download]({url})" if url and url != "EMPTY" else "pending"


def order(name):
    low = name.lower()
    if low.endswith(".dmg"):
        return (0, name)
    if low.endswith(".exe"):
        return (1, name)
    if low.endswith(".appimage"):
        return (2, name)
    if low.endswith(".deb"):
        return (3, name)
    return (4, name)


def demo():
    rows = [
        ("godsend-psp-9.9.9-linux-x64.AppImage", "https://covers.debrid.in/psp/releases/9.9.9/a.AppImage", "132.0 MiB", "abc", "138394104"),
        ("godsend-psp-cli-9.9.9-windows-x64.zip", "EMPTY", "2.1 MiB", "def", "2202009"),
    ]
    m = build(rows, "")
    assert m["version"] == "9.9.9", m["version"]
    assert m["artifacts"][0]["r2"] == m["artifacts"][0]["url"] == "https://covers.debrid.in/psp/releases/9.9.9/a.AppImage", m
    assert m["artifacts"][0]["bytes"] == 138394104, m
    assert m["artifacts"][1]["r2"] == "" and m["artifacts"][1]["bytes"] == 2202009, m
    t = table(m)
    assert "| `godsend-psp-9.9.9-linux-x64.AppImage` | [Download](https://covers.debrid.in/psp/releases/9.9.9/a.AppImage) | 132.0 MiB |" in t, t
    assert "| `godsend-psp-cli-9.9.9-windows-x64.zip` | pending | 2.1 MiB |" in t, t
    print("demo OK")


def build(rows, version):
    artifacts = []
    for name, url, size, sha, nbytes in sorted(rows, key=lambda r: order(r[0])):
        direct = "" if url == "EMPTY" else url
        artifacts.append({
            "name": name,
            "label": label_for(name),
            "r2": direct,
            "url": direct,
            "size": size,
            "bytes": int(nbytes) if nbytes.isdigit() else None,
            "sha256": sha,
        })
    if not version and artifacts:
        match = re.search(r"\d+\.\d+\.\d+", artifacts[0]["name"])
        if match:
            version = match.group(0)
    return {"version": version, "updated": datetime.date.today().isoformat(), "artifacts": artifacts}


def table(manifest):
    lines = ["| Build | Download | Size |", "| --- | --- | --- |"]
    for art in manifest["artifacts"]:
        lines.append(f"| `{art['name']}` | {cell(art['url'])} | {art['size'] or 'pending'} |")
    return "\n".join(lines)


def main(argv):
    if len(argv) < 2:
        raise SystemExit(__doc__)
    rows = []
    for line in Path(argv[1]).read_text().splitlines():
        parts = [p.strip() for p in line.split("|")]
        if len(parts) >= 2 and parts[0]:
            rows.append((parts + [""] * 5)[:5])
    if not rows:
        raise SystemExit("no usable rows in the upload manifest")
    manifest = build(rows, argv[2] if len(argv) > 2 else "")
    # Keep current-version artifacts this run did not rebuild (Linux/Windows
    # come from CI, the local run only builds macOS). A row the manifest no
    # longer carries would drop that platform from the site and the updater.
    previous = {}
    if MANIFEST.is_file():
        try:
            previous = json.loads(MANIFEST.read_text())
        except Exception:
            previous = {}
    if previous.get("version") == manifest["version"]:
        seen = {a["name"] for a in manifest["artifacts"]}
        for art in previous.get("artifacts", []):
            if art.get("r2") and art.get("name") not in seen:
                manifest["artifacts"].append(art)
        manifest["artifacts"].sort(key=lambda a: order(a["name"]))
    MANIFEST.parent.mkdir(parents=True, exist_ok=True)
    MANIFEST.write_text(json.dumps(manifest, indent=2) + "\n")
    text = README.read_text()
    start = text.index(START) + len(START)
    end = text.index(END)
    README.write_text(text[:start] + "\n" + table(manifest) + "\n" + text[end:])
    print(f"patched {len(rows)} rows for {manifest['version']}")


if __name__ == "__main__":
    if "--demo" in sys.argv:
        demo()
    else:
        main(sys.argv)
