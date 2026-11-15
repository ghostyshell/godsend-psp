import { useCallback, useEffect, useMemo, useState } from "react";
import { api, listen } from "./api";
import type { Job, Snapshot } from "./types";
import { PAGE_TITLES } from "./types";
import { Chip } from "./ui";
import {
  House,
  Library as LibraryIcon,
  Store as StoreIcon,
  Download as DownloadIcon,
  FolderOpen,
  Package as PackageIcon,
  Music as MusicIcon,
  Film as FilmIcon,
  Disc3,
  Save as SaveIcon,
  Wrench,
  Settings as SettingsIcon,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import logo from "./assets/logo.png";
import Home from "./pages/Home";
import Library from "./pages/Library";
import GameStore from "./pages/GameStore";
import Downloads from "./pages/Downloads";
import Files from "./pages/Files";
import Store from "./pages/Store";
import Music from "./pages/Music";
import Video from "./pages/Video";
import Iso from "./pages/Iso";
import Saves from "./pages/Saves";
import Mods from "./pages/Mods";
import Settings from "./pages/Settings";

type Page =
  | "home"
  | "library"
  | "gamestore"
  | "downloads"
  | "files"
  | "store"
  | "music"
  | "video"
  | "iso"
  | "saves"
  | "mods"
  | "settings";

const NAV: { id: Page; label: string; icon: LucideIcon }[] = [
  { id: "home", label: "Console", icon: House },
  { id: "library", label: "Library", icon: LibraryIcon },
  { id: "gamestore", label: "Store", icon: StoreIcon },
  { id: "downloads", label: "Downloads", icon: DownloadIcon },
  { id: "files", label: "Files", icon: FolderOpen },
  { id: "store", label: "Homebrew", icon: PackageIcon },
  { id: "music", label: "Music", icon: MusicIcon },
  { id: "video", label: "Video", icon: FilmIcon },
  { id: "iso", label: "Images", icon: Disc3 },
  { id: "saves", label: "Saves", icon: SaveIcon },
  { id: "mods", label: "Mods", icon: Wrench },
  { id: "settings", label: "Settings", icon: SettingsIcon },
];

const VERSION = "0.2.12";

export interface PageProps {
  snap: Snapshot;
  reload: () => void;
  toast: (message: string) => void;
  refreshKey: number;
}

export default function App() {
  const [page, setPage] = useState<Page>("home");
  const [snap, setSnap] = useState<Snapshot | null>(null);
  const [toastMsg, setToastMsg] = useState<string | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);

  const toast = useCallback((message: string) => {
    setToastMsg(message);
    window.setTimeout(() => setToastMsg((cur) => (cur === message ? null : cur)), 4200);
  }, []);

  const reload = useCallback(() => setRefreshKey((k) => k + 1), []);

  const loadSnapshot = useCallback(async () => {
    try {
      const next = await api.snapshot();
      setSnap(next);
    } catch (err) {
      console.error("snapshot failed", err);
    }
  }, []);

  useEffect(() => {
    void loadSnapshot();
    const unlisteners: Promise<() => void>[] = [
      listen<unknown>("device://changed", () => {
        void loadSnapshot();
        setRefreshKey((k) => k + 1);
      }),
      listen<{ id: number; done: number; total: number }>("job://progress", (update) => {
        setSnap((cur) =>
          cur
            ? {
                ...cur,
                jobs: cur.jobs.map((job) =>
                  job.id === update.id
                    ? { ...job, done_bytes: update.done, total_bytes: update.total }
                    : job
                ),
              }
            : cur
        );
      }),
      listen<Job>("job://done", (job) => {
        setSnap((cur) =>
          cur
            ? {
                ...cur,
                jobs: cur.jobs.map((j) => (j.id === job.id ? job : j)),
              }
            : cur
        );
        if (job.state === "done") {
          toast(`${job.label}: ${job.message || "done"}`);
        } else if (job.state === "failed") {
          toast(`${job.label} failed: ${job.message}`);
        }
        setRefreshKey((k) => k + 1);
      }),
    ];
    return () => {
      void Promise.all(unlisteners).then((uns) => uns.forEach((fn) => fn()));
    };
  }, [loadSnapshot, toast]);

  const activePsp = useMemo(
    () => snap?.volumes.find((v) => v.is_psp) ?? null,
    [snap]
  );

  const ffmpegOk = useMemo(
    () => snap?.doctor.find((c) => c.name === "ffmpeg")?.ok ?? false,
    [snap]
  );

  if (!snap) {
    return (
      <div className="shell" aria-busy="true">
        <div className="backdrop">
          <div className="orb cyan" />
          <div className="orb indigo" />
          <div className="orb brass" />
        </div>
      </div>
    );
  }

  const pageProps: PageProps = { snap, reload, toast, refreshKey };

  return (
    <div className="shell">
      <div className="backdrop" aria-hidden="true">
        <div className="orb cyan" />
        <div className="orb indigo" />
        <div className="orb brass" />
      </div>

      <a className="skip-link" href="#workspace">
        Skip to content
      </a>

      <header className="topbar">
        <div className="brand">
          <img className="mark" src={logo} alt="" aria-hidden="true" />
          <div>
            <div className="eyebrow">GODSEND PSP</div>
            <div className="page-title" aria-live="polite">
              {PAGE_TITLES[page]}
            </div>
          </div>
        </div>
        <div className="spacer" />
        {activePsp ? (
          <Chip tone="good" title={activePsp.mount}>
            PSP Connected
          </Chip>
        ) : (
          <Chip tone="bad">PSP Offline</Chip>
        )}
      </header>

      <nav className="rail" aria-label="Sections">
        <div className="rail-caption">Device</div>
        {NAV.map((item) => (
          <button
            key={item.id}
            className={`rail-item ${page === item.id ? "on" : ""}`}
            aria-current={page === item.id ? "page" : undefined}
            onClick={() => setPage(item.id)}
            onKeyDown={(e) => {
              if (e.altKey && e.key >= "1" && e.key <= "9") {
                const idx = Number(e.key) - 1;
                if (NAV[idx]) setPage(NAV[idx].id);
              }
            }}
          >
            <item.icon className="glyph-icon" size={17} aria-hidden="true" />
            <span className="label">{item.label}</span>
          </button>
        ))}
      </nav>

      <main className="workspace with-inspector" id="workspace" tabIndex={-1}>
        {page === "home" && <Home {...pageProps} />}
        {page === "library" && <Library {...pageProps} />}
        {page === "gamestore" && <GameStore {...pageProps} />}
        {page === "downloads" && <Downloads {...pageProps} />}
        {page === "files" && <Files {...pageProps} />}
        {page === "store" && <Store {...pageProps} />}
        {page === "music" && <Music {...pageProps} />}
        {page === "video" && <Video {...pageProps} />}
        {page === "iso" && <Iso {...pageProps} />}
        {page === "saves" && <Saves {...pageProps} />}
        {page === "mods" && <Mods {...pageProps} />}
        {page === "settings" && <Settings {...pageProps} />}
      </main>

      <aside className="inspector" aria-label="Session">
        <div className="session-card">
          <h3>Device</h3>
          {activePsp ? (
            <>
              <div className="kv">
                <span className="k">volume</span>
                <span className="v" title={activePsp.mount}>
                  {activePsp.label}
                </span>
              </div>
              <div className="kv">
                <span className="k">free</span>
                <span className="v">{fmt(activePsp.free_bytes)}</span>
              </div>
              <div className="kv">
                <span className="k">total</span>
                <span className="v">{fmt(activePsp.total_bytes)}</span>
              </div>
              <div style={{ marginTop: 8 }}>
                <div className="bar" aria-hidden="true">
                  <div
                    className="fill"
                    style={{
                      width: `${
                        activePsp.total_bytes
                          ? Math.round(
                              ((activePsp.total_bytes - activePsp.free_bytes) /
                                activePsp.total_bytes) *
                                100
                            )
                          : 0
                      }%`,
                    }}
                  />
                </div>
              </div>
              <div className="kv" style={{ marginTop: 8 }}>
                <span className="k">mount</span>
                <span className="v" title={activePsp.mount}>
                  {activePsp.mount}
                </span>
              </div>
            </>
          ) : (
            <p style={{ margin: 0, fontSize: 12.5, color: "var(--text-faint)" }}>
              Connect the PSP with the USB cable and pick "USB Connection" on the
              console.
            </p>
          )}
        </div>

        <div className="queue-card">
          <div className="panel-head" style={{ marginBottom: 0 }}>
            <h3 style={{ margin: 0 }}>Queue</h3>
            <div className="spacer" />
            {snap.jobs.some((j) => j.state !== "running") ? (
              <button
                className="btn small"
                onClick={async () => {
                  await api.jobsClear();
                  void loadSnapshot();
                }}
              >
                Clear
              </button>
            ) : null}
          </div>
          <div className="rows">
            {snap.jobs.length === 0 ? (
              <p style={{ margin: 0, fontSize: 12.5, color: "var(--text-faint)" }}>
                No jobs yet. Transfers, installs and conversions show up here.
              </p>
            ) : (
              snap.jobs.slice(0, 8).map((job) => <JobRow key={job.id} job={job} onCancel={async () => {
                await api.jobCancel(job.id);
                void loadSnapshot();
              }} />)
            )}
          </div>
        </div>
      </aside>

      <footer className="statusbar">
        <span>
          <span className={`status-dot ${activePsp ? "ok" : "off"}`} />
          {activePsp ? activePsp.mount : "no PSP"}
        </span>
        <span>
          <span className={`status-dot ${ffmpegOk ? "ok" : "idle"}`} />
          ffmpeg {ffmpegOk ? "ready" : "missing"}
        </span>
        <span className="grow" />
        <button
          className="btn small"
          disabled={!activePsp}
          onClick={async () => {
            if (!activePsp) return;
            try {
              await api.eject(activePsp.mount);
              toast("PSP ejected; you can unplug the cable");
              void loadSnapshot();
            } catch (err) {
              toast(`eject failed: ${String(err)}`);
            }
          }}
        >
          Eject
        </button>
        <span>godsend-psp v{VERSION}</span>
      </footer>

      {toastMsg ? (
        <div className="toast" role="status" aria-live="polite">
          <span className="chip laser">
            <span className="dot" />
            PSP
          </span>
          {toastMsg}
        </div>
      ) : null}
    </div>
  );
}

function JobRow({ job, onCancel }: { job: Job; onCancel: () => void }) {
  const running = job.state === "running";
  return (
    <div className="job-row">
      <div className="head">
        <span className="label" title={job.label}>
          {job.label}
        </span>
        {running ? (
          <button className="btn small" onClick={onCancel} title="Cancel job">
            Cancel
          </button>
        ) : (
          <Chip tone={job.state === "done" ? "good" : job.state === "failed" ? "bad" : "warn"}>
            {job.state}
          </Chip>
        )}
      </div>
      {running ? (
        <div style={{ marginTop: 6 }}>
          <div className="bar live">
            <div
              className="fill"
              style={{
                width: `${
                  job.total_bytes ? Math.round((job.done_bytes / job.total_bytes) * 100) : 12
                }%`,
              }}
            />
          </div>
        </div>
      ) : (
        <div className="msg">{job.message}</div>
      )}
    </div>
  );
}

function fmt(bytes: number): string {
  if (!bytes) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.min(units.length - 1, Math.floor(Math.log(bytes) / Math.log(1024)));
  const value = bytes / Math.pow(1024, i);
  return `${i === 0 ? bytes : value.toFixed(1)} ${units[i]}`;
}
