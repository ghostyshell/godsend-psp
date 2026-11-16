# GODsend homebrew apps

Four small user-mode PSP utilities, built here from source with PSPSDK and
shipped as EBOOT.PBP archives:

| id | app | what it does |
|----|-----|--------------|
| `godsend-cleaner` | GODsend Cleaner | storage report and PSP/COMMON + junk cleanup |
| `godsend-saves` | GODsend Saves | save folder backup to `PSP/SAVEDATA/_backups` |
| `godsend-sysinfo` | GODsend SysInfo | firmware, battery, space, MAC, UMD report |
| `godsend-netcheck` | GODsend NetCheck | WLAN switch, apctl connect, IP/gateway report |

All four are plain user-mode PRXs packed into an EBOOT, so they launch from
the XMB under Game > Memory Stick with any loader (ARK-4 or similar). They
draw with the PSPSDK debug screen and use the d-pad, Cross and Circle.

## Build

```sh
scripts/build-homebrew.sh              # build + zip into homebrew/out/
scripts/build-homebrew.sh --upload     # also push to R2
scripts/build-homebrew.sh --verify     # curl the public zips and check them
```

The build runs `psp-gcc` inside the `pspdev/pspdev:latest` Docker image, so
no local PSP toolchain is needed. Each zip contains exactly
`PSP/GAME/<id>/EBOOT.PBP`.

## Source layout

- `common/gs_ui.c` - debug screen banner, colours, footer, button waits.
- `common/gs_fs.c` - sceIo tree walking: sizes, recursive copy, mkdir -p,
  ms0: free space.
- `<id>/main.c` - the app itself.
- `<id>/Makefile` - PSPSDK build with `BUILD_PRX = 1`, `PSP_EBOOT_TITLE`.

## Notes

- No kernel modules, no plugins, no seplugins files.
- Cleaner and Saves never delete anything under `PSP/SAVEDATA`; Cleaner only
  removes files below `PSP/COMMON` plus zero-byte and `*.tmp` leftovers, and
  Saves only writes new folders inside `PSP/SAVEDATA/_backups`.
- SysInfo runs in user mode, where the numeric model id (01g..09g) is not
  exposed. It reports the PSP Go when `ef0:` exists and the 1000/2000/3000
  family otherwise.
