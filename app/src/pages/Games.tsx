import { useEffect, useMemo, useState } from "react";
import { api } from "../api";
import type { GameEntry, LibraryView } from "../types";
import type { PageProps } from "../App";
import { Btn, Chip, Empty, GameHover, Seg, confirmDialog, fmtBytes } from "../ui";

type Filter = "all" | "digital" | "homebrew" | "loader";

export default function Games({ snap, reload, toast, refreshKey }: PageProps) {
  const [lib, setLib] = useState<LibraryView | null>(null);
  const [filter, setFilter] = useState<Filter>("all");
  const [query, setQuery] = useState("");
  const [icons, setIcons] = useState<Record<string, string>>({});

  const connected = !!snap.volumes.find((v) => v.is_psp);

  useEffect(() => {
    if (!connected) {
      setLib(null);
      return;
    }
    void api
      .libraryScan()
      .then(setLib)
      .catch(() => setLib(null));
  }, [connected, refreshKey]);

  useEffect(() => {
    if (!lib) return;
    let cancelled = false;
    void Promise.all(
      lib.games
        .filter((g) => g.has_icon)
        .map(async (g) => [g.id, await api.icon(g.id)] as const)
    ).then((pairs) => {
      if (cancelled) return;
      const map: Record<string, string> = {};
      for (const [id, url] of pairs) {
        if (url) map[id] = url;
      }
      setIcons(map);
    });
    return () => {
      cancelled = true;
    };
  }, [lib]);

  const games = useMemo(() => {
    let list = lib?.games ?? [];
    if (filter !== "all") list = list.filter((g) => g.kind === filter);
    if (query.trim()) {
      const q = query.trim().toLowerCase();
      list = list.filter(
        (g) =>
          g.title.toLowerCase().includes(q) ||
          g.folder.toLowerCase().includes(q) ||
          (g.title_id ?? "").toLowerCase().includes(q)
      );
    }
    return list;
  }, [lib, filter, query]);

  if (!connected) {
    return (
      <div className="page">
        <Empty
          title="No PSP connected"
          hint="Plug the PSP in via USB and select USB Connection on the console to browse its library."
        />
      </div>
    );
  }

  return (
    <div className="page">
      <div style={{ display: "flex", gap: 10, alignItems: "center", flexWrap: "wrap" }}>
        <Seg
          ariaLabel="Library filter"
          value={filter}
          onChange={setFilter}
          options={[
            { value: "all", label: "All" },
            { value: "digital", label: "Digital" },
            { value: "homebrew", label: "Homebrew" },
            { value: "loader", label: "Loaders" },
          ]}
        />
        <input
          type="text"
          className="mono"
          placeholder="search title / folder / serial"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          style={{
            flex: 1,
            minWidth: 180,
            background: "rgba(255,255,255,0.03)",
            border: "1px solid var(--line-strong)",
            color: "var(--text)",
            borderRadius: "var(--radius-sm)",
            padding: "8px 10px",
            fontSize: 12.5,
          }}
          aria-label="Search games"
        />
        <Chip tone="laser">{games.length} shown</Chip>
      </div>

      {games.length === 0 ? (
        <Empty
          title="Nothing here yet"
          hint="Install homebrew from the Homebrew page, or push disc images on the Images page."
        />
      ) : (
        <div className="grid">
          {games.map((game) => (
            <GameTile
              key={game.id}
              game={game}
              iconUrl={icons[game.id]}
              onOpen={async () => {
                await api.reveal(game.path);
              }}
              onDelete={async () => {
                if (confirmDialog("Delete from stick?", `${game.title} (${game.folder}) will be removed from the memory stick.`)) {
                  await api.gameDelete(game.id);
                  toast(`deleted ${game.folder}`);
                  reload();
                }
              }}
            />
          ))}
        </div>
      )}

      <div className="section-caption">ISO / disc images</div>
      {lib && lib.images.length > 0 ? (
        <div className="rows">
          {lib.images.map((image) => (
            <div className="row" key={image.id}>
              <Chip tone={image.format === "iso" ? "laser" : "neutral"}>{image.format}</Chip>
              <div className="grow">
                <div className="title" title={image.path}>
                  {image.title ?? image.file}
                </div>
                <div className="sub">
                  {image.file}
                  {image.title_id ? ` - ${image.title_id}` : ""}
                </div>
              </div>
              <span className="size">{fmtBytes(image.size_bytes)}</span>
              {image.format === "iso" || image.format === "cso" ? (
                <Btn
                  small
                  onClick={async () => {
                    toast(`converting ${image.file}`);
                    await api.isoConvert(image.id);
                    reload();
                  }}
                  title="Convert between ISO and CSO"
                >
                  Convert
                </Btn>
              ) : null}
              <Btn
                small
                onClick={async () => {
                  await api.reveal(image.path);
                }}
              >
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
      ) : (
        <Empty title="No disc images" hint="ISO/CSO files go in the ISO folder; import them from the Images page." />
      )}
    </div>
  );
}

function GameTile({
  game,
  iconUrl,
  onOpen,
  onDelete,
}: {
  game: GameEntry;
  iconUrl?: string;
  onOpen: () => void;
  onDelete: () => void;
}) {
  const kindLabel = game.kind === "digital" ? "digital" : game.kind === "loader" ? "loader" : "homebrew";
  return (
    <div className="tile">
      <div className="cover">
        {iconUrl ? (
          <img src={iconUrl} alt={`${game.title} icon`} />
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
          <span>{game.title_id ?? kindLabel}</span>
          <span>{fmtBytes(game.size_bytes)}</span>
        </div>
      </div>
      <div className="actions">
        <Btn small onClick={onOpen}>
          Open
        </Btn>
        <Btn small variant="danger" onClick={onDelete}>
          Delete
        </Btn>
      </div>
    </div>
  );
}
