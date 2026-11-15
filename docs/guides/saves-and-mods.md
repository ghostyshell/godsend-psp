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

## Categories (optional)

With a large library, Game Categories Lite (a VSH plugin) lets the console
group games into folders. Install the plugin through the Plugins page,
create folders named `CAT_<name>` inside `PSP/GAME`, and move game folders
inside them. The app's library scan follows whatever folder layout you
use, including categories.
