#!/bin/bash
# Publish the PSP themes/mods catalog to R2.
#
# Layout uploaded under the configured prefix:
#   psp/themes.json          array of {name, url, format, size_bytes}
#   psp/mods/<SERIAL>.json   array of {name, author, version, url, size_bytes, notes, kind}
#   psp/files/...            the theme and mod binaries the JSON points at
#
# Source directory: psp-catalog/ in the repo (or PSP_CATALOG_DIR).
# Requires rclone configured for the R2 bucket, or aws s3 with R2 env vars
# (R2_S3_ENDPOINT, R2_S3_ACCESS_KEY_ID, R2_S3_SECRET_ACCESS_KEY, R2_S3_BUCKET).
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
SRC=${PSP_CATALOG_DIR:-psp-catalog}
[ -d "$SRC" ] || { echo "no catalog at $SRC" >&2; exit 1; }
DEST=${PSP_CATALOG_PREFIX:-psp}

if command -v rclone >/dev/null 2>&1 && rclone listremotes 2>/dev/null | grep -q '^r2:'; then
  rclone sync "$SRC" "r2:${R2_S3_BUCKET:-torrent-search-covers-eu}/$DEST" --progress
elif command -v aws >/dev/null 2>&1 && [ -n "${R2_S3_ENDPOINT:-}" ]; then
  aws --endpoint-url "$R2_S3_ENDPOINT" s3 sync "$SRC" \
    "s3://${R2_S3_BUCKET:-torrent-search-covers-eu}/$DEST"
else
  echo "configure rclone r2: or aws with R2_S3_* env first" >&2
  exit 1
fi
echo "uploaded $SRC to $DEST on R2"
