import { useEffect, useState } from "react";
import { api } from "../api";
import type { LibraryView, SaveEntry } from "../types";
import type { PageProps } from "../App";
import { Btn, Chip, Eyebrow, fmtBytes } from "../ui";

export default function Home({ snap, reload, toast, refreshKey }: PageProps) {
  const [lib, setLib] = useState<LibraryView | null>(null);
  const [saves, setSaves] = useState<SaveEntry[] | null>(null);
  const [checks, setChecks] = useState<typeof snap.doctor>(snap.doctor);

  const activePsp = snap.volumes.find((v) => v.is_psp) ?? null;

  useEffect(() => {
    if (!activePsp) {
      setLib(null);
      setSaves(null);
      return;
    }
    void Promise.all([api.libraryScan(), api.savedataList()])
      .then(([library, saveList]) => {
        setLib(library);
        setSaves(saveList);
      })
      .catch(() => {
        setLib(null);
        setSaves(null);
      });
  }, [activePsp, refreshKey]);

  useEffect(() => {
    setChecks(snap.doctor);
  }, [snap.doctor, refreshKey]);

  const used = activePsp ? activePsp.total_bytes - activePsp.free_bytes : 0;

  return (
    <div className="page">
      <section className="hero">
        <div className="top">
          <div style={{ flex: 1 }}>
            <Eyebrow>MEMORY STICK</Eyebrow>
            <h1>{activePsp ? activePsp.label : "No PSP detected"}</h1>
            <div className="sub">
              {activePsp
                ? `${activePsp.mount} in USB mode`
                : "Connect the PSP by USB and choose USB Connection on the console"}
            </div>
          </div>
          {activePsp ? <Chip tone="good" live>Online</Chip> : <Chip tone="bad">Offline</Chip>}
        </div>

        {activePsp ? (
          <>
            <div style={{ marginTop: 16 }}>
              <div className="bar">
                <div
                  className="fill"
                  style={{
                    width: `${
                      activePsp.total_bytes
                        ? Math.round((used / activePsp.total_bytes) * 100)
                        : 0
                    }%`,
                  }}
                />
              </div>
              <div className="sub" style={{ marginTop: 6 }}>
                {fmtBytes(used)} used of {fmtBytes(activePsp.total_bytes)} ({fmtBytes(activePsp.free_bytes)} free)
              </div>
            </div>

            <div className="stats">
              <div className="stat">
                <div className="v">{lib?.games.filter((g) => g.kind !== "loader").length ?? "-"}</div>
                <div className="k">Games & Homebrew</div>
              </div>
              <div className="stat">
                <div className="v">{lib?.images.length ?? "-"}</div>
                <div className="k">Disc Images</div>
              </div>
              <div className="stat">
                <div className="v">{saves?.length ?? "-"}</div>
                <div className="k">Save Folders</div>
              </div>
              <div className="stat">
                <div className="v">{fmtBytes(
                  (lib?.games.reduce((sum, g) => sum + g.size_bytes, 0) ?? 0) +
                    (lib?.images.reduce((sum, i) => sum + i.size_bytes, 0) ?? 0)
                )}</div>
                <div className="k">Library Size</div>
              </div>
            </div>
          </>
        ) : null}

        <div className="quick">
          <Btn
            variant="primary"
            onClick={async () => {
              const volumes = await api.rescan();
              const found = volumes.filter((v) => v.is_psp).length;
              toast(found ? `scan: ${found} PSP volume(s) found` : "scan: no PSP volumes");
              reload();
            }}
          >
            Rescan
          </Btn>
          <Btn
            disabled={!activePsp}
            onClick={async () => {
              if (activePsp) {
                await api.reveal(activePsp.mount);
              }
            }}
          >
            Open Stick
          </Btn>
          <Btn
            disabled={!activePsp}
            onClick={async () => {
              if (!activePsp) return;
              try {
                await api.eject(activePsp.mount);
                toast("PSP ejected; unplug when the console leaves USB mode");
                reload();
              } catch (err) {
                toast(String(err));
              }
            }}
          >
            Eject Safely
          </Btn>
          <Btn
            onClick={async () => {
              setChecks(await api.doctorRun());
            }}
          >
            Run Checks
          </Btn>
        </div>
      </section>

      <section className="card panel">
        <div className="panel-head">
          <h2>Environment</h2>
          <div className="spacer" />
          <Chip tone={checks.every((c) => c.ok) ? "good" : "warn"}>
            {checks.filter((c) => c.ok).length}/{checks.length} ok
          </Chip>
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
          <h2>Recent Activity</h2>
          <div className="spacer" />
          <span className="section-caption">jobs journal</span>
        </div>
        {snap.jobs.length === 0 ? (
          <p style={{ margin: 0, fontSize: 13, color: "var(--text-faint)" }}>
            Nothing yet. Push music, install homebrew or convert an ISO and it
            will be journaled here.
          </p>
        ) : (
          <div className="rows">
            {snap.jobs.slice(0, 5).map((job) => (
              <div className="row" key={job.id}>
                <Chip
                  tone={job.state === "done" ? "good" : job.state === "running" ? "laser" : job.state === "failed" ? "bad" : "warn"}
                >
                  {job.kind}
                </Chip>
                <div className="grow">
                  <div className="title">{job.label}</div>
                  <div className="sub">{job.message || job.state}</div>
                </div>
                <span className="size">{new Date(job.started_at * 1000).toLocaleTimeString()}</span>
              </div>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
