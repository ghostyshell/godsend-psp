import { useEffect, useState } from "react";
import { api, pickers } from "../api";
import type { PageProps } from "../App";
import { Btn, Chip, Seg, Toggle } from "../ui";

export default function Settings({ snap, reload, toast, refreshKey }: PageProps) {
  const [rdKey, setRdKey] = useState("");
  const [tbKey, setTbKey] = useState("");
  const [downloadDir, setDownloadDir] = useState("");
  const [debrid, setDebrid] = useState({ realdebrid: false, torbox: false });

  const config = snap.config;

  useEffect(() => {
    void api.debridStatus().then(setDebrid);
    setDownloadDir(config.download_dir);
  }, [refreshKey, config.download_dir]);

  return (
    <div className="page">
      <section className="card panel">
        <div className="panel-head">
          <h2>Debrid accounts</h2>
          <div className="spacer" />
          <Chip tone={debrid.realdebrid ? "good" : "neutral"}>
            RD {debrid.realdebrid ? "linked" : "off"}
          </Chip>
          <Chip tone={debrid.torbox ? "good" : "neutral"}>
            TB {debrid.torbox ? "linked" : "off"}
          </Chip>
        </div>
        <p style={{ margin: "0 0 12px", fontSize: 12.5, color: "var(--text-faint)" }}>
          Debrid keys unlock torrent/magnet sources in the Store and speed up
          host-limited downloads. Keys are stored locally in the app config.
        </p>
        <div className="field">
          <label>Real-Debrid API key</label>
          <div style={{ display: "flex", gap: 8 }}>
            <input
              type="password"
              value={rdKey}
              onChange={(e) => setRdKey(e.target.value)}
              placeholder={config.rd_api_key ? "saved" : "paste key"}
              style={inputStyle}
            />
            <Btn
              onClick={async () => {
                await api.configSet({ rd_api_key: rdKey.trim() } as never);
                toast("Real-Debrid key saved");
                setDebrid(await api.debridStatus());
                reload();
              }}
            >
              Save
            </Btn>
          </div>
        </div>
        <div className="field">
          <label>TorBox API key</label>
          <div style={{ display: "flex", gap: 8 }}>
            <input
              type="password"
              value={tbKey}
              onChange={(e) => setTbKey(e.target.value)}
              placeholder={config.tb_api_key ? "saved" : "paste key"}
              style={inputStyle}
            />
            <Btn
              onClick={async () => {
                await api.configSet({ tb_api_key: tbKey.trim() } as never);
                toast("TorBox key saved");
                setDebrid(await api.debridStatus());
                reload();
              }}
            >
              Save
            </Btn>
          </div>
        </div>
      </section>

      <section className="card panel">
        <div className="panel-head">
          <h2>Store sources</h2>
        </div>
        <div className="field">
          <label>game-search base URL</label>
          <div style={{ display: "flex", gap: 8 }}>
            <input
              type="text"
              defaultValue={config.gamesearch_base}
              onBlur={async (e) => {
                if (e.target.value !== config.gamesearch_base) {
                  await api.configSet({ gamesearch_base: e.target.value.trim() } as never);
                  toast("source saved");
                }
              }}
              style={inputStyle}
            />
          </div>
        </div>
        <div className="field">
          <label>R2 covers base (empty = catalog URLs only)</label>
          <input
            type="text"
            defaultValue={config.covers_base}
            placeholder="https://covers.example.com"
            onBlur={async (e) => {
              if (e.target.value !== config.covers_base) {
                await api.configSet({ covers_base: e.target.value.trim() } as never);
                toast("covers base saved");
              }
            }}
            style={inputStyle}
          />
        </div>
        <div className="field">
          <label>Mods catalog base (empty = mods disabled)</label>
          <input
            type="text"
            defaultValue={config.mods_base}
            placeholder="https://covers.example.com"
            onBlur={async (e) => {
              if (e.target.value !== config.mods_base) {
                await api.configSet({ mods_base: e.target.value.trim() } as never);
                toast("mods base saved");
              }
            }}
            style={inputStyle}
          />
        </div>
      </section>

      <section className="card panel">
        <div className="panel-head">
          <h2>Downloads</h2>
        </div>
        <div className="field">
          <label>Download folder (empty = Downloads/godsend-psp)</label>
          <div style={{ display: "flex", gap: 8 }}>
            <input
              type="text"
              value={downloadDir}
              onChange={(e) => setDownloadDir(e.target.value)}
              placeholder="downloads destination"
              style={inputStyle}
            />
            <Btn
              onClick={async () => {
                const chosen = await pickers.pickDirectory("Pick download folder");
                if (chosen) setDownloadDir(chosen);
              }}
            >
              Browse
            </Btn>
            <Btn
              onClick={async () => {
                if (!downloadDir) {
                  await api.configSet({ download_dir: "" } as never);
                } else {
                  await api.downloadDirSet(downloadDir);
                }
                toast("download folder saved");
                reload();
              }}
            >
              Save
            </Btn>
          </div>
        </div>
      </section>

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
                } as never);
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
                await api.configSet({ convert_video: next } as never);
                reload();
              }}
              label="Convert to PSP AVC 480x272"
            />
            <Toggle
              checked={config.video_thumbnails}
              onChange={async (next) => {
                await api.configSet({ video_thumbnails: next } as never);
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
                void api.configSet({ cso_level: Number(level) } as never);
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
          <Btn small onClick={async () => toast(`checks: ${(await api.doctorRun()).filter((c) => c.ok).length} ok`)}>
            Run Checks
          </Btn>
        </div>
        <div className="checks">
          {snap.doctor.map((check) => (
            <div className="check" key={check.name}>
              <Chip tone={check.ok ? "good" : "bad"}>{check.ok ? "ok" : "issue"}</Chip>
              <span className="name">{check.name}</span>
              <span className="detail" title={check.detail}>
                {check.detail}
              </span>
            </div>
          ))}
        </div>
        <div style={{ marginTop: 12, display: "flex", gap: 8 }}>
          <Btn onClick={() => void api.openDataDir()}>Open Data Folder</Btn>
        </div>
        <p style={{ margin: "12px 0 0", fontSize: 12.5, color: "var(--text-faint)" }}>
          godsend-psp 0.2.2 - Rust engine + Tauri. USB mass storage, no drivers.
        </p>
      </section>
    </div>
  );
}

const inputStyle: React.CSSProperties = {
  flex: 1,
  minWidth: 180,
  background: "rgba(255,255,255,0.03)",
  border: "1px solid var(--line-strong)",
  color: "var(--text)",
  borderRadius: "var(--radius-sm)",
  padding: "8px 10px",
  fontFamily: "var(--mono)",
  fontSize: 12.5,
};
