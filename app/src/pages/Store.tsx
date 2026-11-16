import { useEffect, useMemo, useState } from "react";
import { api, pickers } from "../api";
import type { CatalogEntry } from "../types";
import type { PageProps } from "../App";
import { Btn, Chip, Empty, Seg } from "../ui";

type Category = "all" | "app" | "emu" | "cfw";

export default function Store({ snap, toast, refreshKey }: PageProps) {
  const [catalog, setCatalog] = useState<CatalogEntry[]>([]);
  const [installed, setInstalled] = useState<Record<string, boolean>>({});
  const [category, setCategory] = useState<Category>("all");
  const [busy, setBusy] = useState<string | null>(null);

  const connected = !!snap.volumes.find((v) => v.is_psp);

  useEffect(() => {
    void Promise.all([api.homebrewCatalog(), api.homebrewInstalled()])
      .then(([entries, installedList]) => {
        setCatalog(entries);
        const map: Record<string, boolean> = {};
        for (const entry of installedList) map[entry.id] = entry.installed;
        setInstalled(map);
      })
      .catch(() => {
        setCatalog([]);
      });
  }, [refreshKey, connected]);

  const visible = useMemo(
    () =>
      catalog.filter((entry) =>
        category === "all" ? true : category === "emu" ? entry.category === "emu" : entry.category === category
      ),
    [catalog, category]
  );

  async function install(entry: CatalogEntry) {
    if (!connected) {
      toast("connect a PSP first");
      return;
    }
    if (!entry.zip_url) {
      toast(`${entry.name}: no zip asset on the latest release`);
      return;
    }
    setBusy(entry.id);
    try {
      await api.homebrewInstall(entry.id);
      toast(`${entry.name}: install queued`);
    } catch (err) {
      toast(`install failed: ${String(err)}`);
    } finally {
      setBusy(null);
    }
  }

  return (
    <div className="page">
      <div style={{ display: "flex", gap: 10, alignItems: "center", flexWrap: "wrap" }}>
        <Seg
          ariaLabel="Homebrew category"
          value={category}
          onChange={setCategory}
          options={[
            { value: "all", label: "All" },
            { value: "app", label: "Apps" },
            { value: "emu", label: "Emulators" },
            { value: "cfw", label: "CFW" },
          ]}
        />
        <div style={{ flex: 1 }} />
        <Btn
          onClick={async () => {
            try {
              const refreshed = await api.homebrewRefresh();
              setCatalog(refreshed);
              toast("catalog refreshed from GitHub releases");
            } catch (err) {
              toast(`refresh failed: ${String(err)}`);
            }
          }}
        >
          Refresh
        </Btn>
        <Btn
          variant="primary"
          disabled={!connected}
          onClick={async () => {
            const zips = await pickers.pickFiles("Pick a homebrew zip", ["zip", "rar"]);
            if (!zips.length) return;
            for (const zip of zips) {
              await api.homebrewImport(zip);
            }
            toast(`importing ${zips.length} archive(s)`);
          }}
        >
          Import Zip
        </Btn>
      </div>

      {visible.length === 0 ? (
        <Empty
          title="Catalog unavailable"
          hint="The bundled catalog failed to load or was filtered out. Try Refresh."
          action={<Btn onClick={() => void api.homebrewRefresh()}>Refresh now</Btn>}
        />
      ) : (
        <div className="store-grid">
          {visible.map((entry) => (
            <article className="store-card" key={entry.id}>
              <div className="head">
                <span className="name">{entry.name}</span>
                <span className="tag">{entry.latest_tag ?? "unreleased"}</span>
              </div>
              <div className="desc">{entry.description}</div>
              <div className="repo">{entry.repo}</div>
              {entry.note ? <div className="note">{entry.note}</div> : null}
              <div className="foot">
                {installed[entry.id] ? <Chip tone="good">installed</Chip> : null}
                <Chip tone={entry.zip_url ? "laser" : "warn"}>
                  {entry.zip_url ? "zip ready" : "no zip"}
                </Chip>
                <div className="spacer" />
                <Btn
                  variant="primary"
                  disabled={!entry.zip_url || !connected || busy === entry.id}
                  onClick={() => void install(entry)}
                >
                  {installed[entry.id] ? "Reinstall" : "Install"}
                </Btn>
              </div>
            </article>
          ))}
        </div>
      )}

      <p style={{ margin: 0, fontSize: 12.5, color: "var(--text-faint)" }}>
        Homebrew needs custom firmware (ARK-4 or similar) on the console. CFW
        entries install their loaders onto the stick; launch the loader on the
        PSP to boot the firmware. Any homebrew zip can also be imported locally
        and the layout is detected automatically.
      </p>
    </div>
  );
}
