#!/usr/bin/env bash
#
# Build the bundled GODsend homebrew apps with PSPSDK inside Docker, package
# each one as PSP/GAME/<id>/EBOOT.PBP in a zip, and optionally publish the
# zips to R2 for the desktop app catalog.
#
#   scripts/build-homebrew.sh              build + zip into homebrew/out/
#   scripts/build-homebrew.sh --upload     also push to R2
#   scripts/build-homebrew.sh --verify     curl the public zips and check them
#
# R2 credentials come from ~/.r2.env (R2_S3_ACCESS_KEY_ID,
# R2_S3_SECRET_ACCESS_KEY, R2_S3_ENDPOINT).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HOMEBREW="$ROOT/homebrew"
OUT="$HOMEBREW/out"
IMAGE="${PSPDEV_IMAGE:-pspdev/pspdev:latest}"
BUCKET="torrent-search-covers-eu"
PREFIX="psp/homebrew"
BASE_URL="https://covers.debrid.in"

APPS=(
	godsend-cleaner
	godsend-saves
	godsend-sysinfo
	godsend-netcheck
)

UPLOAD=0
VERIFY=0

usage() {
	cat <<'EOF'
Usage: scripts/build-homebrew.sh [--upload] [--verify]

  (no flags)  build all four apps and zip them under homebrew/out/
  --upload    build, then copy each zip to R2 at psp/homebrew/<id>.zip
  --verify    download each public zip and assert HTTP 200 + byte size
  --help      this text
EOF
}

for arg in "$@"; do
	case "$arg" in
	--upload) UPLOAD=1 ;;
	--verify) VERIFY=1 ;;
	-h | --help) usage; exit 0 ;;
	*)
		echo "unknown flag: $arg" >&2
		usage >&2
		exit 2
		;;
	esac
done

for tool in docker zip curl; do
	if ! command -v "$tool" >/dev/null 2>&1; then
		echo "missing required tool: $tool" >&2
		exit 1
	fi
done

file_size() {
	# macOS stat then GNU stat
	stat -f%z "$1" 2>/dev/null || stat -c%s "$1"
}

mkdir -p "$OUT"

EBOOT_SIZES=()
ZIP_SIZES=()

for app in "${APPS[@]}"; do
	log="$OUT/$app.build.log"
	echo "== building $app"
	# psp-gcc runs under amd64 emulation here and occasionally segfaults in
	# LTO. Retry with a clean tree before treating it as a real failure.
	ok=0
	for attempt in 1 2 3; do
		if docker run --rm \
			--user "$(id -u):$(id -g)" \
			-v "$HOMEBREW:/src" \
			-w "/src/$app" \
			"$IMAGE" make clean all >"$log" 2>&1; then
			ok=1
			break
		fi
		if ! grep -q "Segmentation fault\|internal compiler error\|Killed" "$log"; then
			break
		fi
		echo "   compiler crashed (attempt $attempt/3), retrying clean"
	done
	if [ "$ok" -ne 1 ]; then
		echo "build failed for $app" >&2
		cat "$log" >&2
		exit 1
	fi
	if grep -Eq "warning:|error:" "$log"; then
		echo "compiler diagnostics for $app:" >&2
		grep -E "warning:|error:" "$log" >&2
		exit 1
	fi

	eboot="$HOMEBREW/$app/EBOOT.PBP"
	if [ ! -f "$eboot" ]; then
		echo "missing EBOOT.PBP for $app" >&2
		exit 1
	fi
	eboot_bytes="$(file_size "$eboot")"
	if [ "$eboot_bytes" -lt 20000 ]; then
		echo "EBOOT.PBP for $app is suspiciously small ($eboot_bytes bytes)" >&2
		exit 1
	fi

	rm -rf "$OUT/$app"
	mkdir -p "$OUT/$app/PSP/GAME/$app"
	cp "$eboot" "$OUT/$app/PSP/GAME/$app/EBOOT.PBP"

	rm -f "$OUT/$app.zip"
	# -D keeps the archive to the single EBOOT entry (no folder entries).
	(cd "$OUT/$app" && zip -q -X -D -r "$OUT/$app.zip" PSP)

	zip_bytes="$(file_size "$OUT/$app.zip")"
	EBOOT_SIZES+=("$eboot_bytes")
	ZIP_SIZES+=("$zip_bytes")
	echo "   EBOOT.PBP $eboot_bytes bytes -> out/$app.zip ($zip_bytes bytes)"
done

echo
echo "artifacts in homebrew/out/"
for app in "${APPS[@]}"; do
	printf '  %-18s zip %8s bytes\n' "$app" "$(file_size "$OUT/$app.zip")"
done

if [ "$UPLOAD" -eq 1 ]; then
	R2_ENV="$HOME/.r2.env"
	if [ ! -f "$R2_ENV" ]; then
		echo "missing $R2_ENV (R2 credentials)" >&2
		exit 1
	fi
	# shellcheck disable=SC1090
	set -a
	. "$R2_ENV"
	set +a
	: "${R2_S3_ENDPOINT:?R2_S3_ENDPOINT missing in ~/.r2.env}"
	: "${R2_S3_ACCESS_KEY_ID:?R2_S3_ACCESS_KEY_ID missing in ~/.r2.env}"
	: "${R2_S3_SECRET_ACCESS_KEY:?R2_S3_SECRET_ACCESS_KEY missing in ~/.r2.env}"
	export AWS_ACCESS_KEY_ID="$R2_S3_ACCESS_KEY_ID"
	export AWS_SECRET_ACCESS_KEY="$R2_S3_SECRET_ACCESS_KEY"
	export AWS_DEFAULT_REGION=auto
	unset AWS_SESSION_TOKEN

	for app in "${APPS[@]}"; do
		echo "== uploading $app.zip"
		aws s3 cp "$OUT/$app.zip" "s3://$BUCKET/$PREFIX/$app.zip" \
			--endpoint-url "$R2_S3_ENDPOINT" \
			--content-type application/zip \
			--only-show-errors
	done
	echo "uploaded ${#APPS[@]} zips to s3://$BUCKET/$PREFIX/"
fi

if [ "$VERIFY" -eq 1 ]; then
	echo
	echo "verifying public URLs"
	fail=0
	for app in "${APPS[@]}"; do
		url="$BASE_URL/$PREFIX/$app.zip"
		# Cache-bust the edge: R2 overwrites can serve a stale object briefly.
		url="$url?cb=$(date +%s)-$RANDOM"
		local_bytes="$(file_size "$OUT/$app.zip")"
		tmp="$(mktemp)"
		code="$(curl -sS -L -o "$tmp" -w '%{http_code}' "$url")"
		remote_bytes="$(file_size "$tmp")"
		rm -f "$tmp"
		if [ "$code" = "200" ] && [ "$remote_bytes" = "$local_bytes" ]; then
			printf '  OK   %s  HTTP %s  %s bytes\n' "$url" "$code" "$remote_bytes"
		else
			printf '  FAIL %s  HTTP %s  %s bytes (expected %s)\n' \
				"$url" "$code" "$remote_bytes" "$local_bytes"
			fail=1
		fi
	done
	if [ "$fail" -ne 0 ]; then
		exit 1
	fi
fi
