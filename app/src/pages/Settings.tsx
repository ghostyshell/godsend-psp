import { useEffect, useState } from "react";
import { api } from "../api";
import type { PageProps } from "../App";
import { Btn, Chip, Seg, Toggle } from "../ui";

export default function Settings({ snap, reload, toast, refreshKey }: PageProps) {
  const [checks, setChecks] = useState<typeof snap.doctor>(snap.doctor);
  const [ffmpegPath, setFfmpegPath] = useState(snap.config.ffmpeg_path);

  useEffect(() => {
    setChecks(snap.doctor);
    setFfmpegPath(snap.config.ffmpeg_path);
  }, [snap.doctor, snap.config, refreshKey]);

  const config = snap.config;

  return (
    <div className="page">
      <section className="card panel">
        <div className="panel-head">
          <h2>Transfers</h2>
        </div>
        <div className="field">
          <label>Music conversion</label>
          <div>
            <Seg
              ariaLabel="Music format"
              value={!config.convert_music ? "raw" : config.music_format === "m4a" ? "m4a" : "mp3"}
              onChange={(mode) => {
                void api.configSet({
                  convert_music: mode !== "raw",
                  music_format: mode === "m4a" ? "m4a" : "mp3",
                });
                reload();
              }}
              options={[
                { value: "mp3", label: "MP3 192k" },
                { value: "m4a", label: "M4A (AAC)" },
                { value: "raw", label: "Copy as-is" },
              ]}
            />
          </div>
        </div>
        <div className="field">
          <label>Video</label>
          <div style={{ display: "flex", gap: 14, flexWrap: "wrap", alignItems: "center" }}>
            <Toggle
              checked={config.convert_video}
              onChange={async (next) => {
                await api.configSet({ convert_video: next });
                reload();
              }}
              label="Convert to PSP AVC 480x272"
            />
            <Toggle
              checked={config.video_thumbnails}
              onChange={async (next) => {
                await api.configSet({ video_thumbnails: next });
                reload();
              }}
              label="Generate .THM thumbnails"
            />
          </div>
        </div>
        <div className="field">
          <label>ISO to CSO compression level ({config.cso_level})</label>
          <div>
            <Seg
              ariaLabel="CSO level"
              value={String(config.cso_level)}
              onChange={(level) => {
                void api.configSet({ cso_level: Number(level) });
                reload();
              }}
              options={[
                { value: "1", label: "1 fast" },
                { value: "3", label: "3" },
                { value: "6", label: "6" },
                { value: "9", label: "9 smallest" },
              ]}
            />
          </div>
        </div>
      </section>

      <section className="card panel">
        <div className="panel-head">
          <h2>Tools</h2>
          <div className="spacer" />
          <Chip tone={checks.find((c) => c.name === "ffmpeg")?.ok ? "good" : "warn"}>
            {checks.find((c) => c.name === "ffmpeg")?.ok ? "ffmpeg found" : "ffmpeg missing"}
          </Chip>
        </div>
        <div className="field">
          <label>ffmpeg path (empty = auto-detect)</label>
          <div style={{ display: "flex", gap: 8 }}>
            <input
              type="text"
              value={ffmpegPath}
              onChange={(e) => setFfmpegPath(e.target.value)}
              placeholder="/opt/homebrew/bin/ffmpeg"
              style={{
                flex: 1,
                background: "rgba(255,255,255,0.03)",
                border: "1px solid var(--line-strong)",
                color: "var(--text)",
                borderRadius: "var(--radius-sm)",
                padding: "8px 10px",
                fontFamily: "var(--mono)",
                fontSize: 12.5,
              }}
            />
            <Btn
              onClick={async () => {
                await api.configSet({ ffmpeg_path: ffmpegPath.trim() });
                setChecks(await api.doctorRun());
                toast("ffmpeg path saved");
              }}
            >
              Save
            </Btn>
          </div>
        </div>
        <div className="checks">
          {checks.map((check) => (
            <div className="check" key={check.name}>
              <Chip tone={check.ok ? "good" : "bad"}>{check.ok ? "ok" : "issue"}</Chip>
              <span className="name">{check.name}</span>
              <span className="detail" title={check.detail}>
                {check.detail}
              </span>
            </div>
          ))}
        </div>
      </section>

      <section className="card panel">
        <div className="panel-head">
          <h2>About</h2>
        </div>
        <div className="kv">
          <span className="k">version</span>
          <span className="v">godsend-psp 0.1.0</span>
        </div>
        <div className="kv">
          <span className="k">engine</span>
          <span className="v">godsend-core (Rust) + Tauri</span>
        </div>
        <div className="kv">
          <span className="k">device link</span>
          <span className="v">USB mass storage (no drivers needed)</span>
        </div>
        <div style={{ marginTop: 12, display: "flex", gap: 8 }}>
          <Btn
            onClick={async () => {
              await api.openDataDir();
            }}
          >
            Open Data Folder
          </Btn>
          <Btn
            onClick={async () => {
              setChecks(await api.doctorRun());
              toast("checks re-run");
            }}
          >
            Re-run Checks
          </Btn>
        </div>
        <p style={{ margin: "12px 0 0", fontSize: 12.5, color: "var(--text-faint)" }}>
          Works with PSP models in USB mode over custom firmware (ARK-4 or
          classic CFW). Homebrew, digital titles, ISO/CSO images, saves,
          plugins and themes are managed straight on the memory stick.
        </p>
      </section>
    </div>
  );
}
