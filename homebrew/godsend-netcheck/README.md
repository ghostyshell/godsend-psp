# GODsend NetCheck

Shows the WLAN switch state, then connects the configured access point with
`sceNetApctlConnect` (trying connection slots 1 to 5) and polls until the PSP
holds an IP. The screen prints the assigned IP, subnet, gateway and DNS plus a
clear PASS or FAIL line with the reason.

Cross re-runs the check, Circle exits and disconnects. No data is transferred.

Build: `scripts/build-homebrew.sh`, then copy
`PSP/GAME/godsend-netcheck/EBOOT.PBP` to the stick.
