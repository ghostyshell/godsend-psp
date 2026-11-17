#!/bin/bash
# Build the prebuilt Store indexes and publish them to the covers bucket.
#
#   scripts/refresh-store-index.sh           # build, then upload to R2
#   scripts/refresh-store-index.sh --check   # build only, print the counts
#
# The desktop app reads these indexes before building anything itself, so a
# stale index here only costs a live fallback build, never a broken Store.
set -euo pipefail

SELF=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$SELF/.." && pwd)
cd "$REPO"

R2_BUCKET=${R2_BUCKET:-torrent-search-covers-eu}
R2_PREFIX=psp/index
R2_BASE=${R2_BASE:-https://covers.debrid.in}

CHECK=0
while [ $# -gt 0 ]; do
  case "$1" in
    --check) CHECK=1; shift ;;
    -h | --help)
      sed -n '2,9p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      exit 2
      ;;
  esac
done

BIN="$REPO/target/release/godsend-psp"
needs_build=0
[ -x "$BIN" ] || needs_build=1
if [ "$needs_build" = 0 ] \
   && [ -n "$(find "$REPO/core/src" "$REPO/cli/src" "$REPO/Cargo.toml" "$REPO/Cargo.lock" \
        -newer "$BIN" -print -quit 2>/dev/null)" ]; then
  # A stale binary would publish an old index, so refresh it like a missing one.
  needs_build=1
fi
if [ "$needs_build" = 1 ]; then
  echo "building the release CLI (target/release/godsend-psp is missing or stale)" >&2
  cargo build -q -p godsend-psp-cli --release
fi

out=$(mktemp -d "${TMPDIR:-/tmp}/godsend-store-index.XXXXXX")
trap 'rm -rf "$out"' EXIT

"$BIN" store-index --out "$out"

for kind in minerva rroms; do
  [ -s "$out/$kind.json" ] || { echo "no $kind.json was written" >&2; exit 1; }
  printf '%s.json: %s bytes\n' "$kind" "$(wc -c < "$out/$kind.json" | tr -d ' ')" >&2
done

if [ "$CHECK" = 1 ]; then
  echo "--check: skipped the R2 upload" >&2
  exit 0
fi

if [ ! -f "$HOME/.r2.env" ]; then
  echo "no $HOME/.r2.env; cannot upload the store indexes" >&2
  exit 1
fi
# shellcheck disable=SC1090
. "$HOME/.r2.env"
: "${R2_S3_ENDPOINT:?R2_S3_ENDPOINT missing from ~/.r2.env}"
: "${R2_S3_ACCESS_KEY_ID:?R2_S3_ACCESS_KEY_ID missing from ~/.r2.env}"
: "${R2_S3_SECRET_ACCESS_KEY:?R2_S3_SECRET_ACCESS_KEY missing from ~/.r2.env}"
command -v aws >/dev/null 2>&1 || { echo "aws CLI not found; cannot upload" >&2; exit 1; }

export AWS_ACCESS_KEY_ID=$R2_S3_ACCESS_KEY_ID
export AWS_SECRET_ACCESS_KEY=$R2_S3_SECRET_ACCESS_KEY
export AWS_DEFAULT_REGION=auto

for kind in minerva rroms; do
  aws --endpoint-url "$R2_S3_ENDPOINT" s3 cp "$out/$kind.json" \
    "s3://$R2_BUCKET/$R2_PREFIX/$kind.json" \
    --content-type application/json --no-progress >/dev/null
  echo "uploaded $R2_BASE/$R2_PREFIX/$kind.json" >&2
done
