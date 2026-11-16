# GODsend Saves

Lists the folders in `ms0:/PSP/SAVEDATA` with their sizes. Up/Down moves the
selection, Cross copies the selected folder to
`ms0:/PSP/SAVEDATA/_backups/<name>-<YYYYmmdd-HHMMSS>` with a plain recursive
copy (no zip), then shows a success or failure line with the file count and
bytes written. Circle exits.

`ARK_01234` (CFW payload) and the `_backups` folder itself are skipped.

Build: `scripts/build-homebrew.sh`, then copy
`PSP/GAME/godsend-saves/EBOOT.PBP` to the stick.
