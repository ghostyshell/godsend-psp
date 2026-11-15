# Changelog

All notable changes to GODsend PSP. Newest first.


## [0.2.16] - 2026-11-15

### Changed

- The site page carries a canonical URL, Open Graph and Twitter cards, and
  SoftwareApplication structured data, plus a sitemap. Its
  favicon is now a 3.5 KB SVG mark instead of the 548 KB app icon, and the
  header mark is a 2.5 KB 64px copy. The header gained a real mobile menu,
  cards and screenshots rise in on scroll (scroll-driven CSS, no JavaScript),
  focus rings are visible, and the screenshot viewer dims the page behind it,
  locks scrolling, keeps Tab inside itself and returns focus on close.
  With JavaScript off the header shows the links inline instead of a menu
  that cannot open.


## [0.2.15] - 2026-11-15

### Added

- A loopback MCP server (JSON-RPC over HTTP on 127.0.0.1:8849) that starts
  with the desktop app and runs headless via `godsend-psp mcp`, exposing
  read-only device, library, store, downloads, plugins and doctor tools.
  Toggle it in Settings.


## [0.2.14] - 2026-11-15


## [0.2.13] - 2026-11-15


## [0.2.12] - 2026-11-15


## [0.2.11] - 2026-11-15


## [0.2.10] - 2026-11-15


## [0.2.9] - 2026-11-15


## [0.2.8] - 2026-11-15


## [0.2.7] - 2026-11-15


## [0.2.6] - 2026-11-15


## [0.2.5] - 2026-11-15


## [0.2.4] - 2026-11-15

### Fixed

- Installer DMG falls back to a plain `hdiutil create` when the Intel
  runner cannot convert the read-write image, so the Applications link and
  Read Me still ship.

### Fixed

- Publish polls the CI run and fetches each artifact archive individually
  with overwrite-tolerant extraction, so the blocking watch and the
  duplicate-entry Windows zip can no longer stall a release.

### Fixed

- Installer DMG attach uses an explicit mountpoint (the old field parse
  broke on the space in "GODsend PSP") and convert retries before failing.
- fetch-ffmpeg no longer renames the binary onto itself on non-Windows
  targets.

### Fixed

- CI no longer reuses stale ffmpeg binaries: the fetched arm64 pair had been
  committed (gitignore only covered `ffmpeg*`), which broke the x64 macOS
  and Linux jobs with "Bad CPU type". The binaries are untracked and the
  fetch script always replaces them.

### Fixed

- Publish downloads CI artifacts exactly once (a second extraction was
  colliding with the first and failing with "file exists").

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

- Covers and animations come straight from the game files: ICON0.PNG is
  preferred from the EBOOT folder, PBP section or UMD image, and the
  game's own ICON1.PMF animated icon is transcoded to a looping MP4 that
  plays over the cover on hover (the overlay owns the hover trigger, so
keyboard users get the CSS zoom instead). Remote index covers remain the
fallback.

### Added

- Video tiles play a cached three second muted clip of the film on hover or
  keyboard focus, extracted with ffmpeg at 240px wide, the way PSP video
  icons animate.

### Added

- Fixture hits carry the new rawgCover field so the core tests match the
  index shape.
- Connected-library covers resolve beyond ICON0: the app asks the configured
  R2 base first, then the game-search index (rawgCover) by title/serial and
  caches the result, so device games without icons still get real art.

### Changed

- Screenshot modal has a transparent backdrop, is centred, and carries zoom
  in/out buttons with a percentage readout plus a close button; Escape and
  clicks outside also close it.
- Mock covers load real art from `docs/demo-covers/<SERIAL>.png` when the
  files exist, so site captures can show actual game covers.

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
