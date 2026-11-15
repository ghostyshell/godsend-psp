# Connect your PSP

## What the app needs

A PSP in USB mode is a plain USB mass storage device. No drivers, no
libusb, no custom services: the app only ever touches the mounted volume's
filesystem. Any model works (PSP-1000 through PSP-3000, PSP Go with its
Memory Stick or internal storage, and E1000 street).

## Steps

1. Put the console into Settings, then USB Connection, and confirm.
2. The memory stick mounts within a few seconds:
   - macOS: `/Volumes/MEMORY STICK`
   - Windows: a new drive letter such as `E:\`
   - Linux: `/media/<you>/MEMORY STICK` or `/run/media/<you>/...`
3. GODsend PSP detects the stick automatically (a `PSP Connected` chip
   appears in the top bar). Detection re-runs every 2.5 seconds, so you can
   plug and unplug without restarting the app.

If more than one PSP-style volume is mounted, open Settings, pick the
active one, or use `godsend-psp devices` in the CLI.

## What detection looks for

A volume counts as a PSP when its root contains a `PSP` directory. Custom
firmware sticks also have an `ISO` folder and a `seplugins` folder; those
are optional. A stick freshly formatted by the console always has at least
`PSP/GAME`.

## Ejecting safely

- Use the `Eject` button in the status bar (or `diskutil eject` on macOS,
  `umount` on Linux, the tray icon on Windows).
- Then exit USB mode on the console before pulling the cable.

Pulling the cable mid-write is the one way this app can corrupt content on
the stick. Installs and transfers chunk their writes and verify completion,
but nothing survives a yanked cable.

## USB mode tips

- `Settings > USB Connection` needs a memory stick inserted on non-Go
  models.
- Keep the console plugged into its charger during long transfers; USB
  mode drains the battery on some models.
- If the console charges but no volume appears, try the cable's data port
  (some cheap cables are charge-only).
