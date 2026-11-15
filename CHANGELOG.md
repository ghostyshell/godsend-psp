# Changelog

All notable changes to GODsend PSP. Newest first.


## [0.2.10] - 2026-11-15


## [0.2.9] - 2026-11-15


## [0.2.8] - 2026-11-15


## [0.2.7] - 2026-11-15


## [0.2.6] - 2026-11-15


## [0.2.5] - 2026-11-15


## [0.2.4] - 2026-11-15

### Fixed

- Publish clears the artifacts directory before downloading, so a retried
  run no longer fails with "file exists" while extracting the CI archive.

### Fixed

- Installer DMGs now carry an Applications drop target, a Read Me with
  first-launch steps, and a background with arranged icons
  (`scripts/make-dmg.sh`), instead of a bare app bundle.

### Fixed

- CI fetches the bundled ffmpeg pair before unit tests; the resource config
  previously made the Tauri build script fail during `cargo test` on all
  three platforms, which blocked the 0.2.7 publish.

### Added

- Video tiles play a cached three second muted clip of the film on hover or
  keyboard focus, extracted with ffmpeg at 240px wide, the way PSP video
  icons animate.

### Changed

- Site screenshots are zoomable: clicking any capture opens a full-screen
  lightbox (click, Close or Escape to dismiss), and the strip now shows the
  console, library and store.

### Changed

- Library tiles are landscape (9:5, matching PSP ICON0 art) with a PSP XMB
  style focus: lift, laser rim, cover zoom and a light sweep on hover or
  keyboard focus. Entries without art render a real cover card (gradient,
  title, serial) instead of a monogram placeholder. Mock covers are
  landscape too. Version 0.2.8.

### Changed

- Ship checklist now includes screenshots and a full publish: every change
  regenerates the site captures (`app/scripts/capture-screens.mjs`), builds
  through `scripts/publish-release.sh`, uploads to GoFile + file.kiwi and
  verifies the live feed. Captures refreshed for the new nav, library and
  store pages. Version 0.2.7.

### Changed

- Logo assets are PNG everywhere: the app topbar, window icon set and site
  (header, footer, downloads page) use the derived PNG; the SVG mark and the
  SVG social card are gone, replaced by a rendered og.png.

### Changed

- App and site icon now derive from the supplied glossy emblem
  (`scripts/iconize-source.mjs`: white background removed, art cropped
  square and centered at 1024, then the full Tauri icon set regenerated).

### Changed

- library covers keep their full aspect, the rail uses real icons, and the PSP tab lists ISO/CSO images from the stick


## [0.2.3] - 2026-11-15

### Changed

- refreshed the README and site screenshots and re-recorded all twelve video guides with the new logo

## [0.2.2] - 2026-11-15

### Fixed

- Site download table served stale links: the page now cache-busts downloads.json, the publish script verifies the live feed, and the release rule requires that check.

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
