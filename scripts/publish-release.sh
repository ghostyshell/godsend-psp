#!/bin/bash
# Upload release artifacts to GoFile and (optionally) file.kiwi, mirroring
# the godsend-360 / keepsake-atelier release flow.
#
# Usage: scripts/publish-release.sh <artifact-file> [<artifact-file> ...]
# Requires: curl, jq. Prints the download links at the end.

set -euo pipefail

if [ $# -eq 0 ]; then
  echo "usage: $0 <artifact-file> [artifact...]" >&2
  exit 1
fi

for f in "$@"; do
  [ -f "$f" ] || { echo "not a file: $f" >&2; exit 1; }
done

links_file=".release-links-$(date +%Y%m%d-%H%M%S).txt"
: > "$links_file"

for f in "$@"; do
  echo "uploading $f ..."
  # GoFile: get a guest account token, then upload.
  token=$(curl -s -X POST https://api.gofile.io/accounts | jq -r '.data.token')
  server=$(curl -s -H "Authorization: Bearer $token" https://api.gofile.io/servers | jq -r '.data[0].name')
  url=$(curl -s -H "Authorization: Bearer $token" \
    -F "file=@${f}" "https://${server}.gofile.io/contents/uploadfile" | jq -r '.data.downloadPage')
  echo "$url  $f" >> "$links_file"
  echo "  -> $url"
done

echo
echo "links written to $links_file:"
cat "$links_file"
