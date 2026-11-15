import { useEffect, useState } from "react";
import { api } from "../api";
import type { DiscImage, GameEntry, GameInfo, LocalGame } from "../types";
import type { PageProps } from "../App";
import { Btn, Chip, Empty, GameHover, Seg, fmtBytes } from "../ui";

type Tab = "psp" | "local";

export default function Library({ snap, toast, refreshKey }: PageProps) {
  const [tab, setTab] = useState<Tab>("psp");
  const [games, setGames] = useState<GameEntry[]>([]);
  const [local, setLocal] = useState<LocalGame[]>([]);
  const [images, setImages] = useState<DiscImage[]>([]);
  const [covers, setCovers] = useState<Record<string, string>>({});
  const [selected, setSelected] = useState<{ serial: string; title: string; info: GameInfo | null } | null>(null);
  const connected = !!snap.volumes.find((v) => v.is_psp);

  useEffect(() => {
    void api
      .libraryScan()
      .then(async (lib) => {
        setGames(lib.games);
        setImages(lib.images);
        const entries = await Promise.all(
          lib.games.map(async (g) => [g.id, await api.cover(g.id, g.title_id ?? undefined, g.title)] as const)
        );
        const map: Record<string, string> = {};
        for (const [id, url] of entries) if (url) map[id] = url;
        setCovers(map);
      })
      .catch(() => setGames([]));
    void api.localLibrary().then(setLocal).catch(() => setLocal([]));
  }, [refreshKey, connected]);

  async function openDetail(serial: string, title: string) {
    const info = await api.gameInfo(serial, title).catch(() => null);
    setSelected({ serial, title, info });
  }

  return (
    <div className="page">
      <div style={{ display: "flex", gap: 10, alignItems: "center", flexWrap: "wrap" }}>
        <Seg
          ariaLabel="Library side"
          value={tab}
          onChange={setTab}
          options={[
            { value: "psp", label: `PSP Library (${games.length})` },
            { value: "local", label: `Local Library (${local.length})` },
          ]}
        />
        <div style={{ flex: 1 }} />
        <Chip tone={connected ? "good" : "warn"}>{connected ? "device ready" : "local only"}</Chip>
      </div>

      {tab === "psp" ? (
        !connected ? (
          <Empty title="No PSP connected" hint="Connect the console in USB mode to browse its library." />
        ) : games.length === 0 ? (
          <Empty title="No games on the stick" hint="Pull something from the Store or copy from the Local tab." />
        ) : (
          <div className="grid">
            {games.map((game) => (
              <div className="tile" key={game.id}>
                <div className="cover" onClick={() => void openDetail(game.title_id ?? "", game.title)}>
                  {covers[game.id] ? (
                    <img src={covers[game.id]} alt={`${game.title} cover`} />
                  ) : (
                    <div className="cover-fallback">
                      <span className="cf-title">{game.title}</span>
                      <span className="cf-serial">{game.title_id ?? game.kind}</span>
                    </div>
                  )}
                  <GameHover id={game.id} />
                </div>
                <div className="body">
                  <div className="t" title={game.title}>
                    {game.title}
                  </div>
                  <div className="s">
                    <span>{game.title_id ?? game.kind}</span>
                    <span>{fmtBytes(game.size_bytes)}</span>
                  </div>
                </div>
                <div className="actions">
                  <Btn small onClick={() => void openDetail(game.title_id ?? "", game.title)}>
                    Info
                  </Btn>
                  <Btn
                    small
                    onClick={async () => {
                      await api.fmFromPsp(game.id.startsWith("iso/") ? game.id : game.id);
                      toast(`copying ${game.title} to local`);
                    }}
                  >
                    To Local
                  </Btn>
                </div>
              </div>
            ))}
          </div>
        )
      ) : local.length === 0 ? (
        <Empty
          title="Local library is empty"
          hint="Store downloads land here; copy PSP games over from the PSP tab."
        />
      ) : (
        <div className="rows">
          {images.length > 0 ? (
            <>
              <div className="section-caption">ISO / CSO on the stick</div>
              {images.map((image) => (
                <div className="row" key={image.id}>
                  <Chip tone={image.format === "iso" ? "laser" : "neutral"}>{image.format}</Chip>
                  <div className="grow">
                    <div className="title">{image.title ?? image.file}</div>
                    <div className="sub">
                      {image.file}
                      {image.title_id ? ` - ${image.title_id}` : ""}
                    </div>
                  </div>
                  <span className="size">{fmtBytes(image.size_bytes)}</span>
                  <Btn
                    small
                    onClick={async () => {
                      await api.fmFromPsp(`iso/${image.file}`);
                      toast(`copying ${image.file} to local`);
                    }}
                  >
                    To Local
                  </Btn>
                </div>
              ))}
            </>
          ) : null}
          {local.map((item) => (
            <div className="row" key={item.id}>
              <Chip tone={item.kind === "iso" || item.kind === "cso" ? "laser" : "neutral"}>
                {item.kind}
              </Chip>
              <div className="grow">
                <div className="title">{item.title ?? item.name}</div>
                <div className="sub">
                  {item.name}
                  {item.serial ? ` - ${item.serial}` : ""}
                </div>
              </div>
              <span className="size">{fmtBytes(item.size_bytes)}</span>
              <Btn
                small
                variant="primary"
                disabled={!connected}
                onClick={async () => {
                  const kind = item.kind === "iso" || item.kind === "cso" ? "iso" : "game";
                  await api.fmToPsp(item.path, kind);
                  toast(`copying ${item.name} to PSP`);
                }}
              >
                To PSP
              </Btn>
            </div>
          ))}
        </div>
      )}

      {selected ? (
        <section className="card panel">
          <div className="panel-head">
            <h2>{selected.title}</h2>
            {selected.serial ? <Chip tone="laser">{selected.serial}</Chip> : null}
            <div className="spacer" />
            <Btn small onClick={() => setSelected(null)}>
              Close
            </Btn>
          </div>
          {selected.info?.animation ? (
            <p style={{ margin: "0 0 8px", fontSize: 12.5, color: "var(--text-faint)" }}>
              Animation: <span className="mono">{selected.info.animation}</span>
            </p>
          ) : null}
          {selected.info?.known ? (
            <div className="kv">
              <span className="k">catalog</span>
              <span className="v">{selected.info.known.title}</span>
            </div>
          ) : (
            <p style={{ margin: 0, fontSize: 12.5, color: "var(--text-faint)" }}>
              Not in the local database yet. Pulling it from the Store adds it.
            </p>
          )}
          <div className="section-caption">Mods</div>
          {selected.info && selected.info.mods.length > 0 ? (
            <div className="rows">
              {selected.info.mods.map((mod) => (
                <div className="row" key={mod.url}>
                  <Chip tone="neutral">{mod.kind}</Chip>
                  <div className="grow">
                    <div className="title">{mod.name}</div>
                    <div className="sub">
                      {mod.author || "unknown"} {mod.notes}
                    </div>
                  </div>
                  <Btn
                    small
                    onClick={async () => {
                      await api.modsInstall(selected.serial, mod.url, mod.name);
                      toast(`mod queued: ${mod.name}`);
                    }}
                  >
                    Pull
                  </Btn>
                </div>
              ))}
            </div>
          ) : (
            <p style={{ margin: 0, fontSize: 12.5, color: "var(--text-faint)" }}>
              No mods for this serial. Set a mods base in Settings to enable the mods catalog.
            </p>
          )}
        </section>
      ) : null}
    </div>
  );
}
