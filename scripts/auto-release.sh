#!/bin/bash
# Publish automatically when the repo version is ahead of the live feed.
# Builds all platforms through the workflow, then uploads via publish-release.
set -euo pipefail
export PATH="/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:$PATH"
cd "$(git rev-parse --show-toplevel)"
V=$(python3 -c 'import json;print(json.load(open("app/src-tauri/tauri.conf.json"))["version"])')
LIVE=$(curl -fsS "https://ghostyshell.github.io/godsend-psp/downloads.json?v=$(date +%s)" 2>/dev/null \
  | python3 -c 'import json,sys
try: print(json.load(sys.stdin).get("version",""))
except Exception: print("")' 2>/dev/null || true)
if [ "$LIVE" = "$V" ]; then
  echo "up to date ($V)"
  exit 0
fi
[ -n "${GH_TOKEN:-}" ] || true
echo "releasing $V (live ${LIVE:-unknown})"
gh workflow run build.yml --repo ghostyshell/godsend-psp || { echo "dispatch failed"; exit 1; }
RUN=""
for _ in $(seq 1 80); do
  RUN=$(gh run list --repo ghostyshell/godsend-psp --workflow build --limit 3 \
    --json databaseId,status,event \
    --jq '[.[]|select(.event=="workflow_dispatch" and (.status=="queued" or .status=="in_progress" or .status=="requested"))][0].databaseId' 2>/dev/null || true)
  [ -n "$RUN" ] && break
  sleep 6
done
[ -n "$RUN" ] || { echo "no CI run appeared"; exit 1; }
for _ in $(seq 1 120); do
  S=$(gh run view "$RUN" --repo ghostyshell/godsend-psp --json status,conclusion --jq '.status+"/"+(.conclusion//"-")')
  case "$S" in
    completed/success) break ;;
    completed/*) echo "CI failed: $S"; exit 1 ;;
  esac
  sleep 15
done
D=$(mktemp -d)
gh api "repos/ghostyshell/godsend-psp/actions/runs/$RUN/artifacts" \
  --jq '.artifacts[]|.name+" "+(.id|tostring)' > "$D/a.txt"
mkdir -p "$D/raw"
while read -r n id; do
  gh api "repos/ghostyshell/godsend-psp/actions/artifacts/$id/zip" > "$D/$n.zip" \
    && unzip -oq "$D/$n.zip" -d "$D/raw"
done < "$D/a.txt"
find "$D/raw" -type f \( -name "*.dmg" -o -name "*.exe" -o -name "*.AppImage" \
  -o -name "*.deb" -o -name "*.tar.gz" -o -name "godsend-psp-cli-*.zip" \) -exec mv {} "$D/" \;
./scripts/publish-release.sh --artifacts "$D"
echo "released $V"
