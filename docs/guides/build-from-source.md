# Build from source

## Toolchain

- Rust 1.85 or newer (edition 2021, workspace with three crates).
- Node 20+ with npm.
- Tauri 2 prerequisites:
  - macOS: Xcode Command Line Tools.
  - Windows: MSVC build tools and WebView2.
  - Linux: `libwebkit2gtk-4.1-dev libgtk-3-dev` (plus `build-essential`,
    `curl`, `wget`, `file`, `libxdo-dev`, `libssl-dev`, `libayatana-appindicator3-dev`,
    `librsvg2-dev` per the Tauri docs).

## One-time setup

```sh
git clone https://github.com/ghostyshell/godsend-psp
cd godsend-psp
npm install --prefix app
```

## Desktop app

```sh
# dev with hot reload (needs a real PSP or use the mock UI in a browser)
npm run --prefix app tauri dev

# release bundles for this platform
npm run --prefix app tauri build
# macOS: .app + .dmg in app/src-tauri/target/release/bundle/
# Windows: NSIS setup exe in bundle/nsis/
# Linux: .deb + .AppImage in bundle/
```

## Mock mode (no hardware)

The full UI runs in a plain browser against a fixture memory stick:

```sh
npm run --prefix app dev:mock     # opens http://localhost:1420/?mock=1
```

The bridge is in `app/src/mock.ts` and is dropped from release builds.

## CLI

```sh
cargo build --release -p godsend-psp-cli
./target/release/godsend-psp doctor
```

The CLI is a single self-contained binary; it uses the system ffmpeg when
conversion is needed.

## MCP

The desktop app serves a read-only Model Context Protocol endpoint on
`http://127.0.0.1:8849` while it runs. The endpoint is toggled in Settings
(`mcp_enabled`) and the port comes from `mcp_port` in the config file. For a
headless endpoint without the app:

```sh
./target/release/godsend-psp mcp
```

Clients POST JSON-RPC 2.0 to the endpoint; `tools/list` names the available
read-only tools.

## Tests

```sh
cargo test --workspace                 # 21 unit tests: sfo, pbp, cso,
                                       # plugins, saves, jobs, names
cd app && npx playwright test          # UI over the mock bridge
```

## Icons

The icon set is committed under `app/src-tauri/icons/`. To change the mark,
edit `scripts/make-icon.py`, then:

```sh
python3 scripts/make-icon.py
npx --prefix app tauri icon app/src-tauri/icons/icon.png
```

## Release flow

Push a tag `v*` and GitHub Actions builds every platform and attaches the
artifacts to a GitHub Release:

- macOS arm64 + x64: DMG and CLI tarballs
- Linux x64: AppImage, deb, CLI tarball
- Windows x64: NSIS setup exe and CLI zip
- A UI check job runs the Playwright suite on every tag and manual run

The upload script for mirror hosts (GoFile) lives at
`scripts/publish-release.sh` and takes the same artifact files.
