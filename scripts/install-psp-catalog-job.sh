#!/bin/bash
# Install (or remove) the daily PSP catalog scrape + sync as a user launchd
# agent. Default: every day at 04:12, wrapped in caffeinate -i.
#
#   scripts/install-psp-catalog-job.sh
#   scripts/install-psp-catalog-job.sh --time 05:00
#   scripts/install-psp-catalog-job.sh --uninstall
set -euo pipefail

SELF=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$SELF/.." && pwd)
LABEL=click.godsendpsp.psp-catalog
HOUR=4
MINUTE=12
UNINSTALL=0
LOAD=1

while [ $# -gt 0 ]; do
  case "$1" in
    --repo) REPO=$2; shift 2 ;;
    --label) LABEL=$2; shift 2 ;;
    --time) HOUR=${2%%:*}; MINUTE=${2##*:}; shift 2 ;;
    --uninstall) UNINSTALL=1; shift ;;
    --no-load) LOAD=0; shift ;;
    -h|--help) sed -n '2,8p' "$0"; exit 0 ;;
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

# Run the scraper first: a flaky source should not stop the existing catalog
# from being published, so the sync runs either way. launchd starts with a
# bare PATH, so pin the node that installed this agent and add the usual
# Homebrew directories for rclone/aws.
NODE_BIN=$(command -v node || true)
[ -n "$NODE_BIN" ] || { echo "node not found on PATH" >&2; exit 1; }
NODE_DIR=$(dirname "$NODE_BIN")
JOB="export PATH=\"$NODE_DIR:/opt/homebrew/bin:/usr/local/bin:\$PATH\"; cd \"$REPO\" && \"$NODE_BIN\" scripts/scrape-psp-catalog.mjs; \"$REPO/scripts/sync-psp-catalog.sh\""
JOB_XML=${JOB//&/&amp;}
JOB_XML=${JOB_XML//</&lt;}

mkdir -p "$HOME/Library/LaunchAgents"
cat > "$PLIST" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>$LABEL</string>
  <key>ProgramArguments</key>
  <array>
    <string>/usr/bin/caffeinate</string>
    <string>-i</string>
    <string>/bin/bash</string>
    <string>-lc</string>
    <string>$JOB_XML</string>
  </array>
  <key>WorkingDirectory</key><string>$REPO</string>
  <key>StartCalendarInterval</key>
  <dict>
    <key>Hour</key><integer>$HOUR</integer>
    <key>Minute</key><integer>$MINUTE</integer>
  </dict>
  <key>StandardOutPath</key><string>$REPO/.psp-catalog.log</string>
  <key>StandardErrorPath</key><string>$REPO/.psp-catalog.log</string>
  <key>RunAtLoad</key><false/>
</dict>
</plist>
EOF

if [ "$LOAD" = "1" ]; then
  launchctl bootout "gui/$UID_NUM/$LABEL" 2>/dev/null || true
  launchctl bootstrap "gui/$UID_NUM" "$PLIST"
fi
echo "installed $LABEL (daily at $(printf '%02d:%02d' "$HOUR" "$MINUTE"))"
