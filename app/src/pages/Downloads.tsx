import { useEffect, useState } from "react";
import { api, pickers } from "../api";
import type { FmEntry } from "../types";
import type { PageProps } from "../App";
import { Btn, Chip, Empty, fmtBytes, progressText } from "../ui";

export default function Downloads({ snap, reload, toast, refreshKey }: PageProps) {
  const [dir, setDir] = useState("");
  const [files, setFiles] = useState<FmEntry[]>([]);
  const connected = !!snap.volumes.find((v) => v.is_psp);

  async function load() {
    try {
      const result = await api.downloadsList();
      setDir(result.dir);
      setFiles(result.files);
    } catch {
      setFiles([]);
    }
  }

  useEffect(() => {
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [refreshKey]);

  const downloads = snap.jobs.filter((j) => j.kind === "download");

  return (
    <div className="page">
      <div className="card panel">
        <div className="panel-head">
          <h2>Downloads</h2>
          <div className="spacer" />
          <Btn small onClick={() => void api.openDataDir()}>
            Data Folder
          </Btn>
          <Btn
            small
            onClick={async () => {
              const chosen = await pickers.pickDirectory("Pick download folder");
              if (!chosen) return;
              await api.downloadDirSet(chosen);
              toast(`downloads now go to ${chosen}`);
              reload();
            }}
          >
            Change Folder
          </Btn>
        </div>
        <div className="kv">
          <span className="k">destination</span>
          <span className="v" title={dir}>
            {dir || "-"}
          </span>
        </div>
        <div className="kv">
          <span className="k">active</span>
          <span className="v">{downloads.filter((j) => j.state === "running").length}</span>
        </div>
      </div>

      {downloads.length > 0 ? (
        <section className="card panel">
          <div className="panel-head">
            <h2>Queue</h2>
            <div className="spacer" />
            <Chip tone="laser">{downloads.length}</Chip>
          </div>
          <div className="rows">
            {downloads.slice(0, 10).map((job) => (
              <div className="row" key={job.id}>
                <Chip tone={job.state === "done" ? "good" : job.state === "failed" ? "bad" : "laser"}>
                  {job.state}
                </Chip>
                <div className="grow">
                  <div className="title">{job.label}</div>
                  <div className="sub">
                    {job.state === "running"
                      ? progressText(job) || job.message || "queued"
                      : job.message || "queued"}
                  </div>
                  {job.state === "running" ? (
                    <div className="bar live" style={{ marginTop: 6 }}>
                      <div
                        className="fill"
                        style={{
                          width: `${
                            job.total_bytes ? Math.round((job.done_bytes / job.total_bytes) * 100) : 10
                          }%`,
                        }}
                      />
                    </div>
                  ) : null}
                </div>
                {job.state === "running" ? (
                  <Btn
                    small
                    onClick={async () => {
                      await api.jobCancel(job.id);
                      toast(`cancelled ${job.label}`);
                    }}
                  >
                    Cancel
                  </Btn>
                ) : null}
              </div>
            ))}
          </div>
        </section>
      ) : null}

      {files.length === 0 ? (
        <Empty
          title="No local downloads"
          hint="Games pulled from the Store land here. Change the folder above or in Settings."
        />
      ) : (
        <div className="rows">
          {files.map((file) => (
            <div className="row" key={file.path}>
              <Chip tone={file.is_dir ? "neutral" : "laser"}>
                {file.is_dir ? "dir" : (file.kind === "other" ? "file" : file.kind)}
              </Chip>
              <div className="grow">
                <div className="title" title={file.name}>
                  {file.name}
                </div>
              </div>
              <span className="size">{file.is_dir ? "" : fmtBytes(file.size_bytes)}</span>
              <Btn small onClick={() => void api.downloadOpen(file.name)}>
                Open
              </Btn>
              <Btn
                small
                variant="primary"
                disabled={!connected}
                onClick={async () => {
                  await api.downloadPush(file.name);
                  toast(`pushing ${file.name} to PSP`);
                }}
              >
                To PSP
              </Btn>
              <Btn
                small
                variant="danger"
                onClick={async () => {
                  if (!window.confirm(`Delete ${file.name}?`)) return;
                  await api.downloadDelete(file.name);
                  toast(`deleted ${file.name}`);
                  void load();
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
