# GODsend SysInfo

Read-only report: firmware version and build, model family, nickname, battery
percentage with charging state, free/used `ms0:` space, WLAN MAC address and
UMD drive state. Cross redraws the report, Circle exits.

User mode cannot read the numeric model id (01g..09g), so the report shows
`PSP Go (N1000)` when `ef0:` exists and `PSP-1000/2000/3000` otherwise.

Build: `scripts/build-homebrew.sh`, then copy
`PSP/GAME/godsend-sysinfo/EBOOT.PBP` to the stick.
