#!/bin/bash
# Fetch static ffmpeg + ffprobe for the target platform into the Tauri
# resources so every build ships its own pair (like Keepsake Atelier).
#
#   scripts/fetch-ffmpeg.sh darwin-arm64 | darwin-x64 | linux-x64 | win32-x64
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

PLAT=${1:?usage: scripts/fetch-ffmpeg.sh <darwin-arm64|darwin-x64|linux-x64|win32-x64>}
BASE="https://github.com/eugeneware/ffmpeg-static/releases/latest/download"
RES="app/src-tauri/resources"
mkdir -p "$RES"

suffix=""
[ "$PLAT" = "win32-x64" ] && suffix=".exe"

for tool in ffmpeg ffprobe; do
  out="$RES/${tool}${suffix}"
  # Always fetch: a stale binary from another architecture must never ship.
  rm -f "$out"
  echo "[fetch] ${tool}-${PLAT}"
  curl -fsSL --retry 3 --max-time 600 "$BASE/${tool}-${PLAT}.gz" -o "$RES/${tool}.gz"
  gunzip -f "$RES/${tool}.gz"
  mv "$RES/$tool" "$out"
  chmod +x "$out" 2>/dev/null || true
done

# On the build host itself, prove the binaries run (skip for cross platforms
# and Windows executables).
if [ "$suffix" = "" ] && [ "$(uname -s)" = "Darwin" ] && [ "$PLAT" = "darwin-$(uname -m | sed 's/x86_64/x64/')" ]; then
  "$RES/ffmpeg" -version | head -1
  "$RES/ffprobe" -version | head -1
elif [ "$suffix" = "" ] && [ "$(uname -s)" = "Linux" ] && [ "$PLAT" = "linux-x64" ]; then
  "$RES/ffmpeg" -version | head -1
  "$RES/ffprobe" -version | head -1
fi
echo "done: $RES/${PLAT} pairs"
