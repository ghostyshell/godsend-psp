import { useEffect, useState } from "react";
import { api } from "../api";
import type { GameHit, GameInfo, IaItem, IaFile, NpsEntry, RromsItem } from "../types";
import type { PageProps } from "../App";
import { Btn, Chip, Empty, Seg, fmtBytes } from "../ui";

type Source = "gamesearch" | "ia" | "minerva" | "rroms" | "nps";
type View =
  | { kind: "none" }
  | { kind: "game"; hit: GameHit; info: GameInfo | null }
  | { kind: "ia"; item: IaItem; files: IaFile[] };

export default function Store({ snap, reload, toast, refreshKey }: PageProps) {
  const [source, setSource] = useState<Source>("gamesearch");
  const [query, setQuery] = useState("");
  const [items, setItems] = useState<GameHit[]>([]);
  const [iaItems, setIaItems] = useState<IaItem[]>([]);
  const [minervaItems, setMinervaItems] = useState<{ id: string; title: string; url: string }[]>([]);
  const [rromsItems, setRromsItems] = useState<RromsItem[]>([]);
  const [npsItems, setNpsItems] = useState<NpsEntry[]>([]);
  const [view, setView] = useState<View>({ kind: "none" });
  const [busy, setBusy] = useState(false);
  const [debrid, setDebrid] = useState({ realdebrid: false, torbox: false });

  const connected = !!snap.volumes.find((v) => v.is_psp);

  useEffect(() => {
    void api.debridStatus().then(setDebrid);
  }, [refreshKey]);

  async function search(q: string, src: Source) {
    setBusy(true);
    try {
      const result = await api.storeSearch(q, src);
      if (src === "ia") {
        setIaItems((result.items as unknown as IaItem[]) ?? []);
        setItems([]);
      } else if (src === "minerva") {
        setMinervaItems((result.items as unknown as { id: string; title: string; url: string }[]) ?? []);
        setItems([]);
      } else if (src === "rroms") {
        const all = (result.items as unknown as RromsItem[]) ?? [];
        const needle = q.trim().toLowerCase();
        setRromsItems(needle ? all.filter((item) => item.title.toLowerCase().includes(needle)) : all);
        setItems([]);
      } else if (src === "nps") {
        setNpsItems((result.items as unknown as NpsEntry[]) ?? []);
        setItems([]);
      } else {
        setItems((result.items as unknown as GameHit[]) ?? []);
        setIaItems([]);
      }
    } catch (err) {
      toast(`search failed: ${String(err)}`);
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => {
    void search("", "gamesearch");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [refreshKey]);

  async function openGame(hit: GameHit) {
    setView({ kind: "game", hit, info: null });
    try {
      const detail = (await api.storeDetail("gamesearch", hit.slug)) as { hit?: GameHit };
      const full = detail.hit ?? hit;
      const info = await api.gameInfo(serialFor(full), full.title);
      setView({ kind: "game", hit: full, info });
    } catch {
      setView({ kind: "game", hit, info: null });
    }
  }

  async function openIa(item: IaItem) {
    setView({ kind: "ia", item, files: [] });
    try {
      const detail = (await api.storeDetail("ia", item.id)) as { files?: IaFile[] };
      setView({ kind: "ia", item, files: detail.files ?? [] });
    } catch {
      setView({ kind: "ia", item, files: [] });
    }
  }

  async function download(source: Source, id: string, label: string) {
    if (!debrid.realdebrid && !debrid.torbox && source === "gamesearch") {
      toast("no debrid account: DDL links work, torrents need Settings > Debrid");
    }
    try {
      await api.storeDownload(source, id);
      toast(`download queued: ${label}`);
      reload();
    } catch (err) {
      toast(`download failed: ${String(err)}`);
    }
  }

  return (
    <div className="page">
      <div className="card panel">
        <div className="panel-head">
          <h2>Store</h2>
          <div className="spacer" />
          <Chip tone={debrid.realdebrid || debrid.torbox ? "good" : "warn"}>
            {debrid.realdebrid || debrid.torbox
              ? `${debrid.realdebrid ? "RD" : ""}${debrid.realdebrid && debrid.torbox ? " + " : ""}${debrid.torbox ? "TorBox" : ""}`
              : "no debrid"}
          </Chip>
        </div>
        <div style={{ display: "flex", gap: 10, flexWrap: "wrap", alignItems: "center" }}>
          <Seg
            ariaLabel="Store source"
            value={source}
            onChange={(next) => {
              setSource(next);
              setView({ kind: "none" });
              void search(query, next);
            }}
            options={[
              { value: "gamesearch", label: "Game Debrid" },
              { value: "ia", label: "Internet Archive" },
              { value: "minerva", label: "Minerva" },
              { value: "rroms", label: "r-roms" },
              { value: "nps", label: "NoPayStation" },
            ]}
          />
          <input
            type="text"
            className="mono"
            placeholder={
              source === "ia"
                ? "PSP ISO (megathread set)"
                : source === "minerva"
                  ? "Minerva PSP sets load automatically"
                  : source === "rroms"
                    ? "filter the r-roms links"
                    : source === "nps"
                      ? "filter NoPayStation by title"
                      : "search PSP games"
            }
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") void search(query, source);
            }}
            style={{
              flex: 1,
              minWidth: 200,
              background: "rgba(255,255,255,0.03)",
              border: "1px solid var(--line-strong)",
              color: "var(--text)",
              borderRadius: "var(--radius-sm)",
              padding: "8px 10px",
              fontSize: 12.5,
            }}
            aria-label="Store search"
          />
          <Btn onClick={() => void search(query, source)} disabled={busy}>
            {busy ? "Searching..." : "Search"}
          </Btn>
        </div>
        <p style={{ margin: "8px 0 0", fontSize: 12.5, color: "var(--text-faint)" }}>
          {source === "gamesearch"
            ? "Debrid index wired to the game-search service. Links are resolved through Real-Debrid or TorBox when connected, then downloaded with aria2c."
            : source === "minerva"
              ? "Minerva Archive Myrient PSP torrents. Downloading saves the torrent, which aria2c can fetch."
              : source === "rroms"
                ? "The r-roms PSP page: Internet Archive sets and direct archives. An archive.org item downloads every game file in that set."
                : source === "nps"
                  ? "NoPayStation PSP packages. Installing a .pkg needs pkg2zip or NoPayStation to decrypt it; the .zrif sidecar is saved next to the download."
                  : "Internet Archive sets in the r/Roms megathread style. Direct downloads, no account needed."}
        </p>
      </div>

      {view.kind === "game" ? (
        <section className="card panel">
          <div className="panel-head">
            <h2>{view.hit.title}</h2>
            <Chip tone="laser">{view.hit.platform}</Chip>
            <div className="spacer" />
            <Btn small onClick={() => setView({ kind: "none" })}>
              Close
            </Btn>
          </div>
          <div className="kv">
            <span className="k">source</span>
            <span className="v">{view.hit.source || "gamesearch"}</span>
          </div>
          <div className="kv">
            <span className="k">slug</span>
            <span className="v">{view.hit.slug}</span>
          </div>
          {view.hit.size ? (
            <div className="kv">
              <span className="k">size</span>
              <span className="v">{fmtBytes(view.hit.size)}</span>
            </div>
          ) : null}
          <div className="section-caption">Links</div>
          <div className="rows">
            {(view.hit.links.length ? view.hit.links : []).map((link, i) => (
              <div className="row" key={`${link.host}-${i}`}>
                <Chip tone={link.linkType === "ddl" ? "neutral" : "laser"}>{link.linkType}</Chip>
                <div className="grow">
                  <div className="title mono" style={{ fontSize: 12 }}>
                    {link.host}
                  </div>
                  <div className="sub">{link.supportedBy.join(", ") || "no debrid support"}</div>
                </div>
                <Btn
                  small
                  variant="primary"
                  onClick={() => void download("gamesearch", view.hit.slug, view.hit.title)}
                >
                  Download
                </Btn>
              </div>
            ))}
            {view.hit.links.length === 0 ? (
              <p style={{ margin: 0, fontSize: 12.5, color: "var(--text-faint)" }}>
                No links returned for this title yet.
              </p>
            ) : null}
          </div>
          {view.info && view.info.mods.length > 0 ? (
            <>
              <div className="section-caption">Mods</div>
              <div className="rows">
                {view.info.mods.map((mod) => (
                  <div className="row" key={mod.url}>
                    <Chip tone="neutral">{mod.kind}</Chip>
                    <div className="grow">
                      <div className="title">{mod.name}</div>
                      <div className="sub">
                        {mod.author || "unknown"} {mod.version ? `v${mod.version}` : ""} {mod.notes}
                      </div>
                    </div>
                    <Btn
                      small
                      onClick={async () => {
                        await api.modsInstall(serialFor(view.hit), mod.url, mod.name);
                        toast(`mod queued: ${mod.name}`);
                      }}
                    >
                      Pull
                    </Btn>
                  </div>
                ))}
              </div>
            </>
          ) : null}
        </section>
      ) : null}

      {view.kind === "ia" ? (
        <section className="card panel">
          <div className="panel-head">
            <h2>{view.item.title}</h2>
            <Chip tone="neutral">{view.item.files_count} files</Chip>
            <div className="spacer" />
            <Btn small onClick={() => setView({ kind: "none" })}>
              Close
            </Btn>
          </div>
          <div className="rows">
            {view.files.slice(0, 60).map((file) => (
              <div className="row" key={file.name}>
                <Chip tone="neutral">
                  {file.name.split(".").pop()?.toUpperCase() ?? "FILE"}
                </Chip>
                <div className="grow">
                  <div className="title" title={file.name}>
                    {file.name}
                  </div>
                </div>
                <span className="size">{fmtBytes(file.size_bytes)}</span>
                <Btn
                  small
                  variant="primary"
                  onClick={() => void download("ia", `${view.item.id}/${file.name}`, file.name)}
                >
                  Download
                </Btn>
              </div>
            ))}
          </div>
        </section>
      ) : null}

      {source === "gamesearch" ? (
        items.length === 0 ? (
          <Empty
            title="No results"
            hint="Check that the game-search service is reachable (Settings > Sources) or search another title."
          />
        ) : (
          <div className="store-grid">
            {items.map((hit) => (
              <article className="store-card" key={hit.slug + hit.id}>
                <div className="head">
                  <span className="name">{hit.title}</span>
                  <span className="tag">{hit.platform}</span>
                </div>
                <div className="desc">{hit.source || hit.host || "debrid index"}</div>
                <div className="repo">{hit.slug}</div>
                <div className="foot">
                  <Chip tone={hit.supportedBy.length ? "good" : "warn"}>
                    {hit.supportedBy.length ? hit.supportedBy.join(" ") : "direct"}
                  </Chip>
                  <div className="spacer" />
                  <Btn small onClick={() => void openGame(hit)}>
                    Links
                  </Btn>
                  <Btn
                    small
                    variant="primary"
                    onClick={() => void download("gamesearch", hit.slug, hit.title)}
                  >
                    Download
                  </Btn>
                </div>
              </article>
            ))}
          </div>
        )
      ) : source === "minerva" ? (
        minervaItems.length === 0 ? (
          <Empty title="No Minerva sets" hint="Minerva PSP torrents could not be probed; check the connection." />
        ) : (
          <div className="rows">
            {minervaItems.map((set) => (
              <div className="row" key={set.id}>
                <Chip tone="laser">minerva</Chip>
                <div className="grow">
                  <div className="title">{set.title}</div>
                  <div className="sub">{set.id}</div>
                </div>
                <Btn
                  small
                  variant="primary"
                  onClick={() => void download("minerva", set.id, set.title)}
                >
                  Download
                </Btn>
              </div>
            ))}
          </div>
        )
      ) : source === "rroms" ? (
        rromsItems.length === 0 ? (
          <Empty
            title="No r-roms links"
            hint="No links matched. Clear the filter to load the page, or check the connection."
          />
        ) : (
          <div className="rows">
            {rromsItems.map((item) => (
              <div className="row" key={item.id}>
                <Chip tone={item.kind === "ia_item" ? "laser" : "neutral"}>
                  {item.kind === "ia_item" ? "IA" : item.kind}
                </Chip>
                <div className="grow">
                  <div className="title">{item.title}</div>
                  <div className="sub mono" style={{ fontSize: 11.5 }}>
                    {item.id}
                  </div>
                </div>
                <Btn
                  small
                  variant="primary"
                  onClick={() => void download("rroms", item.id, item.title)}
                >
                  Download
                </Btn>
              </div>
            ))}
          </div>
        )
      ) : source === "nps" ? (
        npsItems.length === 0 ? (
          <Empty
            title="No packages"
            hint="Search NoPayStation by title or title id, e.g. LocoRoco."
          />
        ) : (
          <div className="rows">
            {npsItems.map((entry) => (
              <div className="row" key={entry.title_id}>
                <Chip tone="laser">{entry.region || "NPS"}</Chip>
                <div className="grow">
                  <div className="title">{entry.title}</div>
                  <div className="sub mono" style={{ fontSize: 11.5 }}>
                    {entry.title_id}
                    {entry.zrif ? " - key in index" : ""}
                  </div>
                </div>
                <span className="size">{fmtBytes(entry.size_bytes)}</span>
                <Btn
                  small
                  variant="primary"
                  onClick={() => void download("nps", entry.title_id, entry.title)}
                >
                  Download
                </Btn>
              </div>
            ))}
          </div>
        )
      ) : iaItems.length === 0 ? (
        <Empty title="No items" hint="Search an Internet Archive PSP set, e.g. PSP ISO collection." />
      ) : (
        <div className="rows">
          {iaItems.map((item) => (
            <div className="row" key={item.id}>
              <Chip tone="laser">IA</Chip>
              <div className="grow">
                <div className="title">{item.title}</div>
                <div className="sub">
                  {item.id} - {item.files_count} files
                </div>
              </div>
              <span className="size">{fmtBytes(item.size_bytes)}</span>
              <Btn small onClick={() => void openIa(item)}>
                Files
              </Btn>
            </div>
          ))}
        </div>
      )}

      {!connected ? (
        <p style={{ margin: 0, fontSize: 12.5, color: "var(--text-faint)" }}>
          Downloads go to the local download folder; connect a PSP to copy them over
          from Downloads or Files.
        </p>
      ) : null}
    </div>
  );
}

function serialFor(hit: GameHit): string {
  // Games with a real serial in the slug look like UCUS98737; otherwise the
  // serial is unknown until the game lands on the stick.
  const slug = hit.slug.toUpperCase();
  if (/^[A-Z]{4}-?\d{5}$/.test(slug)) {
    return slug.includes("-") ? slug : `${slug.slice(0, 4)}-${slug.slice(4)}`;
  }
  return "";
}
