import { useEffect, useState } from "react";
import { api, pickers } from "../api";
import type { CatalogEntry, PluginsView, RemoteTheme, ThemeEntry, ThemePack } from "../types";
import type { PageProps } from "../App";
import { Btn, Chip, Empty, Seg, Toggle, confirmDialog, fmtBytes } from "../ui";
import { clearTheme, loadStoredTheme, selectTheme, slugify, tint } from "../theme";
import type { StoredTheme } from "../theme";

type Tab = "plugins" | "themes" | "cfw";

export default function Mods({ snap, reload, toast, refreshKey }: PageProps) {
  const [tab, setTab] = useState<Tab>("plugins");
  const connected = !!snap.volumes.find((v) => v.is_psp);

  return (
    <div className="page">
      <Seg
        ariaLabel="Mods section"
        value={tab}
        onChange={setTab}
        options={[
          { value: "plugins", label: "Plugins" },
          { value: "themes", label: "Themes" },
          { value: "cfw", label: "Custom Firmware" },
        ]}
      />

      {tab === "plugins" && <PluginsPanel snap={snap} reload={reload} toast={toast} refreshKey={refreshKey} />}
      {tab === "themes" && <ThemesPanel snap={snap} reload={reload} toast={toast} refreshKey={refreshKey} />}
      {tab === "cfw" && <CfwPanel snap={snap} reload={reload} toast={toast} refreshKey={refreshKey} />}

      {!connected ? (
        <Empty title="No PSP connected" hint="Mods are installed onto the memory stick; connect the PSP in USB mode." />
      ) : null}
    </div>
  );
}

/* ---------- plugins ---------- */

function PluginsPanel({ snap, reload, toast, refreshKey }: PageProps) {
  const [view, setView] = useState<PluginsView | null>(null);
  const [addSection, setAddSection] = useState<"game" | "vsh" | "pops">("game");
  const connected = !!snap.volumes.find((v) => v.is_psp);

  useEffect(() => {
    if (!connected) {
      setView(null);
      return;
    }
    void api
      .pluginsList()
      .then(setView)
      .catch(() => setView(null));
  }, [connected, refreshKey]);

  const sections: { key: keyof PluginsView; label: string; hint: string }[] = [
    { key: "game", label: "Game", hint: "PSP/UMD games" },
    { key: "vsh", label: "VSH (XMB)", hint: "dashboard" },
    { key: "pops", label: "Pops (PS1)", hint: "PS1 emulator" },
  ];

  return (
    <>
      <div className="card panel">
        <div className="panel-head">
          <h2>seplugins</h2>
          <div className="spacer" />
          <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
            <Seg
              ariaLabel="Plugin target section"
              value={addSection}
              onChange={setAddSection}
              options={[
                { value: "game", label: "Game" },
                { value: "vsh", label: "VSH" },
                { value: "pops", label: "Pops" },
              ]}
            />
            <Btn
              disabled={!connected}
              onClick={async () => {
                const prx = await pickers.pickFile("Pick a plugin .prx", ["prx"]);
                if (!prx) return;
                await api.pluginAdd(addSection, prx);
                toast(`plugin installed into ${addSection}`);
                reload();
              }}
            >
              Add .prx
            </Btn>
          </div>
        </div>
        <p style={{ margin: "0 0 6px", fontSize: 12.5, color: "var(--text-faint)" }}>
          Each section is a text file in seplugins/ on the stick. Toggling a
          switch rewrites the enable flag; the file is never reformatted.
        </p>
      </div>

      {view
        ? sections.map(({ key, label, hint }) => (
            <section className="card panel" key={key}>
              <div className="panel-head">
                <h2>{label}</h2>
                <span className="section-caption">{key}.txt - {hint}</span>
                <div className="spacer" />
                <Chip tone="neutral">{view[key].length}</Chip>
              </div>
              {view[key].length === 0 ? (
                <p style={{ margin: 0, fontSize: 13, color: "var(--text-faint)" }}>
                  No plugins registered in this section.
                </p>
              ) : (
                <div className="rows">
                  {view[key].map((plugin) => (
                    <div className="row" key={`${key}-${plugin.path}`}>
                      <Toggle
                        checked={plugin.enabled}
                        onChange={async (next) => {
                          await api.pluginToggle(key, plugin.path, next);
                          reload();
                          const fresh = await api.pluginsList();
                          setView(fresh);
                        }}
                        label={plugin.enabled ? "on" : "off"}
                      />
                      <div className="grow">
                        <div className="title mono" style={{ fontSize: 12.5 }}>
                          {plugin.path}
                        </div>
                        <div className="sub">
                          {plugin.exists ? "prx present" : "prx missing on stick"}
                        </div>
                      </div>
                      {!plugin.exists ? <Chip tone="warn">missing</Chip> : null}
                      <Btn
                        small
                        variant="danger"
                        onClick={async () => {
                          await api.pluginRemove(key, plugin.path);
                          toast(`removed ${plugin.path.split("/").pop()}`);
                          const fresh = await api.pluginsList();
                          setView(fresh);
                        }}
                      >
                        Remove
                      </Btn>
                    </div>
                  ))}
                </div>
              )}
            </section>
          ))
        : null}
    </>
  );
}

/* ---------- themes ---------- */

function packDate(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleDateString();
}

function ThemesPanel({ snap, reload, toast, refreshKey }: PageProps) {
  const [themes, setThemes] = useState<ThemeEntry[]>([]);
  const [remote, setRemote] = useState<RemoteTheme[]>([]);
  const [packs, setPacks] = useState<ThemePack[]>([]);
  const [active, setActive] = useState<StoredTheme | null>(() => loadStoredTheme());
  const [name, setName] = useState("");
  const [accent, setAccent] = useState("#6d8bff");
  const [wallpaper, setWallpaper] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const connected = !!snap.volumes.find((v) => v.is_psp);

  useEffect(() => {
    if (!connected) {
      setThemes([]);
      setPacks([]);
      return;
    }
    void api
      .themesList()
      .then(setThemes)
      .catch(() => setThemes([]));
    void api
      .themesRemote()
      .then(setRemote)
      .catch(() => setRemote([]));
    void api
      .themePacksList()
      .then(setPacks)
      .catch(() => setPacks([]));
  }, [connected, refreshKey]);

  const createPack = async () => {
    if (!name.trim() || busy) return;
    setBusy(true);
    try {
      const pack = await api.themePackCreate(name.trim(), accent, wallpaper);
      setPacks((prev) => [pack, ...prev.filter((p) => p.name !== pack.name)]);
      toast(`theme pack created: ${pack.name}`);
      setName("");
      setWallpaper(null);
      reload();
    } catch (err) {
      toast(`theme pack failed: ${String(err)}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <section className="card panel">
        <div className="panel-head">
          <h2>Theme Studio</h2>
          <div className="spacer" />
          <Btn
            disabled={!connected}
            onClick={async () => {
              const zip = await pickers.pickFile("Import a .godsend-theme.zip", ["zip"]);
              if (!zip) return;
              try {
                const pack = await api.themePackInstall(zip);
                toast(`theme pack installed: ${pack.name}`);
                reload();
              } catch (err) {
                toast(`import failed: ${String(err)}`);
              }
            }}
          >
            Import pack
          </Btn>
        </div>
        <p className="hint">
          CXMB .ctf and Sony .ptf are closed containers, so packs here are
          wallpaper plus accent themes. Existing .ctf and .ptf files are still
          installed and managed below.
        </p>

        <div className="theme-form">
          <div className="field">
            <label htmlFor="theme-pack-name">Pack name</label>
            <input
              id="theme-pack-name"
              type="text"
              value={name}
              placeholder="Midnight Brass"
              onChange={(event) => setName(event.target.value)}
            />
          </div>
          <div className="field narrow">
            <label htmlFor="theme-pack-accent">Accent</label>
            <input
              id="theme-pack-accent"
              type="color"
              value={accent}
              onChange={(event) => setAccent(event.target.value)}
            />
          </div>
          <div className="field">
            <label>Wallpaper (optional)</label>
            <div className="theme-pick">
              <Btn
                small
                onClick={async () => {
                  const file = await pickers.pickFile("Pick a wallpaper", ["png", "jpg", "jpeg"]);
                  if (file) setWallpaper(file);
                }}
              >
                {wallpaper ? "Change" : "Choose"}
              </Btn>
              <span className="meta mono">
                {wallpaper ? wallpaper.split("/").pop() : "accent only"}
              </span>
              {wallpaper ? (
                <Btn small variant="outline" onClick={() => setWallpaper(null)}>
                  Clear
                </Btn>
              ) : null}
            </div>
          </div>
          <Btn variant="primary" disabled={!connected || !name.trim() || busy} onClick={createPack}>
            {busy ? "Creating..." : "Create"}
          </Btn>
        </div>

        <div
          className="theme-preview"
          style={{ background: `linear-gradient(112deg, ${tint(accent, 0.5)}, ${tint(accent, 0.1)})` }}
        >
          <span className="swatch" style={{ background: accent }} aria-hidden="true" />
          <span className="theme-preview-name">{name.trim() || "Untitled pack"}</span>
          <span className="theme-preview-sub mono">
            {accent} {wallpaper ? `+ ${wallpaper.split("/").pop()}` : "accent only"}
          </span>
        </div>
      </section>

      {packs.length === 0 ? (
        <Empty
          title="No theme packs yet"
          hint="Create one above; it lands in PSP/PICTURE/<name>/ on the stick."
        />
      ) : (
        <div className="theme-grid">
          {packs.map((pack) => {
            const folder = slugify(pack.name);
            const isActive = active?.folder === folder;
            return (
              <article className={`theme-card ${isActive ? "active" : ""}`} key={`${folder}-${pack.created_at}`}>
                <div className="theme-card-head">
                  <span className="swatch" style={{ background: pack.accent }} aria-hidden="true" />
                  <div className="grow">
                    <div className="title">{pack.name}</div>
                    <div className="meta">{packDate(pack.created_at)}</div>
                  </div>
                  {isActive ? <Chip tone="good">in use</Chip> : null}
                </div>
                <div className="meta">
                  {pack.background ? "wallpaper + accent" : "accent only"}
                </div>
                <div className="actions">
                  <Btn
                    small
                    variant={isActive ? "outline" : "primary"}
                    onClick={() => {
                      selectTheme(pack, folder);
                      setActive({ ...pack, folder });
                    }}
                  >
                    Use
                  </Btn>
                  <Btn
                    small
                    variant="danger"
                    onClick={async () => {
                      if (!confirmDialog("Delete theme pack?", `${pack.name} will be removed from PSP/PICTURE/${folder}.`)) {
                        return;
                      }
                      try {
                        await api.themePackDelete(folder);
                        if (isActive) {
                          clearTheme();
                          setActive(null);
                        }
                        setPacks((prev) => prev.filter((p) => p !== pack));
                        toast(`deleted ${pack.name}`);
                        reload();
                      } catch (err) {
                        toast(`delete failed: ${String(err)}`);
                      }
                    }}
                  >
                    Delete
                  </Btn>
                </div>
              </article>
            );
          })}
        </div>
      )}

      <div className="card panel">
        <div className="panel-head">
          <h2>XMB themes</h2>
          <div className="spacer" />
          {active ? (
            <Btn
              small
              onClick={() => {
                clearTheme();
                setActive(null);
                toast("app theme reset");
              }}
            >
              Reset
            </Btn>
          ) : null}
          <Btn
            variant="primary"
            disabled={!connected}
            onClick={async () => {
              const file = await pickers.pickFile("Pick a theme", ["ptf", "ctf"]);
              if (!file) return;
              await api.themeInstall(file);
              toast(`theme installed: ${file.split("/").pop()}`);
              reload();
            }}
          >
            Install Theme
          </Btn>
        </div>
        <p style={{ margin: 0, fontSize: 12.5, color: "var(--text-faint)" }}>
          .ptf themes are official XMB themes. .ctf themes need the CXMB plugin
          enabled for VSH; install it from the Plugins tab or the catalog.
          Themes land in PSP/THEME.
        </p>
      </div>

      {remote.length > 0 ? (
        <section className="card panel">
          <div className="panel-head">
            <h2>Hosted themes</h2>
            <div className="spacer" />
            <Chip tone="laser">{remote.length}</Chip>
          </div>
          <div className="rows">
            {remote.map((theme) => (
              <div className="row" key={theme.url}>
                <Chip tone={theme.format === "ctf" ? "laser" : "neutral"}>
                  {theme.format || "ctf"}
                </Chip>
                <div className="grow">
                  <div className="title">{theme.name}</div>
                  <div className="sub">
                    {theme.size_bytes ? fmtBytes(theme.size_bytes) : "hosted catalog"}
                  </div>
                </div>
                <Btn
                  small
                  variant="primary"
                  disabled={!connected}
                  onClick={async () => {
                    await api.themeInstallRemote(theme.name, theme.url, theme.format);
                    toast(`theme installed: ${theme.name}`);
                    reload();
                  }}
                >
                  Install
                </Btn>
              </div>
            ))}
          </div>
        </section>
      ) : null}

      {themes.length === 0 ? (
        <Empty title="No themes installed" hint="Drop a .ptf or .ctf file to customize the XMB." />
      ) : (
        <div className="rows">
          {themes.map((theme) => (
            <div className="row" key={theme.name}>
              <Chip tone={theme.format === "ctf" ? "laser" : "neutral"}>{theme.format}</Chip>
              <div className="grow">
                <div className="title">{theme.name}</div>
              </div>
              <span className="size">{fmtBytes(theme.size_bytes)}</span>
              <Btn
                small
                variant="danger"
                onClick={async () => {
                  if (confirmDialog("Delete theme?", `${theme.name} will be removed.`)) {
                    await api.themeDelete(theme.name);
                    toast(`deleted ${theme.name}`);
                    reload();
                  }
                }}
              >
                Delete
              </Btn>
            </div>
          ))}
        </div>
      )}
    </>
  );
}

/* ---------- cfw ---------- */

function CfwPanel({ snap, reload, toast, refreshKey }: PageProps) {
  const [catalog, setCatalog] = useState<CatalogEntry[]>([]);
  const [installed, setInstalled] = useState<Record<string, boolean>>({});
  const connected = !!snap.volumes.find((v) => v.is_psp);

  useEffect(() => {
    void Promise.all([api.homebrewCatalog(), api.homebrewInstalled()])
      .then(([entries, installedList]) => {
        setCatalog(entries.filter((e) => e.category === "cfw"));
        const map: Record<string, boolean> = {};
        for (const entry of installedList) map[entry.id] = entry.installed;
        setInstalled(map);
      })
      .catch(() => setCatalog([]));
  }, [refreshKey]);

  return (
    <>
      <div className="note">
        Custom firmware is installed by copying loaders onto the stick, then
        launching the loader once on the console. ARK runs as a live CFW until
        you make it permanent with cIPL. Never pull the cable mid-install.
      </div>
      {catalog.map((entry) => (
        <article className="store-card" key={entry.id}>
          <div className="head">
            <span className="name">{entry.name}</span>
            <span className="tag">{entry.latest_tag ?? "unreleased"}</span>
            {installed[entry.id] ? <Chip tone="good">on stick</Chip> : null}
          </div>
          <div className="desc">{entry.description}</div>
          <div className="repo">{entry.repo}</div>
          {entry.note ? <div className="note">{entry.note}</div> : null}
          <div className="foot">
            <div className="spacer" />
            <Btn
              variant="primary"
              disabled={!entry.zip_url || !connected}
              onClick={async () => {
                if (!confirmDialog("Install custom firmware?", `${entry.name} ${entry.latest_tag ?? ""} will be copied onto the memory stick (ARK_01234 to PSP/SAVEDATA, loaders to PSP/GAME).`)) {
                  return;
                }
                await api.homebrewInstall(entry.id);
                toast(`${entry.name}: install queued`);
                reload();
              }}
            >
              Install {entry.name}
            </Btn>
          </div>
        </article>
      ))}
    </>
  );
}
