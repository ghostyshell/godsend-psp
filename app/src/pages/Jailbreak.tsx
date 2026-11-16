import { useEffect, useMemo, useState } from "react";
import { api } from "../api";
import type { CatalogEntry, CfwState } from "../types";
import type { PageProps } from "../App";
import { Btn, Chip, Empty, confirmDialog } from "../ui";

export default function Jailbreak({ snap, toast, refreshKey }: PageProps) {
  const [state, setState] = useState<CfwState | null>(null);
  const [plan, setPlan] = useState<string[]>([]);
  const [catalog, setCatalog] = useState<CatalogEntry[]>([]);
  const [installed, setInstalled] = useState<Record<string, boolean>>({});

  const connected = !!snap.volumes.find((v) => v.is_psp);

  useEffect(() => {
    void api
      .homebrewCatalog()
      .then((entries) => setCatalog(entries.filter((entry) => entry.category === "cfw")))
      .catch(() => setCatalog([]));
  }, [refreshKey]);

  useEffect(() => {
    if (!connected) {
      setInstalled({});
      return;
    }
    void api
      .homebrewInstalled()
      .then((installedList) => {
        const map: Record<string, boolean> = {};
        for (const entry of installedList) map[entry.id] = entry.installed;
        setInstalled(map);
      })
      .catch(() => setInstalled({}));
  }, [connected, refreshKey]);

  useEffect(() => {
    if (!connected) {
      setState(null);
      setPlan([]);
      return;
    }
    void Promise.all([api.cfwDetect(), api.cfwPlan()])
      .then(([detected, steps]) => {
        setState(detected);
        setPlan(steps);
      })
      .catch(() => {
        setState(null);
        setPlan([]);
      });
  }, [connected, refreshKey]);

  const anyCfw = useMemo(
    () => !!state && (state.ark4 || state.ark5 || state.pro || state.me),
    [state]
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
    if (
      !confirmDialog(
        "Install custom firmware?",
        `${entry.name} ${entry.latest_tag ?? ""} will be copied onto the memory stick (ARK_01234 to PSP/SAVEDATA, loaders to PSP/GAME). Never pull the cable mid-install.`
      )
    ) {
      return;
    }
    try {
      await api.homebrewInstall(entry.id);
      toast(`${entry.name}: install queued`);
    } catch (err) {
      toast(`install failed: ${String(err)}`);
    }
  }

  return (
    <div className="page">
      <section className="card panel">
        <div className="panel-head">
          <h2>Firmware detected</h2>
          <div className="spacer" />
          {connected ? (
            <Chip tone={anyCfw ? "good" : "warn"}>
              {anyCfw ? "CFW present" : "stock firmware"}
            </Chip>
          ) : (
            <Chip tone="bad">PSP offline</Chip>
          )}
        </div>

        {!connected ? (
          <Empty
            title="Connect a PSP"
            hint="Plug the console in over USB and choose USB Connection. Detection reads the memory stick only."
          />
        ) : !state ? (
          <p style={{ margin: 0, fontSize: 12.5, color: "var(--text-faint)" }}>
            Scanning the memory stick for CFW folders and loaders.
          </p>
        ) : (
          <>
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
              <Chip tone={state.ark4 ? "good" : "neutral"}>
                {state.ark4 ? "ARK-4" : "No ARK-4"}
              </Chip>
              <Chip tone={state.ark5 ? "good" : "neutral"}>
                {state.ark5 ? "ARK-5" : "No ARK-5"}
              </Chip>
              <Chip tone={state.pro ? "good" : "neutral"}>
                {state.pro ? "PRO" : "No PRO"}
              </Chip>
              <Chip tone={state.me ? "good" : "neutral"}>
                {state.me ? "ME" : "No ME"}
              </Chip>
              <Chip tone={state.time_machine ? "good" : "neutral"}>
                {state.time_machine ? "Time Machine" : "No Time Machine"}
              </Chip>
              <Chip tone={state.recovery ? "good" : "neutral"}>
                {state.recovery ? "Recovery" : "No recovery"}
              </Chip>
              <Chip tone={state.iso_driver ? "good" : "neutral"}>
                {state.iso_driver ? "ISO driver" : "No ISO driver"}
              </Chip>
              <Chip tone={state.plugins.length ? "laser" : "neutral"}>
                {`${state.plugins.length} plugin${state.plugins.length === 1 ? "" : "s"}`}
              </Chip>
            </div>
            {state.plugins.length ? (
              <div
                style={{
                  marginTop: 12,
                  display: "flex",
                  gap: 6,
                  flexWrap: "wrap",
                  fontFamily: "var(--mono)",
                  fontSize: 11.5,
                  color: "var(--text-dim)",
                }}
              >
                {state.plugins.map((path) => (
                  <span key={path}>{path}</span>
                ))}
              </div>
            ) : null}
          </>
        )}
      </section>

      <section className="card panel">
        <div className="panel-head">
          <h2>Install firmware</h2>
          <div className="spacer" />
          <span className="section-caption">live CFW loaders</span>
        </div>
        {catalog.length === 0 ? (
          <Empty
            title="No CFW in the catalog"
            hint="The homebrew catalog did not return any custom firmware entries."
          />
        ) : (
          <div className="store-grid">
            {catalog.map((entry) => (
              <article className="store-card" key={entry.id}>
                <div className="head">
                  <span className="name">{entry.name}</span>
                  <span className="tag">{entry.latest_tag ?? "unreleased"}</span>
                </div>
                <div className="desc">{entry.description}</div>
                <div className="repo">{entry.repo}</div>
                {entry.note ? <div className="note">{entry.note}</div> : null}
                <div className="foot">
                  {installed[entry.id] ? <Chip tone="good">on stick</Chip> : null}
                  <Chip tone={entry.zip_url ? "laser" : "warn"}>
                    {entry.zip_url ? "zip ready" : "no zip"}
                  </Chip>
                  <div className="spacer" />
                  <Btn
                    variant="primary"
                    disabled={!entry.zip_url || !connected}
                    onClick={() => void install(entry)}
                  >
                    {installed[entry.id] ? "Reinstall" : "Install"}
                  </Btn>
                </div>
              </article>
            ))}
          </div>
        )}
      </section>

      <section className="card panel">
        <div className="panel-head">
          <h2>Migration plan</h2>
          <div className="spacer" />
          <Chip tone="bad">New cIPL is permanent</Chip>
        </div>
        <div className="note">
          New cIPL writes the firmware into the console flash. It survives a
          memory stick wipe and is the point of no easy return, so only run it
          after ARK-5 boots cleanly from the loader.
        </div>
        {plan.length === 0 ? (
          <Empty
            title="No plan yet"
            hint="Connect the PSP so the stick can be scanned, then the ordered steps appear here."
          />
        ) : (
          <div className="rows">
            {plan.map((step, index) => (
              <div className="row" key={step}>
                <Chip tone="laser">{index + 1}</Chip>
                <div className="grow">
                  <div className="title">{step}</div>
                </div>
              </div>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
