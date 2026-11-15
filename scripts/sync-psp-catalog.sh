#!/bin/bash
# Validate and publish the PSP themes/mods catalog to R2.
#
# Layout uploaded under the configured prefix:
#   psp/themes.json          array of {name, url, format, size_bytes}
#   psp/mods/<SERIAL>.json   array of {name, author, version, url, size_bytes, notes, kind}
#   psp/files/...            the theme and mod binaries the JSON points at
#
# Source directory: psp-catalog/ in the repo (or PSP_CATALOG_DIR).
# Requires rclone configured for the R2 bucket, or aws s3 with R2 env vars
# (R2_S3_ENDPOINT, R2_S3_ACCESS_KEY_ID, R2_S3_SECRET_ACCESS_KEY, R2_S3_BUCKET).
#
#   scripts/sync-psp-catalog.sh            validate, then upload
#   scripts/sync-psp-catalog.sh --dry-run  validate and print the upload command
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

SRC=${PSP_CATALOG_DIR:-psp-catalog}
DEST=${PSP_CATALOG_PREFIX:-psp}
DRY_RUN=0
BUCKET=${R2_S3_BUCKET:-torrent-search-covers-eu}

while [ $# -gt 0 ]; do
  case "$1" in
    --dry-run|-n) DRY_RUN=1; shift ;;
    -h|--help) sed -n '2,15p' "$0"; exit 0 ;;
    *) echo "unknown option: $1" >&2; exit 2 ;;
  esac
done

[ -d "$SRC" ] || { echo "no catalog at $SRC" >&2; exit 1; }

# Parse both JSON shapes, resolve each url to a file under files/, and fail on
# anything missing. Prints a human summary on success.
python3 - "$SRC" <<'PY'
import glob
import json
import os
import sys
import urllib.parse

src = sys.argv[1]
files_root = os.path.join(src, "files")
errors = []
themes = []
mods_files = 0
mods_entries = 0

def load(path, label):
    try:
        with open(path, encoding="utf-8") as fh:
            return json.load(fh)
    except FileNotFoundError:
        errors.append(f"{label}: missing {path}")
    except json.JSONDecodeError as exc:
        errors.append(f"{label}: invalid JSON in {path}: {exc}")
    return None

def target(url):
    """Map a catalog url to a path relative to files/."""
    if not isinstance(url, str) or not url.strip():
        return None
    parsed = urllib.parse.urlparse(url.strip())
    path = parsed.path if parsed.scheme else url.split("?")[0].split("#")[0]
    path = urllib.parse.unquote(path).replace("\\", "/")
    marker = "/files/"
    if marker in path:
        rel = path.split(marker, 1)[1]
    else:
        rel = path.lstrip("/")
        if rel.startswith("files/"):
            rel = rel[len("files/"):]
        else:
            rel = os.path.basename(rel)
    rel = rel.strip("/")
    if not rel or ".." in rel.split("/"):
        return None
    return rel

def require_file(label, item, expected_exts=None):
    url = item.get("url")
    rel = target(url)
    if not rel:
        errors.append(f"{label}: bad url {url!r}")
        return
    full = os.path.join(files_root, rel)
    if not os.path.isfile(full):
        errors.append(f"{label}: missing file files/{rel} (url {url})")
    if expected_exts:
        ext = os.path.splitext(rel)[1].lower().lstrip(".")
        if ext not in expected_exts:
            errors.append(f"{label}: files/{rel} must be one of {expected_exts}, got {ext!r}")

themes_data = load(os.path.join(src, "themes.json"), "themes.json")
if themes_data is not None:
    if not isinstance(themes_data, list):
        errors.append("themes.json: expected a JSON array")
    else:
        themes = themes_data
        for i, item in enumerate(themes):
            label = f"themes.json[{i}]"
            if not isinstance(item, dict):
                errors.append(f"{label}: expected an object")
                continue
            if not str(item.get("name", "")).strip():
                errors.append(f"{label}: missing name")
            size = item.get("size_bytes", 0)
            if not isinstance(size, int) or size < 0:
                errors.append(f"{label}: size_bytes must be a non-negative integer")
            require_file(label, item, ("ctf", "ptf"))
            fmt = str(item.get("format", "")).lower()
            if fmt and fmt not in ("ctf", "ptf"):
                errors.append(f"{label}: format must be ctf or ptf")

for path in sorted(glob.glob(os.path.join(src, "mods", "*.json"))):
    serial = os.path.splitext(os.path.basename(path))[0]
    label = f"mods/{serial}.json"
    data = load(path, label)
    if data is None:
        continue
    if not isinstance(data, list):
        errors.append(f"{label}: expected a JSON array")
        continue
    mods_files += 1
    mods_entries += len(data)
    for i, item in enumerate(data):
        row = f"{label}[{i}]"
        if not isinstance(item, dict):
            errors.append(f"{row}: expected an object")
            continue
        if not str(item.get("name", "")).strip():
            errors.append(f"{row}: missing name")
        size = item.get("size_bytes", 0)
        if not isinstance(size, int) or size < 0:
            errors.append(f"{row}: size_bytes must be a non-negative integer")
        require_file(row, item)

files = []
file_bytes = 0
for root, _dirs, names in os.walk(files_root):
    for name in names:
        if name == ".DS_Store":
            continue
        full = os.path.join(root, name)
        files.append(os.path.relpath(full, files_root))
        file_bytes += os.path.getsize(full)

if errors:
    print("catalog check failed:", file=sys.stderr)
    for err in errors:
        print(f"  - {err}", file=sys.stderr)
    sys.exit(1)

print(f"themes: {len(themes)} entries")
print(f"mods: {mods_entries} entries across {mods_files} serial files")
print(f"files: {len(files)} files, {file_bytes / (1024 * 1024):.2f} MiB")

referenced = set()
for item in themes:
    rel = target(item.get("url"))
    if rel:
        referenced.add(rel)
for path in glob.glob(os.path.join(src, "mods", "*.json")):
    try:
        data = json.load(open(path, encoding="utf-8"))
    except (OSError, ValueError):
        continue
    if isinstance(data, list):
        for item in data:
            if isinstance(item, dict):
                rel = target(item.get("url"))
                if rel:
                    referenced.add(rel)
extra = [rel for rel in files if rel.replace(os.sep, "/") not in referenced]
if extra:
    print(f"note: {len(extra)} file(s) under files/ are not referenced by any JSON")
PY

if [ "$DRY_RUN" = "1" ]; then
  if command -v rclone >/dev/null 2>&1 && rclone listremotes 2>/dev/null | grep -q '^r2:'; then
    echo "dry run: rclone sync $SRC r2:$BUCKET/$DEST --exclude sources*.json --exclude .DS_Store"
  elif command -v aws >/dev/null 2>&1 && [ -n "${R2_S3_ENDPOINT:-}" ]; then
    echo "dry run: aws --endpoint-url $R2_S3_ENDPOINT s3 sync $SRC s3://$BUCKET/$DEST --exclude sources*.json --exclude .DS_Store"
  else
    echo "dry run: no uploader configured (rclone r2: or R2_S3_* env)"
  fi
  echo "dry run: nothing uploaded"
  exit 0
fi

if command -v rclone >/dev/null 2>&1 && rclone listremotes 2>/dev/null | grep -q '^r2:'; then
  rclone sync "$SRC" "r2:$BUCKET/$DEST" \
    --exclude 'sources*.json' --exclude '.DS_Store' --progress
elif command -v aws >/dev/null 2>&1 && [ -n "${R2_S3_ENDPOINT:-}" ]; then
  aws --endpoint-url "$R2_S3_ENDPOINT" s3 sync "$SRC" "s3://$BUCKET/$DEST" \
    --exclude 'sources*.json' --exclude '.DS_Store'
else
  echo "configure rclone r2: or aws with R2_S3_* env first" >&2
  exit 1
fi
echo "uploaded $SRC to $DEST on R2"
