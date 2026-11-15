// Typed bridge to the Tauri runtime. In ?mock=1 mode the mock.ts shim
// installs window.__TAURI_INTERNALS__ so this file works unchanged.

import type {
  CatalogEntry,
  Check,
  Config,
  DiscImage,
  GameEntry,
  InstalledEntry,
  Job,
  LibraryView,
  MediaFile,
  PluginEntry,
  PluginsView,
  SaveEntry,
  Snapshot,
  ThemeEntry,
  Volume,
} from "./types";

export function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const internals = (window as unknown as Record<string, any>).__TAURI_INTERNALS__;
  return internals.invoke(cmd, args);
}

type UnlistenFn = () => void;

export function listen<T>(event: string, handler: (payload: T) => void): Promise<UnlistenFn> {
  const api = (window as unknown as Record<string, any>).__TAURI_INTERNALS__;
  const wrapped = (e: { payload: T }) => handler(e.payload);
  return api.invoke("plugin:event|listen", {
    event,
    handler: api.transformCallback(wrapped),
    target: { kind: "Any" },
  }).then((id: number) => {
    return () => {
      void api.invoke("plugin:event|unlisten", { event, eventId: id });
    };
  });
}

// ---- commands ----

export const api = {
  snapshot: () => invoke<Snapshot>("snapshot"),
  rescan: () => invoke<Volume[]>("rescan"),
  setActiveDevice: (mount: string) => invoke<void>("set_active_device", { mount }),
  eject: (mount: string) => invoke<void>("eject_device", { mount }),

  libraryScan: () => invoke<LibraryView>("library_scan"),
  icon: (id: string) => invoke<string | null>("icon", { id }),
  gameDelete: (id: string) => invoke<void>("game_delete", { id }),

  homebrewCatalog: () => invoke<CatalogEntry[]>("homebrew_catalog"),
  homebrewRefresh: () => invoke<CatalogEntry[]>("homebrew_refresh"),
  homebrewInstalled: () => invoke<InstalledEntry[]>("homebrew_installed"),
  homebrewInstall: (id: string) => invoke<number>("homebrew_install", { id }),
  homebrewImport: (zip: string) => invoke<number>("homebrew_import", { zip }),

  musicList: () => invoke<MediaFile[]>("music_list"),
  videoList: () => invoke<MediaFile[]>("video_list"),
  mediaDelete: (kind: string, rel: string) => invoke<void>("media_delete", { kind, rel }),
  videoPreview: (rel: string) => invoke<string | null>("video_preview", { rel }),
  musicPush: (files: string[], convert: boolean) =>
    invoke<number[]>("music_push", { files, convert }),
  videoPush: (files: string[], convert: boolean, thm: boolean) =>
    invoke<number[]>("video_push", { files, convert, thm }),

  isoImport: (file: string, toCso: boolean) => invoke<number>("iso_import", { file, toCso }),
  isoConvert: (imageId: string) => invoke<number>("iso_convert", { imageId }),
  imageDelete: (imageId: string) => invoke<void>("image_delete", { imageId }),

  savedataList: () => invoke<SaveEntry[]>("savedata_list"),
  savedataIcon: (folder: string) => invoke<string | null>("savedata_icon", { folder }),
  savedataBackup: (folders: string[]) => invoke<number>("savedata_backup", { folders }),
  savedataRestore: (zip: string) => invoke<number>("savedata_restore", { zip }),
  savedataDelete: (folder: string) => invoke<void>("savedata_delete", { folder }),
  savedataReveal: (folder: string) => invoke<string>("savedata_reveal", { folder }),

  pluginsList: () => invoke<PluginsView>("plugins_list"),
  pluginToggle: (section: string, path: string, enable: boolean) =>
    invoke<void>("plugin_toggle", { section, path, enable }),
  pluginAdd: (section: string, prx: string) => invoke<PluginEntry>("plugin_add", { section, prx }),
  pluginRemove: (section: string, path: string) => invoke<void>("plugin_remove", { section, path }),

  themesList: () => invoke<ThemeEntry[]>("themes_list"),
  themeInstall: (file: string) => invoke<ThemeEntry>("theme_install", { file }),
  themeDelete: (name: string) => invoke<void>("theme_delete", { name }),

  doctorRun: () => invoke<Check[]>("doctor_run"),
  configGet: () => invoke<Config>("config_get"),
  configSet: (patch: Partial<Config>) => invoke<Config>("config_set", { patch }),

  jobs: () => invoke<Job[]>("jobs"),
  jobCancel: (id: number) => invoke<boolean>("job_cancel", { id }),
  jobsClear: () => invoke<void>("jobs_clear"),

  reveal: (path: string) => invoke<void>("reveal", { path }),
  openDataDir: () => invoke<string>("open_data_dir"),

  // store / debrid / downloads / files / catalog
  debridStatus: () => invoke<import("./types").DebridStatus>("debrid_status"),
  storeSearch: (query: string, source: "gamesearch" | "ia" | "minerva") =>
    invoke<import("./types").StoreResult>("store_search", { query, source, limit: 40 }),
  storeDetail: (source: string, id: string) =>
    invoke<import("./types").StoreResult>("store_detail", { source, id }),
  storeDownload: (source: string, id: string) => invoke<number>("store_download", { source, id }),
  downloadsList: () => invoke<{ dir: string; files: import("./types").FmEntry[] }>("downloads_list"),
  downloadDirSet: (path: string) => invoke<void>("download_dir_set", { path }),
  downloadOpen: (name: string) => invoke<string>("download_open", { name }),
  downloadDelete: (name: string) => invoke<void>("download_delete", { name }),
  downloadPush: (name: string) => invoke<number>("download_push", { name }),
  fmList: (path: string) => invoke<import("./types").FmEntry[]>("fm_list", { path }),
  fmMkdir: (path: string, name: string) => invoke<string>("fm_mkdir", { path, name }),
  fmDelete: (path: string) => invoke<void>("fm_delete", { path }),
  fmToPsp: (path: string, destKind: string) => invoke<number>("fm_to_psp", { path, destKind }),
  fmFromPsp: (pspId: string) => invoke<number>("fm_from_psp", { pspId }),
  localLibrary: () => invoke<import("./types").LocalGame[]>("local_library"),
  gameInfo: (serial: string, title: string) =>
    invoke<import("./types").GameInfo>("game_info", { serial, title }),
  cover: (id: string, serial?: string, title?: string) =>
    invoke<string | null>("cover", { id, serial: serial ?? null, title: title ?? null }),
  animation: (id: string) => invoke<string | null>("animation", { id }),
  modsList: (serial: string) => invoke<import("./types").ModEntry[]>("mods_list", { serial }),
  modsInstall: (serial: string, url: string, name: string) =>
    invoke<number>("mods_install", { serial, url, name }),
};

// ---- native file pickers (dialog plugin) ----

interface DialogInternals {
  invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
}

export const pickers = {
  async pickFiles(title: string, extensions: string[]): Promise<string[]> {
    const api = (window as unknown as Record<string, any>).__TAURI_INTERNALS__ as DialogInternals;
    const result = (await api.invoke("plugin:dialog|open", {
      options: {
        title,
        multiple: true,
        filters: [{ name: "Files", extensions }],
      },
    })) as string | string[] | null;
    if (!result) return [];
    return Array.isArray(result) ? result : [result];
  },
  async pickFile(title: string, extensions: string[]): Promise<string | null> {
    const files = await pickers.pickFiles(title, extensions);
    return files.length ? files[0] : null;
  },
  async pickDirectory(title: string): Promise<string | null> {
    const api = (window as unknown as Record<string, any>).__TAURI_INTERNALS__ as DialogInternals;
    const result = (await api.invoke("plugin:dialog|open", {
      options: { title, directory: true, multiple: false },
    })) as string | null;
    return result ?? null;
  },
  async pickSavePath(title: string, defaultName: string, extensions: string[]): Promise<string | null> {
    const api = (window as unknown as Record<string, any>).__TAURI_INTERNALS__ as DialogInternals;
    const result = (await api.invoke("plugin:dialog|save", {
      options: {
        title,
        defaultPath: defaultName,
        filters: [{ name: "Archive", extensions }],
      },
    })) as string | null;
    return result ?? null;
  },
};

// Re-exports for pages that need them.
export type { GameEntry, DiscImage, CatalogEntry, Job, Volume };
