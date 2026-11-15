import { useEffect, useState } from "react";
import { api } from "../api";
import type { FmEntry } from "../types";
import type { PageProps } from "../App";
import { Btn, Chip, Empty, fmtBytes } from "../ui";

export default function Files({ snap, toast, refreshKey }: PageProps) {
  const activePsp = snap.volumes.find((v) => v.is_psp) ?? null;
  const [localDir, setLocalDir] = useState("");
  const [localEntries, setLocalEntries] = useState<FmEntry[]>([]);
  const [pspDir, setPspDir] = useState<string | null>(null);
  const [pspEntries, setPspEntries] = useState<FmEntry[]>([]);

  async function loadLocal(path?: string) {
    if (path) {
      setLocalDir(path);
    }
    try {
      const result = await api.downloadsList();
      const dir = path ?? result.dir;
      setLocalDir(dir);
      setLocalEntries(await api.fmList(dir));
    } catch {
      setLocalEntries([]);
    }
  }

  async function loadPsp(dir?: string) {
    if (!activePsp) return;
    const target = dir ?? pspDir ?? activePsp.mount;
    setPspDir(target);
    try {
      setPspEntries(await api.fmList(target));
    } catch {
      setPspEntries([]);
    }
  }

  useEffect(() => {
    void loadLocal();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [refreshKey]);

  useEffect(() => {
    if (activePsp) {
      void loadPsp(pspDir ?? activePsp.mount);
    } else {
      setPspEntries([]);
      setPspDir(null);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activePsp?.mount]);

  const localParent = localDir ? localDir.replace(/\/[^/]+$/, "") : "";
  const pspParent = pspDir ? pspDir.replace(/\/[^/]+$/, "") : "";
  const canLocalUp = localDir && localParent && localDir !== localParent;
  const canPspUp = pspDir && pspParent && pspDir !== pspParent && activePsp && pspDir !== activePsp.mount;

  return (
    <div className="page">
      <div className="card panel">
        <div className="panel-head">
          <h2>Files</h2>
          <div className="spacer" />
          <Chip tone={activePsp ? "good" : "bad"}>{activePsp ? "PSP mounted" : "no device"}</Chip>
        </div>
        <p style={{ margin: 0, fontSize: 12.5, color: "var(--text-faint)" }}>
          Two panes: local storage on the left, the memory stick on the right. Copy
          files between them; ISO/CSO go to the ISO folder, EBOOT folders go to
          PSP/GAME.
        </p>
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
        <section className="card panel">
          <div className="panel-head">
            <h2 style={{ fontSize: 14 }}>Local</h2>
            <div className="spacer" />
            <Chip tone="neutral" title={localDir}>
              {localDir.split("/").pop() || "root"}
            </Chip>
          </div>
          <div style={{ display: "flex", gap: 6, marginBottom: 8 }}>
            <Btn small disabled={!canLocalUp} onClick={() => void loadLocal(localParent)}>
              Up
            </Btn>
            <Btn
              small
              onClick={async () => {
                const name = window.prompt("New folder name");
                if (!name) return;
                await api.fmMkdir(localDir, name);
                toast(`created ${name}`);
                void loadLocal();
              }}
            >
              New Folder
            </Btn>
          </div>
          <BrowserList
            entries={localEntries}
            onNavigate={(entry) => {
              if (entry.is_dir) void loadLocal(entry.path);
            }}
            actions={(entry) =>
              entry.is_dir ? null : (
                <Btn
                  small
                  variant="primary"
                  disabled={!activePsp}
                  onClick={async () => {
                    const kind = entry.kind === "iso" || entry.kind === "cso" ? "iso" : "game";
                    await api.fmToPsp(entry.path, kind);
                    toast(`copying ${entry.name} to PSP`);
                  }}
                >
                  To PSP
                </Btn>
              )
            }
            onDelete={async (entry) => {
              if (!window.confirm(`Delete ${entry.name}?`)) return;
              await api.fmDelete(entry.path);
              toast(`deleted ${entry.name}`);
              void loadLocal();
            }}
          />
        </section>

        <section className="card panel">
          <div className="panel-head">
            <h2 style={{ fontSize: 14 }}>Memory Stick</h2>
            <div className="spacer" />
            <Chip tone="neutral" title={pspDir ?? ""}>
              {pspDir?.split("/").pop() || "offline"}
            </Chip>
          </div>
          <div style={{ display: "flex", gap: 6, marginBottom: 8 }}>
            <Btn small disabled={!canPspUp} onClick={() => void loadPsp(pspParent)}>
              Up
            </Btn>
            <Btn
              small
              disabled={!activePsp}
              onClick={async () => {
                const name = window.prompt("New folder name");
                if (!name || !pspDir) return;
                await api.fmMkdir(pspDir, name);
                toast(`created ${name}`);
                void loadPsp();
              }}
            >
              New Folder
            </Btn>
          </div>
          {!activePsp ? (
            <Empty title="No PSP" hint="Connect the console in USB mode." />
          ) : (
            <BrowserList
              entries={pspEntries}
              onNavigate={(entry) => {
                if (entry.is_dir) void loadPsp(entry.path);
              }}
              actions={(entry) =>
                entry.kind === "iso" || entry.kind === "cso" || entry.kind === "eboot_dir" ? (
                  <Btn
                    small
                    onClick={async () => {
                      const id =
                        entry.kind === "eboot_dir"
                          ? `game/${entry.name}`
                          : `iso/${entry.name}`;
                      await api.fmFromPsp(id);
                      toast(`copying ${entry.name} to local`);
                    }}
                  >
                    To Local
                  </Btn>
                ) : null
              }
              onDelete={async (entry) => {
                if (!window.confirm(`Delete ${entry.name}?`)) return;
                await api.fmDelete(entry.path);
                toast(`deleted ${entry.name}`);
                void loadPsp();
              }}
            />
          )}
        </section>
      </div>
    </div>
  );
}

function BrowserList({
  entries,
  onNavigate,
  actions,
  onDelete,
}: {
  entries: FmEntry[];
  onNavigate: (entry: FmEntry) => void;
  actions: (entry: FmEntry) => React.ReactNode;
  onDelete: (entry: FmEntry) => void;
}) {
  if (entries.length === 0) {
    return <Empty title="Empty folder" />;
  }
  return (
    <div className="rows" style={{ maxHeight: 420, overflowY: "auto" }}>
      {entries.map((entry) => (
        <div className="row" key={entry.path}>
          <Chip tone={entry.is_dir ? "neutral" : "laser"}>
            {entry.is_dir ? "dir" : entry.kind === "other" ? "file" : entry.kind}
          </Chip>
          <div className="grow">
            <div
              className="title"
              style={{ cursor: entry.is_dir ? "pointer" : "default" }}
              title={entry.path}
              onClick={() => onNavigate(entry)}
            >
              {entry.name}
            </div>
          </div>
          <span className="size">{entry.is_dir ? "" : fmtBytes(entry.size_bytes)}</span>
          {actions(entry)}
          <Btn small variant="danger" onClick={() => onDelete(entry)}>
            Delete
          </Btn>
        </div>
      ))}
    </div>
  );
}
