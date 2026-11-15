# Changelog

All notable changes to GODsend PSP. Newest first.

## [0.2.1] - 2026-11-15

### Added

- Version-bump and changelog enforcement: `scripts/bump-version.sh` syncs
  every version location, `scripts/check-changelog.sh` runs from the
  pre-commit hook, and `docs/agents/skills/changelog.md` documents the rule.
  The changelog no longer uses an Unreleased section; each change lands
  under a versioned, dated header.

### Changed

- New app and site icon: the cyan download-arrow mark (assets/logo.svg),
  regenerated across the Tauri icon set and the Pages site.
- scripts/render-logo.mjs renders the vector mark to icon.png via Playwright.

### Fixed

- Publish script normalizes CI artifact names to `godsend-psp-<version>-...`
  before upload, so the README and site table carry consistent versioned
  filenames for every platform.
- GitHub Releases and tags removed; CI builds only on manual dispatch and
  the publish script dispatches its own build, so nothing recreates a
  release.

- macOS DMGs are ad-hoc signed as a bundle before packaging. Previously the
  app carried only a linker-level signature, so Gatekeeper reported "damaged"
  on Apple Silicon. CI now signs the bundle and verifies it before the DMG
  is created; both published DMGs were re-signed and re-uploaded.

### Changed

- Releases now publish to GoFile (primary) and file.kiwi (backup) like the
  sibling projects; the README table is rewritten from the upload manifest.
  GitHub Releases are no longer created (tags still trigger CI builds).
- scripts/publish-release.sh, refresh-release-links.sh and
  install-refresh-job.sh mirror the keepsake-atelier flow (weekly Sunday
  refresh via launchd).

### Added

- Guides under docs/guides: connecting the PSP, homebrew and CFW, music and
  video, disc images, saves and mods, troubleshooting, building from source.
- CI release job: pushing a v* tag builds all platforms and attaches the
  artifacts to a GitHub Release.
- README download table and guide index.

## 0.2.0 - 2026-11-15

### Added

- Video guides: `docs/video-guides.html` lists twelve short recordings, one per tab, with a walkthrough and steps each. Videos are on R2, the page is linked from the site and the README. Recorded from the renderer's `?mock=1` bridge with the demo-reels skill.
- Store page with two sources: the game-search debrid index (platform=psp,
  ddl/magnet/torrent links with supportedBy) and Internet Archive sets in
  the r/Roms megathread style. Per-game link list, mods, and one-click
  downloads.
- aria2c downloads (16 connections) on every platform with a single
  connection ureq fallback; configurable download folder and a Downloads
  page (queue, open, delete, push to PSP).
- Real-Debrid and TorBox clients: unrestrict DDL links and resolve
  torrent/magnet links into direct downloads. Keys live in Settings and are
  never sent anywhere but the provider APIs.
- File manager (local vs stick panes) with copy in both directions, new
  folder, delete; Local Library and PSP Library tabs with copy between
  them.
- PPSSPP-style local game database (serial keyed JSON), R2 cover and
  animation URLs with disk cache, per-game mods catalog with pull-to-download.
- Settings: debrid keys, source URL, R2 covers base, mods base, download
  folder.

### Changed

- Library nav entry replaces the old Games page; Homebrew moved down the
  rail.

## 0.1.0 - 2026-11-15

### Added

- First working build. Rust engine (godsend-core) with volume detection,
  PARAM.SFO and PBP parsing, ISO9660/UMD analysis, ISO <-> CSO conversion,
  job queue with progress events, ffmpeg PSP presets, homebrew catalog and
  installer with layout detection, seplugins manager, theme installer,
  savedata backup/restore, and doctor checks.
- Tauri 2 desktop app: 9 section UI (Console, Games, Homebrew, Music, Video,
  Images, Saves, Mods, Settings) in the shared atelier design language with
  an indigo->cyan laser spectrum, inspector queue, device watcher events,
  and a full mock bridge (?mock=1) for hardware-free development.
- Headless CLI (godsend-psp) covering the whole engine surface.
- Playwright UI test scaffold and GitHub Actions build matrix (macOS arm64 +
  x64, Linux AppImage/deb/tarball, Windows NSIS + portable CLI).
