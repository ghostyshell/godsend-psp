# GODsend PSP: project overview for agents

GODsend PSP is a desktop workshop for the PSP: plug the console in over USB
mass storage and manage the memory stick from one window. It shares its code
stack and design family with Keepsake Atelier (Rust workspace + Tauri 2) and
offers GODsend 360 style features for the PSP.

## Components

- **`core/` (`godsend-core`)**: framework-agnostic engine. No Tauri, no
  clap. Modules: `device` (volume scan + PspPaths), `sfo` (PARAM.SFO),
  `pbp` (EBOOT.PBP sections), `iso` (ISO9660 walk, UMD analysis), `cso`
  (ISO <-> CSO v1 codec), `library` (game/image scan + icon cache),
  `transfer` (JobManager with progress hook + cancel + journal), `media`
  (ffmpeg discovery + PSP presets), `homebrew` (catalog, GitHub releases,
  layout-aware zip installer), `plugins` (seplugins), `themes`,
  `savedata`, `doctor`, `config`.
- **`cli/` (`godsend-psp`)**: headless CLI over core (clap 4, one command
  per feature area).
- **`app/`**: Tauri 2 desktop app. `app/src-tauri/src/lib.rs` holds ~40
  `#[tauri::command]`s and the device watcher; `app/src` is React 18 +
  TypeScript with one hand-rolled `styles.css` (see DESIGN.md) and a full
  mock bridge in `mock.ts` behind `?mock=1`.

## Contracts and invariants

- The engine never assumes the console itself is addressable; everything is
  plain filesystem work on the mounted stick. Keep `core/` free of UI and
  process-framework dependencies.
- PARAM.SFO values: fmt 0x0004/0x0404 UTF-8, 0x0204 int32, 16-byte index
  rows, key table at `key_table_start`. PBP header is magic + version + 8
  u32 offsets (36-40 bytes, tolerate both).
- CSO v1: 24-byte header, `(blocks+1)` u32 index with compressed flag in
  bit 31 and absolute file offsets in `1 << align` units, 2048-byte frames,
  zlib.
- Plugin lines are `ms0:/path 1|0`; toggling rewrites only the flag. ARK
  also accepts CSV PLUGINS.TXT but we manage the classic files.
- The seplugins toggles must never reformat or reorder unrelated lines.
- Savedata operations must skip `ARK_01234` (CFW payload, not a save).
- Installs prune `__MACOSX`, `._*` and `.DS_Store` from the stick (ARK-4
  ships the same fix on-console; do it at the source).
- **Releases are not done until the live feed proves it.** After running
  `scripts/publish-release.sh`, confirm
  `https://ghostyshell.github.io/godsend-psp/downloads.json` (cache-busted)
  reports the new version. The script performs this check itself; never
  declare a release complete from the local files alone. Site pages must
  fetch the feed with a cache-busting query, because Pages serves it with a
  600s cache.
- Destructive operations (game/image/save/theme delete, CFW install) need
  an explicit user confirmation in the UI.

## Building and testing

```sh
cargo test --workspace                  # unit tests (sfo/pbp/cso/plugins/saves)
cargo check --workspace                 # zero warnings expected
npm run --prefix app build              # tsc --noEmit + vite build
npm run --prefix app dev:mock           # browser UI without hardware
```

Icons: `python3 scripts/make-icon.py` then `npx --prefix app tauri icon
app/src-tauri/icons/icon.png` (only when the mark changes).

## Conventions

- Public-facing copy: no em dashes, no en dashes, ASCII hyphen only.
- **Version bump + changelog, every change.** Any edit to product code,
  config or user-visible copy bumps the version and adds a CHANGELOG entry
  in the same commit. No `[Unreleased]` section: newest `## [x.y.z] -
  YYYY-MM-DD` header at the top. Use `scripts/bump-version.sh x.y.z "Fixed:
  ..."` and read `docs/agents/skills/changelog.md`. The pre-commit hook
  (`scripts/install-hooks.sh`) fails on product changes without a changelog
  diff or on version mismatches.
- Frontend keeps types in `app/src/types.ts` in sync with core serializers;
  the mock bridge fixtures must mirror real shapes.
- Errors: core returns `godsend_core::Result<T>`; the Tauri error type
  serializes as the Display string.

## Demo videos and guides - demo-reels

Skill: `docs/agents/skills/demo-reels.md` (global copy `~/.claude/skills/demo-reels/SKILL.md`, tooling `~/Code/demo-reels`). Desktop app, recorded from the renderer's `?mock=1` bridge. Guides data `docs/guides.json`, page `docs/video-guides.html`. Any user-visible change (new, changed or removed feature) updates its video and guides entry in the same change: new = new reel plus a guides entry, changed = re-record, removed = delete both. Then `node publish.mjs godsendpsp` and `node ~/Code/demo-reels/build-site.mjs godsendpsp` and add a CHANGELOG line.
