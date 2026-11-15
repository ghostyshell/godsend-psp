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
- Every product change updates CHANGELOG.md (newest first, Keep a
  Changelog-ish sections Added/Changed/Fixed).
- Frontend keeps types in `app/src/types.ts` in sync with core serializers;
  the mock bridge fixtures must mirror real shapes.
- Errors: core returns `godsend_core::Result<T>`; the Tauri error type
  serializes as the Display string.
