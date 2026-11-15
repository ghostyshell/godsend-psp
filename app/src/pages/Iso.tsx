import { useEffect, useState } from "react";
import { api, pickers } from "../api";
import type { DiscImage } from "../types";
import type { PageProps } from "../App";
import { Btn, Chip, Empty, Toggle, confirmDialog, fmtBytes } from "../ui";

export default function Iso({ snap, reload, toast, refreshKey }: PageProps) {
  const [images, setImages] = useState<DiscImage[]>([]);
  const [toCso, setToCso] = useState(false);

  const connected = !!snap.volumes.find((v) => v.is_psp);

  useEffect(() => {
    if (!connected) {
      setImages([]);
      return;
    }
    void api
      .libraryScan()
      .then((lib) => setImages(lib.images))
      .catch(() => setImages([]));
  }, [connected, refreshKey]);

  return (
    <div className="page">
      <div className="card panel">
        <div className="panel-head">
          <h2>Disc images</h2>
          <div className="spacer" />
          <Chip tone={connected ? "good" : "bad"}>{connected ? "device ready" : "no device"}</Chip>
        </div>
        <div style={{ display: "flex", gap: 14, alignItems: "center", flexWrap: "wrap" }}>
          <Toggle
            checked={toCso}
            onChange={setToCso}
            label="Compress to CSO on import (saves space, slower)"
          />
          <div style={{ flex: 1 }} />
          <Btn
            variant="primary"
            disabled={!connected}
            onClick={async () => {
              const file = await pickers.pickFile("Pick a disc image", ["iso", "cso", "pbp"]);
              if (!file) return;
              await api.isoImport(file, toCso && file.toLowerCase().endsWith(".iso"));
              toast(`importing ${file.split("/").pop()}`);
            }}
          >
            Import Image
          </Btn>
        </div>
        <p style={{ margin: "10px 0 0", fontSize: 12.5, color: "var(--text-faint)" }}>
          Retail UMD images are read for title and serial directly from the ISO.
          Images live in the ISO folder at the stick root and need a CFW ISO
          driver (ARK includes Inferno) to show up on the console.
        </p>
      </div>

      {images.length === 0 ? (
        <Empty title="No disc images" hint="Import an ISO or CSO to get started." />
      ) : (
        <div className="rows">
          {images.map((image) => (
            <div className="row" key={image.id}>
              <Chip tone={image.format === "iso" ? "laser" : "neutral"}>{image.format}</Chip>
              <div className="grow">
                <div className="title" title={image.title ?? image.file}>
                  {image.title ?? image.file.replace(/\.(iso|cso|pbp)$/i, "")}
                </div>
                <div className="sub">
                  {image.file}
                  {image.title_id ? ` - ${image.title_id}` : ""}
                  {image.is_umd ? " - UMD image" : ""}
                </div>
              </div>
              <span className="size">{fmtBytes(image.size_bytes)}</span>
              {image.format === "iso" || image.format === "cso" ? (
                <Btn
                  small
                  onClick={async () => {
                    await api.isoConvert(image.id);
                    toast(`converting ${image.file}`);
                  }}
                  title={image.format === "iso" ? "Compress to CSO" : "Decompress to ISO"}
                >
                  {image.format === "iso" ? "To CSO" : "To ISO"}
                </Btn>
              ) : null}
              <Btn small onClick={() => void api.reveal(image.path)}>
                Open
              </Btn>
              <Btn
                small
                variant="danger"
                onClick={async () => {
                  if (confirmDialog("Delete disc image?", `${image.file} will be removed from the stick.`)) {
                    await api.imageDelete(image.id);
                    toast(`deleted ${image.file}`);
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
