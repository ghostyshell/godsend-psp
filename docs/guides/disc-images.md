# Disc images

CFW ISO drivers (ARK ships Inferno) load UMD images straight off the
memory stick. This app manages the `ISO/` folder at the stick root.

## Importing

Press **Import Image** and pick an `.iso`, `.cso` or `.pbp`. With
**Compress to CSO on import** enabled, plain ISOs are compressed on the way
in, usually saving 30-50% of the space at level 6.

```sh
godsend-psp iso-import "Game.iso" --cso   # compress on import
godsend-psp iso-import "Game.cso"         # copy as-is
```

## Metadata

For plain ISOs the app walks the ISO9660 tree and reads
`PSP_GAME/PARAM.SFO`, so the Games and Images pages show the real UMD
title and serial (for example `ULUS-10509`) without launching anything.
CSO files are probed by header only; decompress once if you want their
titles in the UI.

## ISO to CSO and back

The conversion is a pure Rust codec (zlib per 2048-byte block, CSO v1):

- **To CSO** compresses block by block with a chosen level (Settings,
  1 fast to 9 smallest). Blocks that do not shrink are stored raw.
- **To ISO** decompresses back to a bit-exact image.

Both run as cancellable jobs with progress.

```sh
godsend-psp convert "Game.iso"             # writes Game.cso
godsend-psp convert "Game.cso" --level 9   # writes Game.iso
```

## Multi-disc games

Some PSN titles ship as `.pbp` with two discs; those are listed and copied
as-is. For ISO folder multi-disc sets, name the discs so they sort sensibly
(`Final Fantasy IX.disc1.iso`).

## Compatibility notes

- CSO v1 is supported by every modern CFW ISO driver. There is no need for
  CSO v2 or ZSO here.
- Very fast memory sticks reduce stutter in games that stream heavily;
  compression is unrelated to that.
- Firmware 6.61/6.60 + ARK-4 reads everything listed here.
