# Homebrew and custom firmware

## Requirements

Running unsigned code on a PSP needs custom firmware. The modern choice is
ARK-4 (or its successor ARK-5); classic CFWs such as PRO also work for
everything in this app except the ARK-specific install layout.

## Installing ARK-4 from the catalog

1. Open the **Homebrew** page, filter to CFW, pick **ARK-4**, press
   **Install**. The app downloads the latest release zip from GitHub,
   extracts it, and copies:
   - `ARK_01234` to `PSP/SAVEDATA/`
   - `ARK_Loader` (plus optional cIPL/DC/Full Flash installers) to
     `PSP/GAME/`
2. Eject the stick, exit USB mode on the console.
3. On the console: Game menu, Memory Stick, run **ARK Loader**. The console
   installs ARK modules to flash and reboots into the CFW (live mode).
4. To make it permanent, install **New cIPL** (ARK_cIPL also ships in the
   release zip and can be installed the same way from this app).

The Mods page has the same installers with an extra confirmation step and a
status chip once `ARK_01234` is on the stick.

## Installing homebrew from the catalog

The catalog ships entries for FrogGBA, TempGBA4PSP-mod and DaedalusX64.
Press **Install** and the app:

1. Fetches the latest GitHub release and picks a `.zip` asset.
2. Detects the archive layout:
   - a `PSP/` tree inside the zip merges at the stick root;
   - any folder containing an `EBOOT.PBP` copies to `PSP/GAME/<name>`;
   - ARK-style folders follow the CFW layout.
3. Prunes `__MACOSX`, `._*` AppleDouble files and `.DS_Store` from the
   stick so the XMB shows no corrupted entries.

Emulator notes appear on each card (for example FrogGBA wants
`gba_bios.bin` dropped into its game folder after install; BIOS files are
not distributed with this app).

## Importing any homebrew zip

Use **Import Zip** on the Homebrew page (or `godsend-psp install file.zip`
in the CLI). The same layout detection runs, so archives from brewology,
wololo or a friend's SD card all land correctly. When detection guesses
wrong, force a layout with the CLI:

```sh
godsend-psp install pack.zip --layout eboot
godsend-psp install cfw.zip --layout ark
```

## Plugins

Plugins are `.prx` modules the CFW loads at boot. The classic manager is
three text files in `seplugins/` on the stick:

- `game.txt` - PSP/UMD games
- `vsh.txt` - the XMB dashboard
- `pops.txt` - PS1 titles

Each line is `ms0:/seplugins/example.prx 1` where the trailing number is
the on/off flag. The **Mods > Plugins** page lists every entry per section
with a switch:

- Toggling rewrites only that line's flag; unrelated lines are untouched.
- **Add .prx** copies the file into `seplugins/` and registers it enabled.
- Missing files are flagged so stale entries are obvious.

Under ARK-4 the same files work as-is. ARK's own `PLUGINS.TXT` CSV and the
Custom Launcher installer are console-side features; this app manages the
classic layout.
