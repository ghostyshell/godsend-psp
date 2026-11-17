# Changelog

All notable changes to GODsend PSP. Newest first.


## [0.2.50] - 2026-11-17

### Changed

- Store rows show a compact container pill (capped at 110 px, full name in
  the tooltip) that expands on hover, so long collection names no longer
  dominate the row or crowd the Download button.
- The Minerva and r-roms Store indexes are prebuilt and served from
  `https://covers.debrid.in/psp/index/` (about 3,637 and 6,196 games), with a
  24 hour on-disk cache and the live build as fallback. Opening those tabs no
  longer downloads a torrent or crawls Internet Archive metadata.
- The update banner keeps Install and Dismiss on screen at the 1000 px
  minimum window width and sticks to the top of the scrolling workspace, so
  the action is still there after scrolling.

### Fixed

- Internet Archive metadata falls back to the plain HTTP endpoint when
  archive.org:443 is filtered, so the r-roms live build still completes on
  such networks.


## [0.2.49] - 2026-11-17

### Fixed

- Queue cards no longer overflow the 300 px inspector. A long game title
  could not shrink, so the Cancel button was pushed outside the card. The
  label now truncates and the button keeps its width.
- The download progress line (percent, downloaded of total, speed and time
  left) wraps instead of ending in an ellipsis, so every figure is visible.
  A UI test checks the cancel button stays inside the card and that neither
  the card nor the progress line is clipped.

## [0.2.48] - 2026-11-17

### Fixed

- Keys and Internet Archive details no longer disappear when the app is
  replaced. A save built from a defaulted config could overwrite
  `secrets.json` with empty values. Credential writes now keep any key that is
  already on disk unless the Settings patch explicitly clears that key.
- `config.json` and `secrets.json` are copied to `.bak` before every write, and
  a config that fails to parse is copied to `config.json.corrupt-<timestamp>`
  and noted in `config-errors.log` instead of being silently replaced by
  defaults on the next save.

## [0.2.47] - 2026-11-17

### Added

- The app checks the release feed a few seconds after launch and offers the
  build for the running platform in a dismissible banner under the topbar.
  Settings has an Updates panel with the current version, an on-demand check
  and an automatic-check toggle, on by default for existing configs.
- Release artifacts are mirrored to the project R2 bucket and the feed carries
  a direct `r2` URL plus an exact byte count for every build. The updater
  downloads from that mirror (falling back to file.kiwi, then GoFile), verifies
  the byte count and SHA-256, deletes a mismatch and then opens the verified
  installer: DMG on macOS, setup.exe on Windows, AppImage/deb on Linux.
- The publish script keeps the three newest release prefixes on R2, sorted by
  parsed version, and always keeps the version being published.

### Changed

- R2 retention is a standalone command now, `scripts/prune-r2-releases.sh`
  (`--keep`, `--prefix`, `--dry-run`), called by the publish script. Verified
  against a throwaway prefix with 0.2.9 through 0.2.12: keep 3 deleted 0.2.9
  and kept 0.2.10 to 0.2.12, proving the numeric version sort.

## [0.2.46] - 2026-11-17

### Fixed

- Game Debrid no longer loads empty. The service returns a `{"games": [...]}`
  envelope, and its rows omit the `id` field, so the parser saw zero games and
  would have rejected every row. The envelope is unwrapped (bare arrays still
  work) and the slug fills in a missing id.

### Changed

- Store opens on the Minerva tab by default.
- Minerva downloads check the debrid cache before touching the torrent. A
  cached file is resolved through Real-Debrid or TorBox and fetched as a direct
  link; an uncached file, or a failed cache check, goes straight to the torrent
  so nothing waits on a server-side debrid download.

## [0.2.45] - 2026-11-17

### Fixed

- The macOS window can be dragged by its header again. The bar relied on
  `-webkit-app-region`, which WKWebView ignores; the header and its title
  elements now carry `data-tauri-drag-region`, so Tauri's native drag handler
  works while buttons and chips stay clickable.

- "To PSP" now unpacks `.zip` downloads and routes them by content instead
  of copying the archive into `PSP/GAME`, where the XMB listed it as
  Corrupted Data. Minerva and r-roms deliver per-game zips, so this was the
  common case.
- A zip with a top-level `PSP/` folder merges that tree over the stick, so
  `PSP/GAME/<x>/EBOOT.PBP` and `PSP/SAVEDATA/...` land where the console
  expects them. A zip holding an `EBOOT.PBP` folder installs it under
  `PSP/GAME/<folder>`, a zip with an ISO or CSO copies each image into
  `ISO/`, and a zip holding a bare `.pbp` wraps it as
  `PSP/GAME/<stem>/EBOOT.PBP`.
- Direct `.iso`/`.cso` pushes still go to `ISO/`, and a direct `.pbp` push
  now installs as `PSP/GAME/<stem>/EBOOT.PBP` instead of a loose file.
- `.chd`, `.7z` and `.rar` now fail with an explanation instead of being
  copied blindly: CHD is not bootable on the PSP (convert it to ISO or CSO),
  and 7z/rar are not unpacked (extract first and push the ISO). Any other
  file type lists the supported ones.
- Zip entries that escape the extraction folder are rejected, and
  `__MACOSX`, `._*` and `.DS_Store` are skipped while extracting.
- The same routing serves the Downloads, Library and Files "To PSP"
  buttons.


## [0.2.44] - 2026-11-16

### Fixed

- Internet Archive username and password now save reliably. They previously
  wrote only on input blur, so typing and then closing the window (or never
  leaving the field) silently lost them. Both fields have an explicit Save
  button and Enter support, the password has a Clear action, and the stored
  values go to config.json and secrets.json like the debrid keys.
- Added a secrets round-trip test that writes and reads the Internet Archive
  email, password and the debrid keys through a temporary file.

### Note

- The Internet Archive username and password are stored locally and are not
  used for downloads yet. archive.org downloads authenticate with the S3
  access and secret keys; set those for items that need an account.

## [0.2.43] - 2026-11-16

### Fixed

- Torrent downloads no longer stall at 99% seeding. aria2 now runs with
  `--seed-time=0` and `--seed-ratio=0.0` and exits the moment the payload is
  complete, for both single-file torrent selection and whole-torrent
  downloads.
- Minerva downloads resolve the chosen game file through the connected
  Real-Debrid or TorBox account and fetch the direct link, instead of always
  pulling the whole set over P2P. The resolver adds the set magnet, matches
  the game by its path basename (falling back to the file index), selects
  that one file and unrestricts its link. Any debrid error falls back to the
  existing P2P path with the reason in the job message.
- The debrid magnet carries a URL-encoded display name, so set titles with
  spaces and parentheses resolve cleanly.

### Changed

- Downloads show real progress. aria2 summary lines are parsed into bytes
  done of total, live speed and time left, rendered as
  `78% - 1.2 GiB of 1.5 GiB - 3.2 MiB/s - 1:20 left` on the Downloads page
  and the queue drawer. Jobs that cannot report a live rate fall back to the
  average measured from elapsed time.


## [0.2.42] - 2026-11-16

### Fixed

- Downloads were launching ffmpeg as if it were aria2c whenever aria2c was
  not on the app's PATH. The aria2 lookup was handed `config.ffmpeg_path`,
  and a packaged macOS app does not inherit `/opt/homebrew/bin`. Resolution
  now checks bundled resources, PATH, then `/opt/homebrew/bin`,
  `/usr/local/bin`, `/usr/bin` and `/snap/bin`, and refuses any binary whose
  name is not aria2c.
- Single-file torrent downloads now fail with an install hint (brew or apt)
  when aria2c is genuinely missing, instead of a raw aria2c exit message.


## [0.2.41] - 2026-11-16

### Changed

- Store Minerva and r-roms now list individual games instead of whole
  collections. Both tabs show one flat, searchable list of titles with a
  per-game Download, so getting one game never means pulling a whole set.
- The Minerva index is built from the set torrent's file list (about 3 MB)
  and the r-roms index from Internet Archive item metadata. Each index is
  cached in memory for 10 minutes, so switching tabs and re-searching does
  not refetch anything. The list renders the first 300 matches and asks you
  to type to narrow it.
- New bencode reader in `core/src/torrent.rs` parses `.torrent` file lists
  with no new dependency (depth and 32 MB size caps). aria2 now takes
  `--select-file` (1-based, `--follow-torrent=mem`) so one file inside a
  torrent downloads on its own. Game Debrid, Internet Archive and
  NoPayStation are unchanged.

### Fixed

- Single-file Minerva downloads account for aria2 keeping the torrent's own
  folder (`<info.name>/...`), and the selected game is moved flat into the
  download folder, so it appears on the Downloads page instead of hiding in a
  nested set directory.


## [0.2.40] - 2026-11-16

### Fixed

- GODsend Cleaner now reads ms0 capacity correctly. `SCE_PR_GETDEV` takes a
  `SceDevctlCmd` holding a pointer to `SceDevInf` as input, not the struct as
  an output buffer, so the app claimed the stick was not mounted and never
  showed free or used space.
- The cleaner report no longer stops at a capacity error. It still lists the
  largest folders, and the Cross prompt only appears when something is
  actually reclaimable; with nothing to clean it says "Nothing to clean" and
  exits.
- The CLI `convert` command also chooses its direction from the file header
  now, not the extension, so a CSO named `.iso` decompresses instead of being
  refused.


## [0.2.39] - 2026-11-16

### Fixed

- CSO files created by the app were unreadable on the console: the header
  magic was written as `CSIO` instead of `CISO`. New conversions write the
  correct magic, and files with the old `CSIO` header still decompress.
- A CSO that is named `.iso` is now identified by its header, so the app
  offers "To ISO" instead of inviting a second compression. The core codec
  also refuses to compress a source that is already a CSO.
- Conversions write to a `.part` file and rename on success, so a cancelled
  or failed job can no longer leave a truncated CSO or ISO on the stick. The
  zero-index CSO that showed up as Corrupted Data in the XMB was exactly that
  failure mode.


## [0.2.38] - 2026-11-16

### Changed

- Homebrew art redone: ICON0 is now a transparent glyph that sits on the XMB
  wave instead of a pasted rectangle, and PIC1 is a designed indigo/cyan
  backdrop with the wave motif and a faint copy of the same glyph. Both come
  from one shared layer in `homebrew/common/make-art.py`, which gains a
  `--preview` contact sheet for checking the icons on the XMB backdrop.

### Fixed

- `scripts/build-homebrew.sh` retries a transient psp-gcc segmentation fault
  (it runs under amd64 emulation on Apple silicon) instead of failing the
  build outright.


## [0.2.37] - 2026-11-16

### Added

- The four GODsend utilities now carry XMB art: a 144x80 ICON0 cover and a
  480x272 PIC1 background packed into each EBOOT. `homebrew/common/make-art.py`
  regenerates them from the indigo-to-cyan family, and the zips on R2 were
  rebuilt and verified byte for byte.

### Fixed

- The ARK-5 migration plan no longer claims PSP-3000, Street and E1000 cannot
  install cIPL. ARK's New cIPL supports every PSP model; the plan now says to
  install DC10 first as the recovery path, then run CustomIPL once ARK boots
  reliably.
- `scripts/build-homebrew.sh --verify` cache-busts the public zip URLs, since
  an R2 overwrite can serve a stale object at the edge for a short window.


## [0.2.36] - 2026-11-16

### Added

- Theme Studio under Mods > Themes: build wallpaper packs with an accent
  colour and an optional wallpaper, install them to PSP/PICTURE, apply a pack
  as the app accent (persisted), and import or manage existing CXMB .ctf and
  Sony .ptf themes (both remain closed containers).
- Theme catalog expanded to 24 verified CTF and PTF themes, all hosted on R2
  and served from covers.debrid.in/psp/themes.json.
- Four PSP utilities built from source in this repo and hosted on R2, listed
  in Homebrew under All: GODsend Cleaner (space report and PSP/COMMON plus
  zero-byte / *.tmp cleanup, SAVEDATA untouched), GODsend Saves (save folder
  backup to PSP/SAVEDATA/_backups), GODsend SysInfo (firmware, battery,
  space, MAC and UMD report) and GODsend NetCheck (WLAN switch plus apctl
  connection, IP and gateway check). `scripts/build-homebrew.sh` rebuilds,
  zips, uploads and verifies them.

### Changed

- The ARK-5 migration plan is model aware: PSP-1000 and PSP-2000 (except
  TA-088v3) can install New cIPL, while PSP-3000, PSP Street and E1000 keep
  booting the ARK Loader once per boot, with DC10 installed first on
  cIPL-capable consoles.
- Homebrew has an Apps filter for the four GODsend utilities.
- `scripts/psp-migrate.sh` finishes the LME to ARK-5 move on a mounted stick
  (installs the utilities, retires the LME apps into PSP/GAME/_retired and
  disables leda.prx), dry run by default. `--purge` archives the retired LME
  apps and SEPLUGINS to ~/Backups/godsend-psp, verifies the zip, then removes
  them.


## [0.2.35] - 2026-11-16

### Added

- Dedicated Jailbreak section with CFW detection and the ARK-5 migration
  plan, plus ARK-4 and ARK-5 (FasterARK) install cards.


## [0.2.34] - 2026-11-16

- Jailbreak page: a dedicated CFW section (separate from Homebrew) with stick
  detection for ARK-4, ARK-5, PRO, ME, Time Machine, recovery and the ISO
  driver plus the plugin count, ARK-4 and ARK-5 (FasterARK) install cards, and
  an ordered ARK-5 migration plan that warns New cIPL is permanent.


## [0.2.33] - 2026-11-16


## [0.2.32] - 2026-11-16


## [0.2.31] - 2026-11-16


## [0.2.30] - 2026-11-16


## [0.2.29] - 2026-11-16


## [0.2.28] - 2026-11-16


## [0.2.27] - 2026-11-16


## [0.2.26] - 2026-11-15


## [0.2.25] - 2026-11-15

### Added

- ARK-5 as FasterARK (PSP-Arkfive/FasterARK): the catalog prefers the
  ARK_UPDATE.zip asset and installs its PSP tree, with on-console cIPL
  steps in the note. Release assets can now name a preferred zip.

### Added

- First real theme catalog: three CTF XMB themes scraped from
  caseyjkey/psp-xmb-themes into psp-catalog/files and published to
  psp-mods.debrid.in for the app's Themes tab.

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

- Themes and mods are served from R2 by default
  (https://covers.debrid.in/psp/...), with psp-mods.debrid.in kept as the
  mirror. The sync script pushes the catalog to both.

### Changed

- Library merges the Images section into a Games/Homebrew split, each with
  PSP and Local sides. ISO/CSO entries appear under Games with their
  import, convert and delete actions (the separate Images nav entry is
  gone, those features moved here).

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

- UI smoke test follows the Library Games/Homebrew split instead of a fixed
  tile count.

### Fixed

- The automatic release job resolves gh and friends under launchd by
  setting an explicit PATH.

### Fixed

- Store source points at the real production index:
  https://game-debrid.link/api. Cover resolution prefers the index RAWG art
  and then the R2 mirror using the index slug.

### Fixed

- Library image rows restore the reload callback used after deleting an
  image.

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
