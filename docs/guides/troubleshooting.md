# Troubleshooting

## No PSP detected

- The console must sit in `Settings > USB Connection` with the memory stick
  inserted.
- Try another cable (charge-only cables mount nothing) and another port.
- On Linux, mounts under `/run/media/<you>/` need a desktop session; the
  CLI sees the same paths as the app.
- `godsend-psp devices` lists every mounted volume and flags the PSP ones.

## "does not look like a PSP memory stick"

The mounted volume has no `PSP` directory. PSP Go owners: make sure the
stick selection on the console matches the storage the USB mode exposes.

## ffmpeg missing

- macOS: `brew install ffmpeg`, or set the path in Settings.
- Linux: `sudo apt install ffmpeg`.
- Windows: winget install ffmpeg, then set the path in Settings.
- Without ffmpeg the app still installs homebrew, copies already-compatible
  files and converts ISO/CSO; only transcoding degrades.

## Conversion fails partway

The queue entry carries the ffmpeg error tail. Most cases:

- Source codec not decodable (corrupt file): try another source.
- Disk full on the stick: check the storage bar.
- Cancelled: the partial output is removed automatically.

## Install says "no zip asset"

The repo's latest GitHub release has no `.zip` asset (some projects only
attach archives in older releases). Use Refresh on the Homebrew page after
a new upstream release, or import a local zip.

## XMB shows corrupted/unknown files

Classic AppleDouble junk (`._*` files, `__MACOSX` folders) left by macOS
copies. Every install through this app prunes them; for existing junk run
the prune from any install once, or delete manually.

## Eject fails

Close other apps holding the volume (Finder window previews count), then
retry. On Windows use the tray's safe-eject; the app does not force-dismount
there.

## ARK Loader missing on the console

The loader installs to `PSP/GAME`; if the console shows nothing, confirm
the stick you installed to is the one in the console (multi-stick setups
and PSP Go internal storage are separate volumes).

## CLI cannot find the app data

The CLI and app share `~/Library/Application Support/godsend-psp` style
directories (per-platform). `godsend-psp doctor` prints the exact path.
