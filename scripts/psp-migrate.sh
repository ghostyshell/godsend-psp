#!/bin/bash
# Finish the LME -> ARK-5 migration on a mounted memory stick and install the
# GODsend homebrew utilities.
#
#   scripts/psp-migrate.sh "/Volumes/NO NAME"          # dry run, prints actions
#   scripts/psp-migrate.sh "/Volumes/NO NAME" --apply  # writes to the stick
#
# Safety:
# - Refuses to run unless FasterARK (PSP/GAME/FasterARK) and the ARK save
#   folder are already on the stick, i.e. ARK was installed first.
# - Never deletes anything: the LME installer/launcher are moved into
#   PSP/GAME/_retired/ (two levels deep, so the XMB stops listing them) and
#   SEPLUGINS/GAME.TXT is rewritten with a timestamped backup beside it.
# - cIPL is never touched here. PSP-3000, PSP Street and E1000 cannot install
#   it, and on cIPL-capable models it must be done on the console.

set -euo pipefail

S=${1:?usage: scripts/psp-migrate.sh "<stick root>" [--apply]}
APPLY=${2:-}
[ "$APPLY" = "--apply" ] || APPLY=""

say() { printf '%s\n' "$*"; }
run() {
  if [ -n "$APPLY" ]; then "$@"; else say "  would: $*"; fi
}

[ -d "$S/PSP" ] || { say "not a PSP stick: $S"; exit 2; }
[ -d "$S/PSP/GAME/FasterARK" ] || { say "FasterARK is not installed; run the Jailbreak install first"; exit 2; }
[ -d "$S/PSP/SAVEDATA/ARK_01234" ] || { say "ARK_01234 is missing; boot FasterARK once, then rerun"; exit 2; }

say "stick: $S"
say "ARK-5: present"
[ -n "$APPLY" ] || say "DRY RUN (pass --apply to write)"

# 1. GODsend homebrew utilities.
OUT="$(cd "$(dirname "$0")/.." && pwd)/homebrew/out"
if [ -d "$OUT" ]; then
  for z in "$OUT"/*.zip; do
    [ -f "$z" ] || continue
    name=$(basename "$z" .zip)
    if [ -d "$S/PSP/GAME/$name" ]; then
      say "app $name: already installed"
    else
      say "app $name: install"
      if [ -n "$APPLY" ]; then
        tmp=$(mktemp -d)
        unzip -q "$z" -d "$tmp"
        ditto "$tmp/PSP/GAME/$name" "$S/PSP/GAME/$name"
        rm -rf "$tmp"
      fi
    fi
  done
else
  say "note: $OUT is missing, skipping homebrew install"
fi

# 2. Retire the LME apps without deleting them.
RET="$S/PSP/GAME/_retired"
for app in installer launcher; do
  if [ -d "$S/PSP/GAME/$app" ]; then
    say "retire LME $app -> PSP/GAME/_retired/$app"
    run mkdir -p "$RET"
    run mv "$S/PSP/GAME/$app" "$RET/$app"
  fi
done

# 3. Drop the LME legacy plugin line, keep everything else.
for txt in "$S/SEPLUGINS/GAME.TXT" "$S/seplugins/GAME.TXT"; do
  [ -f "$txt" ] || continue
  if grep -qiE '^[^#]*leda\.prx' "$txt"; then
    say "disable leda.prx in ${txt#$S/}"
    if [ -n "$APPLY" ]; then
      cp "$txt" "$txt.bak-$(date +%Y%m%d-%H%M%S)"
      awk '{ if ($0 ~ /leda\.prx/ && $0 !~ /^[[:space:]]*#/) print "# " $0; else print }' "$txt" > "$txt.new"
      mv "$txt.new" "$txt"
    fi
  else
    say "leda.prx already disabled or absent in ${txt#$S/}"
  fi
  break
done

# 4. Optional cleanup note.
if [ -d "$S/PSP/GAME/DC10" ]; then
  say "note: DC10 stays on the stick, but it cannot unbrick a PSP-3000 (no cIPL)"
fi

say ""
say "on console: run FasterARK after every boot (PSP-3000 has no cIPL),"
say "then launch GODsend SysInfo to confirm firmware, model family and battery."
