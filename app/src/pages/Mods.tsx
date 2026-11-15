import { useEffect, useState } from "react";
import { api, pickers } from "../api";
import type { CatalogEntry, PluginsView, RemoteTheme, ThemeEntry } from "../types";
import type { PageProps } from "../App";
import { Btn, Chip, Empty, Seg, Toggle, confirmDialog, fmtBytes } from "../ui";

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

function ThemesPanel({ snap, reload, toast, refreshKey }: PageProps) {
  const [themes, setThemes] = useState<ThemeEntry[]>([]);
  const [remote, setRemote] = useState<RemoteTheme[]>([]);
  const connected = !!snap.volumes.find((v) => v.is_psp);

  useEffect(() => {
    if (!connected) {
      setThemes([]);
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
  }, [connected, refreshKey]);

  return (
    <>
      <div className="card panel">
        <div className="panel-head">
          <h2>XMB themes</h2>
          <div className="spacer" />
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
