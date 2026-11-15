# Music and video

## Where files go

- Audio: `PSP/MUSIC/`, with optional `artist/album/track` nesting up to two
  folder levels.
- Video: `PSP/VIDEO/<name>.mp4`, with an optional `<name>.THM` thumbnail
  next to it.

Both pages show what is already on the stick with sizes, a storage bar, and
per-file delete.

## Music

Press **Push Music**, pick files (mp3, m4a, aac, wav, ogg, flac, wma), and
choose a mode:

- **Convert to MP3** (192 kbps, 44.1 kHz) - the safe default, plays on
  every firmware.
- **Convert to M4A** - AAC at 192 kbps, smaller files.
- **Copy as-is** - straight copy for files that are already PSP-compatible
  (mp3, m4a, aac, wav).

Conversion runs through ffmpeg with live progress in the queue. File names
are sanitized for FAT32 (`:` and other illegal characters become `_`).

Set the default mode in Settings, or per push with the CLI:

```sh
godsend-psp music ~/Music/Album/*.flac            # converts to mp3
godsend-psp music ~/Music/Album/*.mp3 --raw       # copies
godsend-psp music song.m4a --format m4a
```

## Video

The PSP plays MPEG-4 AVC up to 480x272. **Push Video** converts anything
you throw at it (mkv, avi, mov, webm, wmv) with this preset:

```text
h264 main profile, level 3.0
scale to fit 480x272 with pad
yuv420p, AAC 128 kbps audio, MP4 +faststart
```

Two toggles:

- **Convert to PSP AVC / Copy as-is** - copies when the file is already an
  mp4 the PSP can read.
- **Generate .THM thumbnails** - writes a 160x120 jpeg next to each video
  so the XMB shows a preview frame.

```sh
godsend-psp video ~/Movies/film.mkv --thm
godsend-psp video clip.mp4 --raw
```

## ffmpeg

Conversion needs ffmpeg on the PATH (or a path set in Settings). Without
it, pushes degrade to plain copies and the status bar shows
`ffmpeg missing`. Check the Home page > Environment panel for versions:

```sh
godsend-psp doctor
```

## WMA note

Stock firmware can play WMA only after enabling it in `Settings > System
Settings > Enable WMA Playback` on the console. Converting to mp3 avoids
that entirely.
