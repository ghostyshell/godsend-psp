# Feature reference

## Device

- Volumes are discovered per OS (macOS /Volumes, Windows drive letters,
  Linux /media and /run/media). A volume is a PSP when its root has a PSP
  directory. The watcher thread polls every 2.5s and emits `device://changed`
  on mount/unmount/space changes.
- Safe eject: `diskutil eject` on macOS, `umount` on Linux, system tray on
  Windows.

## Game library

- PSP/GAME folders with EBOOT.PBP: metadata from the sibling PARAM.SFO or
  from the PBP section; kind classified as digital (valid serial id like
  UCUS-98737), homebrew, or loader (ARK_* folders).
- ISO folder: .iso analyzed via ISO9660 (PSP_GAME/PARAM.SFO for title and
  serial), .cso probed via header, .pbp listed.
- ICON0.PNG covers are extracted once (folder file, PBP section, or ISO
  tree) into the app cache and served as data URLs.

## Homebrew installer

Catalog entries point at GitHub repos; the installer fetches the latest
release, picks the first .zip asset, extracts to staging, detects the
layout, then installs:

- `ARK_01234` and friends to PSP/SAVEDATA; ARK loaders/cIPL/DC/UPDATE
  folders to PSP/GAME (ARK layout).
- Archives containing a PSP/ tree merge PSP/GAME, PSP/SAVEDATA, PSP/MUSIC,
  PSP/VIDEO, PSP/COMMON, PSP/SYSTEM at the stick root (psp_root layout).
- Any folder containing an EBOOT.PBP is copied to PSP/GAME/<name>
  (eboot layout).

`__MACOSX`, `._*` AppleDouble files and `.DS_Store` are pruned from the
whole stick after every install. Local zip imports use the same detection.

## Media

- Music to PSP/MUSIC: convert to MP3 192k or M4A (AAC) via ffmpeg, or copy
  when already compatible or when ffmpeg is missing. FAT-safe names.
- Video to PSP/VIDEO: convert to AVC 480x272 MP4 (level 3.0, faststart),
  optional 160x120 .THM thumbnail. Progress comes from `ffmpeg -progress`
  parsed against ffprobe duration.
- Live progress flows through the job manager to `job://progress` events.

## Disc images

- Pure Rust CSO v1 codec: ISO to CSO (zlib per 2048-byte block, levels
  1-9, stored-raw fallback per block) and CSO to ISO, both cancellable with
  progress. Index entries carry absolute file offsets, compressed flag in
  bit 31, align byte honored on decode.
- Import ISO with on-the-fly compression to save stick space.

## Save data

- PSP/SAVEDATA listing with title/subtitle/serial from each PARAM.SFO.
  ARK_01234 is never listed, backed up, or deleted.
- Backup to zip (per-file progress), restore with zip-slip guards.

## Mods

- seplugins manager: game.txt / vsh.txt / pops.txt parsed per line
  (`ms0:/path 1|0`), toggles rewrite only the flag, add copies a .prx into
  seplugins and registers it enabled.
- Themes: .ptf / .ctf install into PSP/THEME with path-escape guards.
- CFW: ARK-4 and ARK-5 from the catalog with a confirmation step.

## Jobs

Every long operation is a job: id, kind, label, state, done/total bytes,
message, timestamps. Jobs run on their own threads, report progress through
a process-wide hook (the Tauri layer turns it into events), and can be
cancelled cooperatively. The last 50 finished jobs persist to jobs.json.

## CLI parity

The `godsend-psp` binary wraps the same engine: devices, paths, scan,
game metadata, install, catalog, catalog-install, music, video, iso-import,
convert, saves (backup/restore/delete), plugins (list/toggle/add/remove),
themes, doctor.

## Mock mode

`?mock=1` installs a fixture bridge (window.__TAURI_INTERNALS__ shim) with
a full fake stick: games, images, saves, plugins, themes, catalog, jobs
that simulate progress. The desktop app UI is fully explorable without
hardware; `npm run dev:mock`.
