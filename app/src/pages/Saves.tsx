import { useEffect, useState } from "react";
import { api, pickers } from "../api";
import type { SaveEntry } from "../types";
import type { PageProps } from "../App";
import { Btn, Chip, Empty, confirmDialog, fmtBytes } from "../ui";

export default function Saves({ snap, reload, toast, refreshKey }: PageProps) {
  const [saves, setSaves] = useState<SaveEntry[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());

  const connected = !!snap.volumes.find((v) => v.is_psp);

  useEffect(() => {
    if (!connected) {
      setSaves([]);
      return;
    }
    void api
      .savedataList()
      .then(setSaves)
      .catch(() => setSaves([]));
  }, [connected, refreshKey]);

  const selectedList = [...selected];
  const usedBytes = saves.reduce((sum, s) => sum + s.size_bytes, 0);

  function toggle(folder: string) {
    setSelected((cur) => {
      const next = new Set(cur);
      if (next.has(folder)) {
        next.delete(folder);
      } else {
        next.add(folder);
      }
      return next;
    });
  }

  return (
    <div className="page">
      <div className="card panel">
        <div className="panel-head">
          <h2>Save data</h2>
          <div className="spacer" />
          <Chip tone="neutral">{fmtBytes(usedBytes)} across {saves.length} saves</Chip>
        </div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <Btn
            variant="primary"
            disabled={!connected || saves.length === 0}
            onClick={async () => {
              await api.savedataBackup([]);
              toast("backup queued: all saves");
            }}
          >
            Backup All
          </Btn>
          <Btn
            disabled={!connected || selected.size === 0}
            onClick={async () => {
              await api.savedataBackup(selectedList);
              toast(`backup queued: ${selected.size} save(s)`);
              setSelected(new Set());
            }}
          >
            Backup Selected ({selected.size})
          </Btn>
          <Btn
            disabled={!connected}
            onClick={async () => {
              const zip = await pickers.pickFile("Pick a backup zip", ["zip"]);
              if (!zip) return;
              await api.savedataRestore(zip);
              toast("restore queued");
            }}
          >
            Restore Zip
          </Btn>
        </div>
        <p style={{ margin: "10px 0 0", fontSize: 12.5, color: "var(--text-faint)" }}>
          Backups are zipped into the app data folder. ARK-4's ARK_01234 payload
          folder is never listed or touched as a save.
        </p>
      </div>

      {saves.length === 0 ? (
        <Empty title="No saves on the stick" hint="Save data appears in PSP/SAVEDATA once a game has saved." />
      ) : (
        <div className="rows">
          {saves.map((save) => (
            <div className="row" key={save.folder}>
              <input
                type="checkbox"
                checked={selected.has(save.folder)}
                onChange={() => toggle(save.folder)}
                aria-label={`Select ${save.title ?? save.folder}`}
              />
              <SaveIcon folder={save.folder} title={save.title ?? save.folder} hasIcon={save.has_icon} />
              <div className="grow">
                <div className="title">{save.title ?? save.folder}</div>
                <div className="sub">
                  {save.subtitle ? `${save.subtitle} - ` : ""}
                  {save.folder}
                  {save.title_id ? ` - ${save.title_id}` : ""}
                </div>
              </div>
              <span className="size">{fmtBytes(save.size_bytes)}</span>
              <Btn
                small
                onClick={async () => {
                  const dir = await api.savedataReveal(save.folder);
                  await api.reveal(dir);
                }}
              >
                Open
              </Btn>
              <Btn
                small
                variant="danger"
                onClick={async () => {
                  if (confirmDialog("Delete save?", `${save.title ?? save.folder} (${save.folder}) will be removed permanently.`)) {
                    await api.savedataDelete(save.folder);
                    toast(`deleted ${save.folder}`);
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
    </div>
  );
}

/** Save slot art: the save's own ICON0.PNG, landscape like the PSP. */
function SaveIcon({
  folder,
  title,
  hasIcon,
}: {
  folder: string;
  title: string;
  hasIcon: boolean;
}) {
  const [url, setUrl] = useState<string | null>(null);
  useEffect(() => {
    if (hasIcon) void api.savedataIcon(folder).then(setUrl);
  }, [folder, hasIcon]);
  return (
    <div className="save-cell">
      <div className="save-icon">
        {url ? (
          <img src={url} alt="" />
        ) : (
          <div className="cover-fallback">
            <span className="cf-title">{title}</span>
            <span className="cf-serial">{folder}</span>
          </div>
        )}
      </div>
      <div className="save-caption" title={title}>
        {title}
      </div>
    </div>
  );
}
