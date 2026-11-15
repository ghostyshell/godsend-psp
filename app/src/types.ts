// Types shared with the Rust core (keep in sync with core/src serializers).

export interface Volume {
  mount: string;
  label: string;
  is_psp: boolean;
  free_bytes: number;
  total_bytes: number;
}

export interface Config {
  music_format: string;
  video_preset: string;
  video_thumbnails: boolean;
  convert_music: boolean;
  convert_video: boolean;
  video_preset: string;
  video_crf: number;
  video_fps: number;
  video_audio_kbps: number;
  video_resolution: string;
  ffmpeg_path: string;
  ffprobe_path: string;
  cso_level: number;
  download_dir: string;
  rd_api_key: string;
  tb_api_key: string;
  gamesearch_base: string;
  covers_base: string;
  mods_base: string;
  mcp_enabled: boolean;
  mcp_port: number;
}

export interface Check {
  name: string;
  ok: boolean;
  detail: string;
}

export interface Snapshot {
  volumes: Volume[];
  active: string | null;
  config: Config;
  doctor: Check[];
  jobs: Job[];
}

export type GameKind = "digital" | "homebrew" | "loader";

export interface GameEntry {
  id: string;
  folder: string;
  title: string;
  title_id: string | null;
  category: string | null;
  kind: GameKind;
  path: string;
  size_bytes: number;
  has_icon: boolean;
  psp_system_ver: string | null;
  version: string | null;
}

export interface DiscImage {
  id: string;
  file: string;
  format: "iso" | "cso" | "pbp";
  title: string | null;
  title_id: string | null;
  is_umd: boolean;
  path: string;
  size_bytes: number;
}

export interface LibraryView {
  games: GameEntry[];
  images: DiscImage[];
}

export type JobState = "running" | "done" | "failed" | "cancelled";

export interface Job {
  id: number;
  kind: string;
  label: string;
  state: JobState;
  done_bytes: number;
  total_bytes: number;
  message: string;
  started_at: number;
  finished_at: number | null;
}

export interface CatalogEntry {
  id: string;
  name: string;
  description: string;
  category: string;
  repo: string;
  layout: "auto" | "ark" | "psp_root" | "eboot";
  note: string | null;
  latest_tag: string | null;
  zip_url: string | null;
  published_at: string | null;
}

export interface InstalledEntry {
  id: string;
  installed: boolean;
}

export interface MediaFile {
  rel: string;
  name: string;
  size_bytes: number;
}

export interface SaveEntry {
  folder: string;
  title: string | null;
  subtitle: string | null;
  title_id: string | null;
  size_bytes: number;
  has_icon: boolean;
}

export type PluginSection = "game" | "vsh" | "pops";

export interface PluginEntry {
  path: string;
  enabled: boolean;
  exists: boolean;
  section: PluginSection;
}

export interface PluginsView {
  game: PluginEntry[];
  vsh: PluginEntry[];
  pops: PluginEntry[];
}

export interface ThemeEntry {
  name: string;
  format: string;
  size_bytes: number;
}

export const PAGE_TITLES: Record<string, string> = {
  home: "Console",
  library: "Library",
  gamestore: "Store",
  downloads: "Downloads",
  files: "File Manager",
  store: "Homebrew",
  music: "Music",
  video: "Video",
  iso: "Disc Images",
  saves: "Save Data",
  mods: "Mods",
  settings: "Settings",
};

// ---- store / debrid / downloads / files ----

export interface DebridStatus {
  realdebrid: boolean;
  torbox: boolean;
}

export interface GameLink {
  host: string;
  url: string;
  linkType: string;
  supportedBy: string[];
}

export interface GameHit {
  id: string;
  slug: string;
  title: string;
  platform: string;
  source: string;
  host: string;
  url: string;
  linkType: string;
  supportedBy: string[];
  cover: string | null;
  size: number | null;
  links: GameLink[];
}

export interface IaItem {
  id: string;
  title: string;
  files_count: number;
  size_bytes: number;
}

export interface IaFile {
  name: string;
  size_bytes: number;
  url: string;
}

export interface StoreResult {
  kind: "gamesearch" | "ia";
  items?: GameHit[];
  files?: IaFile[];
}

export interface FmEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size_bytes: number;
  kind: string;
}

export interface LocalGame {
  id: string;
  name: string;
  kind: string;
  path: string;
  size_bytes: number;
  serial: string | null;
  title: string | null;
}

export interface ModEntry {
  name: string;
  author: string;
  version: string;
  url: string;
  size_bytes: number;
  notes: string;
  kind: string;
}

export interface GameInfo {
  known: { serial: string; title: string; cover: string; animation: string } | null;
  animation: string | null;
  mods: ModEntry[];
}
