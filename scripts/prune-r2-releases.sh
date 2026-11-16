#!/bin/bash
# Prune old release mirrors from the R2 bucket, keeping the newest semver
# versions. Called by publish-release.sh after the upload, and usable alone:
#
#   scripts/prune-r2-releases.sh                      # keep 3, real delete
#   scripts/prune-r2-releases.sh --dry-run            # show the plan only
#   scripts/prune-r2-releases.sh --keep 5
#   scripts/prune-r2-releases.sh --prefix psp/releases-test
#
# Versions are compared numerically (0.2.9 < 0.2.10), never lexically.
set -euo pipefail

BUCKET=${R2_BUCKET:-torrent-search-covers-eu}
PREFIX=psp/releases
KEEP=3
DRY=0

while [ $# -gt 0 ]; do
  case "$1" in
    --keep) KEEP=${2:?--keep needs a number}; shift 2 ;;
    --prefix) PREFIX=${2:?--prefix needs a value}; shift 2 ;;
    --dry-run) DRY=1; shift ;;
    -h | --help)
      sed -n '2,12p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      exit 2
      ;;
  esac
done

[[ "$KEEP" =~ ^[0-9]+$ ]] || { echo "--keep must be a number" >&2; exit 2; }
[ "$KEEP" -ge 1 ] || { echo "--keep must be at least 1" >&2; exit 2; }

if [ ! -f "$HOME/.r2.env" ]; then
  echo "no $HOME/.r2.env; nothing to prune" >&2
  exit 2
fi
# shellcheck disable=SC1090
. "$HOME/.r2.env"
: "${R2_S3_ENDPOINT:?R2_S3_ENDPOINT missing from ~/.r2.env}"
: "${R2_S3_ACCESS_KEY_ID:?R2_S3_ACCESS_KEY_ID missing from ~/.r2.env}"
: "${R2_S3_SECRET_ACCESS_KEY:?R2_S3_SECRET_ACCESS_KEY missing from ~/.r2.env}"

export AWS_ACCESS_KEY_ID=$R2_S3_ACCESS_KEY_ID
export AWS_SECRET_ACCESS_KEY=$R2_S3_SECRET_ACCESS_KEY
export AWS_DEFAULT_REGION=auto

list_prefixes() {
  aws --endpoint-url "$R2_S3_ENDPOINT" s3api list-objects-v2 \
    --bucket "$BUCKET" --prefix "$PREFIX/" --delimiter "/" \
    --query 'CommonPrefixes[].Prefix' --output text 2>/dev/null
}

listing=$(list_prefixes) || { echo "could not list s3://$BUCKET/$PREFIX/" >&2; exit 1; }

versions=$(printf '%s\n' "$listing" | tr '\t' '\n' \
  | sed "s#^$PREFIX/##; s#/\$##" \
  | grep -E '^[0-9]+\.[0-9]+\.[0-9]+$' \
  | sort -t. -k1,1n -k2,2n -k3,3n || true)

if [ -z "$versions" ]; then
  echo "no release versions under s3://$BUCKET/$PREFIX/"
  exit 0
fi

total=$(printf '%s\n' "$versions" | wc -l | tr -d ' ')
keep=$(printf '%s\n' "$versions" | tail -n "$KEEP")

echo "bucket  s3://$BUCKET/$PREFIX/"
echo "found   $total version(s); keeping newest $KEEP"
printf '%s\n' "$keep" | sed 's/^/  keep   /'

pruned=0
while IFS= read -r old; do
  [ -n "$old" ] || continue
  if printf '%s\n' "$keep" | grep -qx "$old"; then
    continue
  fi
  if [ "$DRY" = 1 ]; then
    echo "  plan   delete $old"
  else
    if aws --endpoint-url "$R2_S3_ENDPOINT" s3 rm \
      "s3://$BUCKET/$PREFIX/$old/" --recursive --only-show-errors >/dev/null 2>&1; then
      echo "  prune  deleted $old"
      pruned=$((pruned + 1))
    else
      echo "  FAIL   could not delete $old" >&2
    fi
  fi
done <<< "$versions"

[ "$DRY" = 1 ] || echo "pruned $pruned old release(s)"
