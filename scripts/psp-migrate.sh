#!/bin/bash
# Finish the LME -> ARK-5 migration on a mounted memory stick and install the
# GODsend homebrew utilities.
#
#   scripts/psp-migrate.sh "/Volumes/NO NAME"                  # dry run
#   scripts/psp-migrate.sh "/Volumes/NO NAME" --apply          # writes
#   scripts/psp-migrate.sh "/Volumes/NO NAME" --apply --purge  # then erase LME
#
# Safety:
# - Refuses to run unless FasterARK (PSP/GAME/FasterARK) and the ARK save
#   folder are already on the stick, i.e. ARK was installed first.
# - By default nothing is deleted: the LME installer/launcher are moved into
#   PSP/GAME/_retired/ (two levels deep, so the XMB stops listing them) and
#   SEPLUGINS/GAME.TXT is rewritten with a timestamped backup beside it.
# - --purge backs PSP/GAME/_retired plus SEPLUGINS to
#   ~/Backups/godsend-psp/lme-removed-<stamp>.zip, verifies the archive, then
#   removes them from the stick.
# - cIPL is never touched here. PSP-3000, PSP Street and E1000 cannot install
#   it, and on cIPL-capable models it must be done on the console.

set -euo pipefail

S=${1:?usage: scripts/psp-migrate.sh "<stick root>" [--apply] [--purge]}
shift
APPLY=""
PURGE=""
for arg in "$@"; do
  case "$arg" in
    --apply) APPLY=1 ;;
    --purge) PURGE=1 ;;
    *) printf 'unknown option: %s\n' "$arg" >&2; exit 2 ;;
  esac
done
[ -n "$PURGE" ] && [ -z "$APPLY" ] && { echo "--purge needs --apply"; exit 2; }

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

# 2. Retire the LME apps without deleting them (unless --purge).
RET="$S/PSP/GAME/_retired"
for app in installer launcher; do
  if [ -d "$S/PSP/GAME/$app" ]; then
    say "retire LME $app -> PSP/GAME/_retired/$app"
    run mkdir -p "$RET"
    run mv "$S/PSP/GAME/$app" "$RET/$app"
  fi
done

if [ -n "$PURGE" ] && [ -d "$RET" ]; then
  B="$HOME/Backups/godsend-psp"
  OUT="$B/lme-removed-$(date +%Y%m%d-%H%M%S).zip"
  say "purge: archive PSP/GAME/_retired + SEPLUGINS -> $OUT"
  if [ -n "$APPLY" ]; then
    mkdir -p "$B"
    ( cd "$S" && zip -qr "$OUT" PSP/GAME/_retired SEPLUGINS )
    unzip -tq "$OUT" >/dev/null || { echo "archive check failed; keeping _retired"; exit 1; }
    say "purge: archive verified ($(stat -f%z "$OUT" 2>/dev/null || stat -c%s "$OUT") bytes)"
    rm -rf "$RET"
    say "purge: removed PSP/GAME/_retired"
  else
    say "  would: zip PSP/GAME/_retired SEPLUGINS, verify, then rm -rf PSP/GAME/_retired"
  fi
fi

# 3. Drop the LME legacy plugin, keep everything else.
for txt in "$S/SEPLUGINS/GAME.TXT" "$S/seplugins/GAME.TXT"; do
  [ -f "$txt" ] || continue
  if grep -qiE 'leda\.prx' "$txt"; then
    if [ -n "$PURGE" ]; then
      say "purge: remove leda.prx and its GAME.TXT line"
      if [ -n "$APPLY" ]; then
        cp "$txt" "$txt.bak-$(date +%Y%m%d-%H%M%S)"
        grep -vi 'leda\.prx' "$txt" > "$txt.new" || true
        mv "$txt.new" "$txt"
        rm -f "$S/SEPLUGINS/leda.prx" "$S/seplugins/leda.prx"
      else
        say "  would: drop leda lines from ${txt#$S/} and delete leda.prx"
      fi
    else
      say "disable leda.prx in ${txt#$S/}"
      if [ -n "$APPLY" ]; then
        cp "$txt" "$txt.bak-$(date +%Y%m%d-%H%M%S)"
        awk '{ if ($0 ~ /leda\.prx/ && $0 !~ /^[[:space:]]*#/) print "# " $0; else print }' "$txt" > "$txt.new"
        mv "$txt.new" "$txt"
      fi
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
