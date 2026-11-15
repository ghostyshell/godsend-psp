# Changelog

All notable changes to GODsend PSP. Newest first.


## [0.2.27] - 2026-11-16


## [0.2.26] - 2026-11-15


## [0.2.25] - 2026-11-15

### Added

- Automatic releases: scripts/auto-release.sh publishes whenever the repo
  version differs from the live downloads feed (all platforms through the
  workflow, then upload to GoFile + file.kiwi). install-auto-release-job.sh
  runs it every 30 minutes via launchd.

### Added

- Store sources for r-roms PSP links (archive.org sets plus direct archives,
  with optional Internet Archive S3 keys) and NoPayStation PSP packages with
  a zRIF sidecar for pkg2zip.
- Hosted PSP catalog pipeline: a validating sync script with `--dry-run`, a
  Playwright scraper driven by `psp-catalog/sources.json`, and a daily launchd
  job that scrapes and syncs at 04:12.

### Changed

- The saves and mods guide describes the hosted catalog layout, the sync and
  scrape scripts, and how the app consumes remote mods and themes.


## [0.2.24] - 2026-11-15


## [0.2.23] - 2026-11-15


## [0.2.22] - 2026-11-15


## [0.2.21] - 2026-11-15


## [0.2.20] - 2026-11-15


## [0.2.19] - 2026-11-15


## [0.2.18] - 2026-11-15


## [0.2.17] - 2026-11-15


## [0.2.16] - 2026-11-15

### Changed

- Each save slot now shows its title under the icon, the way the console
  lists saves, clamped to two lines.

### Changed

- Save Data shows each slot's own ICON0.PNG as a landscape thumbnail, the
  way the PSP save utility does, with a cover-card fallback when a save has
  no icon.

### Changed

- Internet Archive search is PSP-anchored (query + title/identifier
  post-filter) so results no longer include unrelated software, with
  megathread-style seed queries.
- Minerva is a Store source: it probes the Myrient PSP torrent sets and
  downloads the chosen torrent for aria2c.

### Changed

- Publish builds what the host can produce: macOS arm64 and x64 installers
  plus CLI tarballs, the Windows CLI via mingw cross-compilation, and the
  Linux AppImage/deb/CLI through the Tauri Docker image when the daemon is
  running. Missing toolchains are reported and skipped; artifacts staged in
  dist/ are still picked up.

### Changed

- Defaults are now maximum quality: x264 **veryslow**, CRF **16**, AAC
  **256 kbps**, 480x272 at 30 fps. Preset, CRF and audio are selectable in
  Settings; slow mediums trade speed for size.

### Changed

- Store sources ship production defaults: `covers_base` is
  https://covers.debrid.in (R2 covers, `<base>/games/<slug>.<ext>`) and
  `mods_base` is the project site, which now serves `psp/covers/<SERIAL>.png`
  and `psp/mods/<SERIAL>.json`. Both can be cleared in Settings.

### Changed

- Releases are built and uploaded by the working session, not GitHub
  Actions. `publish-release.sh` builds the macOS app, installer DMG and CLI
  locally, includes any cross-platform artifacts staged in `dist/`, and
  uploads to GoFile + file.kiwi. The workflow remains for optional manual
  cross-platform builds only.

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

- Hosted theme catalog: Mods > Themes lists `<mods_base>/psp/themes.json`
  and installs a chosen `.ptf`/`.ctf` straight into PSP/THEME. The default
  base is now https://psp-mods.debrid.in.

### Added

- Video transfer has a Browse Videos flow: pick files, review the staged
  list, then push. Encoding is configurable in Settings with maximum-quality
  defaults: CRF 18, 480x272, 30 fps, AAC 192 kbps (plus 368x208 and keep
  source options).

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

- Store source default points at production: https://debrid.link/api.
- Internet Archive sign-in takes a username and password (plus the optional
  S3 key pair), and all credentials are mirrored to secrets.json (0600) so
  installing a new build no longer loses Real-Debrid, TorBox or IA accounts.

### Fixed

- In-flight transfers no longer appear as library or file-manager entries:
  `.aria2`, `.part`, `.partial`, `.tmp` and `.crdownload` files (and any
  file with a live aria2 control file beside it) are filtered out.
- Queue rows show the percentage and an ETA, with the file name wrapped to
  two lines instead of being truncated to one.

### Fixed

- Save Data titles: saves whose PARAM.SFO has no TITLE now show a formatted
  serial (ULES-00309) once instead of repeating the folder id three times.

### Fixed

- Publish commits both the README table and docs/downloads.json again (the
  site feed was left uncommitted, so Pages kept serving the old version).

### Fixed

- Homebrew install badges were wrong: the installed check joined an empty
  folder name, so any existing PSP/GAME counted as installed. It now scans
  for a real EBOOT folder matching the catalog name (folder or PARAM.SFO
  title), and an empty GAME directory reports nothing installed.

### Fixed

- Store command routing: the Minerva detail arm no longer leaks into the
  download handler, and the client type includes the Minerva source.

### Fixed

- Config default test follows the new veryslow preset.
- The unused `video_preset` profile id was folded into the encoder preset
  field instead of duplicating the key.

### Fixed

- Publish writes the release version into downloads.json from the build
  version (not from the first artifact name), and stale artifacts in dist/
  are no longer picked up.

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
