#!/bin/bash
# Install (or remove) the automatic release watcher. Runs every 30 minutes;
# publishes only when tauri.conf.json is ahead of the live downloads.json.
set -euo pipefail
SELF=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$SELF/.." && pwd)
LABEL=click.godsendpsp.auto-release
INTERVAL=1800
UNINSTALL=0
while [ $# -gt 0 ]; do
  case "$1" in
    --uninstall) UNINSTALL=1; shift ;;
    --interval) INTERVAL=$2; shift 2 ;;
    *) echo "unknown option: $1" >&2; exit 2 ;;
  esac
done
PLIST="$HOME/Library/LaunchAgents/$LABEL.plist"
UID_NUM=$(id -u)
if [ "$UNINSTALL" = "1" ]; then
  launchctl bootout "gui/$UID_NUM/$LABEL" 2>/dev/null || true
  rm -f "$PLIST"
  echo "removed $LABEL"
  exit 0
fi
mkdir -p "$HOME/Library/LaunchAgents"
cat > "$PLIST" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>Label</key><string>$LABEL</string>
  <key>ProgramArguments</key><array>
    <string>/usr/bin/caffeinate</string><string>-i</string>
    <string>$REPO/scripts/auto-release.sh</string>
  </array>
  <key>WorkingDirectory</key><string>$REPO</string>
  <key>StartInterval</key><integer>$INTERVAL</integer>
  <key>StandardOutPath</key><string>$REPO/.auto-release.log</string>
  <key>StandardErrorPath</key><string>$REPO/.auto-release.log</string>
</dict></plist>
EOF
launchctl bootout "gui/$UID_NUM/$LABEL" 2>/dev/null || true
launchctl bootstrap "gui/$UID_NUM" "$PLIST"
echo "installed $LABEL (every ${INTERVAL}s)"
