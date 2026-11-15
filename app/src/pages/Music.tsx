import { useEffect, useState } from "react";
import { api, pickers } from "../api";
import type { MediaFile } from "../types";
import type { PageProps } from "../App";
import { Bar, Btn, Chip, Empty, Seg, Toggle, confirmDialog, fmtBytes } from "../ui";

const MUSIC_EXTS = ["mp3", "m4a", "aac", "wav", "ogg", "flac", "wma"];

export function MediaPage({
  kind,
  snap,
  reload,
  toast,
  refreshKey,
}: PageProps & { kind: "music" | "video" }) {
  const [files, setFiles] = useState<MediaFile[]>([]);
  const [staged, setStaged] = useState<string[]>([]);
  const [query, setQuery] = useState("");
  const isVideo = kind === "video";
  const config = snap.config;

  const connected = !!snap.volumes.find((v) => v.is_psp);
  const ffmpegOk = snap.doctor.find((c) => c.name === "ffmpeg")?.ok ?? false;

  async function load() {
    if (!connected) {
      setFiles([]);
      return;
    }
    try {
      setFiles(isVideo ? await api.videoList() : await api.musicList());
    } catch {
      setFiles([]);
    }
  }

  useEffect(() => {
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [connected, refreshKey, kind]);

  const shown = files.filter((f) => f.rel.toLowerCase().includes(query.toLowerCase()));
  const used = files.reduce((sum, f) => sum + f.size_bytes, 0);
  const volume = snap.volumes.find((v) => v.is_psp);

  return (
    <div className="page">
      <div className="card panel">
        <div className="panel-head">
          <h2>{isVideo ? "Video transfer" : "Music transfer"}</h2>
          <div className="spacer" />
          <Chip tone={connected ? "good" : "bad"}>{connected ? "device ready" : "no device"}</Chip>
        </div>

        <div style={{ display: "flex", gap: 14, alignItems: "center", flexWrap: "wrap" }}>
          {isVideo ? (
            <>
              <Seg
                ariaLabel="Video mode"
                value={config.convert_video ? "convert" : "raw"}
                onChange={(mode) => {
                  void api.configSet({ convert_video: mode === "convert" });
                  reload();
                }}
                options={[
                  { value: "convert", label: "Convert to PSP AVC" },
                  { value: "raw", label: "Copy as-is" },
                ]}
              />
              <Toggle
                checked={config.video_thumbnails}
                onChange={async (next) => {
                  await api.configSet({ video_thumbnails: next });
                  reload();
                }}
                label="Generate .THM thumbnails"
              />
            </>
          ) : (
            <>
              <Seg
                ariaLabel="Music mode"
                value={
                  !config.convert_music
                    ? "raw"
                    : config.music_format === "m4a"
                      ? "m4a"
                      : "mp3"
                }
                onChange={(mode) => {
                  void api.configSet({
                    convert_music: mode !== "raw",
                    music_format: mode === "m4a" ? "m4a" : "mp3",
                  });
                  reload();
                }}
                options={[
                  { value: "mp3", label: "Convert to MP3" },
                  { value: "m4a", label: "Convert to M4A" },
                  { value: "raw", label: "Copy as-is" },
                ]}
              />
            </>
          )}
          <div style={{ flex: 1 }} />
          {isVideo ? (
            <>
              <Btn
                disabled={!connected}
                onClick={async () => {
                  const picked = await pickers.pickFiles("Browse videos to add", [
                    "mp4", "mkv", "avi", "mov", "wmv", "webm",
                  ]);
                  if (!picked.length) return;
                  setStaged((cur) => [...cur, ...picked.filter((p) => !cur.includes(p))]);
                }}
              >
                Browse Videos...
              </Btn>
              <Btn
                variant="primary"
                disabled={!connected || staged.length === 0}
                onClick={async () => {
                  await api.videoPush(
                    staged,
                    config.convert_video && ffmpegOk,
                    config.video_thumbnails
                  );
                  toast(`queued ${staged.length} video(s)`);
                  setStaged([]);
                }}
              >
                Push {staged.length || ""} Video{staged.length === 1 ? "" : "s"}
              </Btn>
            </>
          ) : (
            <Btn
              variant="primary"
              disabled={!connected}
              onClick={async () => {
                const picked = await pickers.pickFiles("Pick music to push", MUSIC_EXTS);
                if (!picked.length) return;
                await api.musicPush(picked, config.convert_music && ffmpegOk);
                toast(`queued ${picked.length} file(s)`);
              }}
            >
              Push Music
            </Btn>
          )}
        </div>

        {isVideo && staged.length > 0 ? (
          <div className="rows" style={{ marginTop: 10 }}>
            {staged.map((path) => (
              <div className="row" key={path}>
                <Chip tone="laser">queued</Chip>
                <div className="grow">
                  <div className="title" title={path}>
                    {path.split("/").pop()}
                  </div>
                  <div className="sub">{path}</div>
                </div>
                <Btn
                  small
                  onClick={() => setStaged((cur) => cur.filter((p) => p !== path))}
                >
                  Remove
                </Btn>
              </div>
            ))}
          </div>
        ) : null}

        {!ffmpegOk && config.convert_music ? (
          <div className="note" style={{ marginTop: 10 }}>
            ffmpeg not found: pushes will copy files as-is instead of converting.
            Install ffmpeg or set its path in Settings.
          </div>
        ) : null}

        <div style={{ marginTop: 12 }}>
          <Bar value={used} max={volume?.total_bytes ?? 0} />
          <div className="mono" style={{ fontSize: 11, color: "var(--text-faint)", marginTop: 5 }}>
            {fmtBytes(used)} in PSP/{isVideo ? "VIDEO" : "MUSIC"}
            {volume ? ` of ${fmtBytes(volume.total_bytes)} stick` : ""}
          </div>
        </div>
      </div>

      <input
        type="text"
        className="mono"
        placeholder="search files"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        aria-label="Search media files"
        style={{
          background: "rgba(255,255,255,0.03)",
          border: "1px solid var(--line-strong)",
          color: "var(--text)",
          borderRadius: "var(--radius-sm)",
          padding: "8px 10px",
          fontSize: 12.5,
        }}
      />

      {shown.length === 0 ? (
        <Empty
          title={isVideo ? "No videos on the stick" : "No music on the stick"}
          hint={`Files live in PSP/${isVideo ? "VIDEO" : "MUSIC"} on the memory stick.`}
        />
      ) : (
        <div className="rows">
          {shown.map((file) => (
            <div className="row" key={file.rel}>
              {isVideo ? <VideoPreview rel={file.rel} /> : <Chip tone="neutral">audio</Chip>}
              <div className="grow">
                <div className="title" title={file.rel}>
                  {file.name}
                </div>
                <div className="sub">{file.rel}</div>
              </div>
              <span className="size">{fmtBytes(file.size_bytes)}</span>
              <Btn
                small
                variant="danger"
                onClick={async () => {
                  if (confirmDialog("Delete from stick?", `${file.rel} will be removed.`)) {
                    await api.mediaDelete(kind, file.rel);
                    toast(`deleted ${file.name}`);
                    reload();
                    void load();
                  }
                }}
              >
                Delete
              </Btn>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export default function Music(props: PageProps) {
  return <MediaPage kind="music" {...props} />;
}

/* PSP-style hover preview: a cached three second clip of the movie. */
function VideoPreview({ rel }: { rel: string }) {
  const [src, setSrc] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [hover, setHover] = useState(false);

  async function ensure() {
    if (src || loading) return;
    setLoading(true);
    try {
      setSrc(await api.videoPreview(rel));
    } catch {
      setSrc(null);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div
      className={`preview-frame ${hover ? "on" : ""} ${loading ? "loading" : ""}`}
      onMouseEnter={() => {
        setHover(true);
        void ensure();
      }}
      onMouseLeave={() => setHover(false)}
      onFocus={() => {
        setHover(true);
        void ensure();
      }}
      onBlur={() => setHover(false)}
      tabIndex={0}
      aria-label={`Preview ${rel}`}
    >
      {hover && src ? (
        <video src={src} autoPlay muted loop playsInline />
      ) : (
        <span className="preview-idle" aria-hidden="true" />
      )}
    </div>
  );
}
