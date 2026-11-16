import { useEffect, useState } from "react";
import { api, pickers } from "../api";
import type { PageProps } from "../App";
import { Btn, Chip, Seg, Toggle } from "../ui";

export default function Settings({ snap, reload, toast, refreshKey }: PageProps) {
  const [rdKey, setRdKey] = useState("");
  const [tbKey, setTbKey] = useState("");
  const [iaKey, setIaKey] = useState("");
  const [iaSecret, setIaSecret] = useState("");
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
        <p style={{ margin: "0 0 12px", fontSize: 12.5, color: "var(--text-faint)" }}>
          Internet Archive keys are optional. Set both to sign archive.org
          downloads for items that need an account; the keys stay in the local
          config and are kept out of job errors.
        </p>
        <div className="field">
          <label>Internet Archive username</label>
          <input
            type="text"
            defaultValue={config.ia_email}
            placeholder="archive.org account email"
            onBlur={async (e) => {
              if (e.target.value !== config.ia_email) {
                await api.configSet({ ia_email: e.target.value.trim() } as never);
                toast("Internet Archive username saved");
              }
            }}
            style={inputStyle}
          />
        </div>
        <div className="field">
          <label>Internet Archive password</label>
          <div style={{ display: "flex", gap: 8 }}>
            <input
              type="password"
              defaultValue=""
              placeholder={config.ia_password ? "saved" : "archive.org password"}
              onBlur={async (e) => {
                if (e.target.value) {
                  await api.configSet({ ia_password: e.target.value } as never);
                  toast("Internet Archive password saved");
                }
              }}
              style={inputStyle}
            />
          </div>
        </div>
        <div className="field">
          <label>Internet Archive access key</label>
          <div style={{ display: "flex", gap: 8 }}>
            <input
              type="password"
              value={iaKey}
              onChange={(e) => setIaKey(e.target.value)}
              placeholder={config.ia_access_key ? "saved" : "paste access key"}
              style={inputStyle}
            />
            <Btn
              onClick={async () => {
                await api.configSet({ ia_access_key: iaKey.trim() } as never);
                toast("Internet Archive access key saved");
                setIaKey("");
                reload();
              }}
            >
              Save
            </Btn>
          </div>
        </div>
        <div className="field">
          <label>Internet Archive secret key</label>
          <div style={{ display: "flex", gap: 8 }}>
            <input
              type="password"
              value={iaSecret}
              onChange={(e) => setIaSecret(e.target.value)}
              placeholder={config.ia_secret_key ? "saved" : "paste secret key"}
              style={inputStyle}
            />
            <Btn
              onClick={async () => {
                await api.configSet({ ia_secret_key: iaSecret.trim() } as never);
                toast("Internet Archive secret key saved");
                setIaSecret("");
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
          <label>Encoder preset (veryslow is the best quality per bit)</label>
          <div>
            <Seg
              ariaLabel="Encoder preset"
              value={config.video_preset}
              onChange={(preset) => {
                void api.configSet({ video_preset: preset } as never);
                reload();
              }}
              options={[
                { value: "veryslow", label: "veryslow" },
                { value: "slow", label: "slow" },
                { value: "medium", label: "medium" },
                { value: "fast", label: "fast" },
              ]}
            />
          </div>
        </div>
        <div className="field">
          <label>Video quality (CRF {config.video_crf}; lower is better)</label>
          <div>
            <Seg
              ariaLabel="Video quality"
              value={String(config.video_crf)}
              onChange={(crf) => {
                void api.configSet({ video_crf: Number(crf) } as never);
                reload();
              }}
              options={[
                { value: "16", label: "Lossless-ish (16)" },
                { value: "18", label: "Maximum (18)" },
                { value: "21", label: "High (21)" },
                { value: "24", label: "Balanced (24)" },
              ]}
            />
          </div>
        </div>
        <div className="field">
          <label>Video resolution</label>
          <div>
            <Seg
              ariaLabel="Video resolution"
              value={config.video_resolution}
              onChange={(resolution) => {
                void api.configSet({ video_resolution: resolution } as never);
                reload();
              }}
              options={[
                { value: "480x272", label: "PSP native 480x272" },
                { value: "368x208", label: "368x208" },
                { value: "source", label: "Keep source" },
              ]}
            />
          </div>
        </div>
        <div className="field">
          <label>Frame rate cap and audio bitrate</label>
          <div style={{ display: "flex", gap: 10, flexWrap: "wrap" }}>
            <Seg
              ariaLabel="Video fps"
              value={String(config.video_fps)}
              onChange={(fps) => {
                void api.configSet({ video_fps: Number(fps) } as never);
                reload();
              }}
              options={[
                { value: "30", label: "30 fps" },
                { value: "24", label: "24 fps" },
              ]}
            />
            <Seg
              ariaLabel="Audio bitrate"
              value={String(config.video_audio_kbps)}
              onChange={(kbps) => {
                void api.configSet({ video_audio_kbps: Number(kbps) } as never);
                reload();
              }}
              options={[
                { value: "256", label: "256 kbps" },
                { value: "192", label: "192 kbps" },
                { value: "128", label: "128 kbps" },
              ]}
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
          <h2>MCP server</h2>
          <div className="spacer" />
          <Chip tone={config.mcp_enabled ? "good" : "neutral"}>
            {config.mcp_enabled ? "on" : "off"}
          </Chip>
        </div>
        <p style={{ margin: "0 0 12px", fontSize: 12.5, color: "var(--text-faint)" }}>
          Model Context Protocol endpoint for local agents: read-only device,
          library, store, downloads, plugins and doctor tools over JSON-RPC.
          Restart the app after changing the port in the config file.
        </p>
        <div className="field">
          <label>MCP server</label>
          <code style={endpointStyle}>http://127.0.0.1:{config.mcp_port}</code>
        </div>
        <div className="field">
          <label>Start with the app</label>
          <Toggle
            checked={config.mcp_enabled}
            onChange={async (next) => {
              await api.configSet({ mcp_enabled: next });
              toast(next ? "MCP server enabled" : "MCP server disabled");
              reload();
            }}
            label="Serve the MCP endpoint while the app runs"
          />
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
          godsend-psp 0.2.35 - Rust engine + Tauri. USB mass storage, no drivers.
        </p>
      </section>
    </div>
  );
}

const endpointStyle: React.CSSProperties = {
  fontFamily: "var(--mono)",
  fontSize: 12.5,
  color: "var(--text)",
  background: "rgba(255,255,255,0.03)",
  border: "1px solid var(--line-strong)",
  borderRadius: "var(--radius-sm)",
  padding: "8px 10px",
  display: "block",
};

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
