# GODsend Cleaner

Reports free and used space on `ms0:`, the largest folders under `PSP/`
(SAVEDATA, GAME, PICTURE, MUSIC, VIDEO, COMMON) and what a cleanup would
reclaim: every file below `PSP/COMMON` plus zero-byte and `*.tmp` leftovers
anywhere under `PSP/`.

- Cross cleans after a confirmation prompt, Circle exits.
- `SAVEDATA` contents are never scanned for deletion.
- Only files are removed; folder structure is left alone.

Build: `scripts/build-homebrew.sh`, then copy
`PSP/GAME/godsend-cleaner/EBOOT.PBP` to the stick.
