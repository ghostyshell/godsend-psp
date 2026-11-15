# Saves and mods

## Save data

Save folders live in `PSP/SAVEDATA/<NAME>/`. The app lists each with its
game title and save slot description read from the folder's own PARAM.SFO,
plus size and icon.

- **Backup All** zips every save into the app data folder
  (`saves-backup-<timestamp>.zip`).
- Select specific saves with the checkboxes and **Backup Selected**.
- **Restore Zip** unpacks a backup back onto the stick (path-traversal
  guarded).
- Delete removes a save permanently; the button asks first.

ARK-4's `ARK_01234` folder is a CFW payload, not a save: it is hidden from
this page and refused by delete.

```sh
godsend-psp saves                                   # list
godsend-psp saves backup --out saves.zip UCUS98737DATA00
godsend-psp saves restore saves.zip
```

## Themes

XMB themes live in `PSP/THEME`:

- `.ptf` official themes, work on any firmware.
- `.ctf` custom themes need the CXMB plugin enabled for VSH; install it
  from the Plugins tab or write it into `vsh.txt` yourself.

The Mods > Themes page installs files (copy into THEME), lists them and
deletes them. Apply themes on the console under `Settings > Theme
Settings`.

## Hosted catalog

The app reads a hosted catalog from the base URL in Settings (`Mods catalog
base`, default `https://psp-mods.debrid.in`). Layout:

    psp/themes.json          [{ name, url, format, size_bytes }]
    psp/mods/<SERIAL>.json   [{ name, author, version, url, size_bytes, notes, kind }]
    psp/files/<name>         the theme and mod binaries

The source tree is `psp-catalog/` at the repo root. `themes.json` and
`mods/<SERIAL>.json` point into `psp-catalog/files/`.

- `scripts/sync-psp-catalog.sh` checks that both JSON shapes parse and that
  every referenced file exists, prints a summary, then uploads the tree to R2
  with rclone or the aws CLI. `--dry-run` validates and prints the planned
  upload command without sending anything.
- `scripts/scrape-psp-catalog.mjs` reads `psp-catalog/sources.json` (start
  from `sources.example.json`), visits each source with Playwright, downloads
  the `.ctf` / `.ptf` files it finds and rewrites `themes.json`. Files are
  deduplicated by SHA-256, so a repeated run only adds new binaries. With no
  sources it prints `no sources configured` and exits without any network
  access.
- `scripts/install-psp-catalog-job.sh` installs a launchd agent that runs the
  scraper and the sync daily at 04:12 under `caffeinate`, logging to
  `.psp-catalog.log`.

Games > Store detail and Library > Info call `game_info`, which returns the
remote mods array for the serial even when the local catalog has no match, so
the mods section shows the hosted entries and each one has a Pull button.
Downloads land in `mods/<SERIAL>/` under the downloads folder. Themes >
Hosted themes reads `themes.json` from the same base URL and installs into
`PSP/THEME`.


## Categories (optional)

With a large library, Game Categories Lite (a VSH plugin) lets the console
group games into folders. Install the plugin through the Plugins page,
create folders named `CAT_<name>` inside `PSP/GAME`, and move game folders
inside them. The app's library scan follows whatever folder layout you
use, including categories.
