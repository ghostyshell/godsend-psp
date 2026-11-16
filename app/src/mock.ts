// Mock bridge: installs window.__TAURI_INTERNALS__ with fixture data so the
// UI runs in a plain browser (?mock=1) with no PSP attached. Dropped from
// release builds because nothing imports it statically.

type Handler = (payload: unknown) => void;

const listeners = new Map<number, { event: string; handler: Handler }>();
let nextCallbackId = 1;

function emit(event: string, payload: unknown) {
  for (const { event: name, handler } of listeners.values()) {
    if (name === event) {
      handler({ event, id: nextCallbackId, payload });
    }
  }
}

// ---- fixtures ----

const GB = 1024 * 1024 * 1024;
const MB = 1024 * 1024;

const pspVolume = {
  mount: "/Volumes/MEMORY STICK",
  label: "MEMORY STICK",
  is_psp: true,
  free_bytes: 3.2 * GB,
  total_bytes: 16 * GB,
};

const laptopVolume = {
  mount: "/",
  label: "Macintosh HD",
  is_psp: false,
  free_bytes: 120 * GB,
  total_bytes: 994 * GB,
};

const volumes = [laptopVolume, pspVolume];

const config = {
  music_format: "mp3",
  video_preset: "psp_avc_480",
  video_thumbnails: true,
  convert_music: true,
  convert_video: true,
  ffmpeg_path: "",
  ffprobe_path: "",
  cso_level: 6,
  download_dir: "",
  rd_api_key: "",
  tb_api_key: "",
  gamesearch_base: "http://localhost:3011",
  ia_access_key: "",
  ia_secret_key: "",
  ia_email: "",
  ia_password: "",
  covers_base: "",
  mods_base: "",
  mcp_enabled: true,
  mcp_port: 8849,
};

const games = [
  { id: "game/GodOfWarChainsOfOlympus", folder: "GodOfWarChainsOfOlympus", title: "God of War: Chains of Olympus", title_id: "UCUS-98737", category: "MG", kind: "digital", path: "/Volumes/MEMORY STICK/PSP/GAME/GodOfWarChainsOfOlympus", size_bytes: 1.1 * GB, has_icon: true, psp_system_ver: "3.80", version: "1.02" },
  { id: "game/LocoRoco2", folder: "LocoRoco2", title: "LocoRoco 2", title_id: "UCUS-98716", category: "MG", kind: "digital", path: "/Volumes/MEMORY STICK/PSP/GAME/LocoRoco2", size_bytes: 640 * MB, has_icon: true, psp_system_ver: "3.80", version: "1.01" },
  { id: "game/WipeoutPure", folder: "WipeoutPure", title: "Wipeout Pure", title_id: "UCUS-98712", category: "MG", kind: "digital", path: "/Volumes/MEMORY STICK/PSP/GAME/WipeoutPure", size_bytes: 420 * MB, has_icon: true, psp_system_ver: "1.50", version: "1.00" },
  { id: "game/FrogGBA", folder: "FrogGBA", title: "FrogGBA", title_id: null, category: null, kind: "homebrew", path: "/Volumes/MEMORY STICK/PSP/GAME/FrogGBA", size_bytes: 4.2 * MB, has_icon: true, psp_system_ver: null, version: "0.4.0" },
  { id: "game/TemperPCE", folder: "TemperPCE", title: "Temper (PC Engine)", title_id: null, category: null, kind: "homebrew", path: "/Volumes/MEMORY STICK/PSP/GAME/TemperPCE", size_bytes: 2.1 * MB, has_icon: false, psp_system_ver: null, version: "0.82" },
  { id: "game/BookrMod", folder: "BookrMod", title: "Bookr Mod (PDF reader)", title_id: null, category: null, kind: "homebrew", path: "/Volumes/MEMORY STICK/PSP/GAME/BookrMod", size_bytes: 3.4 * MB, has_icon: false, psp_system_ver: null, version: null },
  { id: "game/ARK_Loader", folder: "ARK_Loader", title: "ARK Loader", title_id: null, category: null, kind: "loader", path: "/Volumes/MEMORY STICK/PSP/GAME/ARK_Loader", size_bytes: 860 * 1024, has_icon: true, psp_system_ver: null, version: null },
];

const images = [
  { id: "iso/Metal Gear Solid - Peace Walker.iso", file: "Metal Gear Solid - Peace Walker.iso", format: "iso", title: "Metal Gear Solid Peace Walker", title_id: "ULUS-10509", is_umd: true, path: "/Volumes/MEMORY STICK/ISO/Metal Gear Solid - Peace Walker.iso", size_bytes: 1.48 * GB },
  { id: "iso/Persona 3 Portable.cso", file: "Persona 3 Portable.cso", format: "cso", title: null, title_id: null, is_umd: true, path: "/Volumes/MEMORY STICK/ISO/Persona 3 Portable.cso", size_bytes: 780 * MB },
  { id: "iso/Daxter.cso", file: "Daxter.cso", format: "cso", title: null, title_id: null, is_umd: true, path: "/Volumes/MEMORY STICK/ISO/Daxter.cso", size_bytes: 690 * MB },
];

const music = [
  { rel: "Massive Attack/Mezzanine/01 Angel.mp3", name: "01 Angel.mp3", size_bytes: 7.8 * MB },
  { rel: "Massive Attack/Mezzanine/02 Risingson.mp3", name: "02 Risingson.mp3", size_bytes: 6.9 * MB },
  { rel: "Nujabes/Metaphorical Music/01 Blessing It.mp3", name: "01 Blessing It.mp3", size_bytes: 8.2 * MB },
  { rel: "Radiohead/In Rainbows/01 15 Step.mp3", name: "01 15 Step.mp3", size_bytes: 5.9 * MB },
];

const videos = [
  { rel: "Blade Runner 2049 (PSP).mp4", name: "Blade Runner 2049 (PSP).mp4", size_bytes: 640 * MB },
  { rel: "Akira (480x272).mp4", name: "Akira (480x272).mp4", size_bytes: 512 * MB },
];

const saves = [
  { folder: "UCUS98737GODOFWAR", title: "God of War: Chains of Olympus", subtitle: "Chapter 3: The Sun Shield", title_id: "UCUS-98737", size_bytes: 512 * 1024, has_icon: true },
  { folder: "UCUS98712WIPEOUT", title: "Wipeout Pure", subtitle: "Classic Grid", title_id: "UCUS-98712", size_bytes: 128 * 1024, has_icon: true },
  { folder: "ULUS10509PEACEWALKER", title: "Metal Gear Solid Peace Walker", subtitle: "Outer Base", title_id: "ULUS-10509", size_bytes: 2.2 * MB, has_icon: true },
];

const plugins = {
  game: [
    { path: "ms0:/seplugins/cwcheat/cwcheat.prx", enabled: true, exists: true, section: "game" },
    { path: "ms0:/seplugins/gamesharing.prx", enabled: false, exists: true, section: "game" },
  ],
  vsh: [
    { path: "ms0:/seplugins/cxmb/cxmb.prx", enabled: true, exists: true, section: "vsh" },
    { path: "ms0:/seplugins/categorieslite.prx", enabled: false, exists: false, section: "vsh" },
  ],
  pops: [
    { path: "ms0:/seplugins/cdda_enabler.prx", enabled: true, exists: true, section: "pops" },
  ],
};

const themes = [
  { name: "Metal Gear.ctf", format: "ctf", size_bytes: 2.4 * MB },
  { name: "Classic Waves.ptf", format: "ptf", size_bytes: 900 * 1024 },
];

const cfw = {
  ark4: true,
  ark5: false,
  pro: true,
  me: false,
  time_machine: false,
  recovery: true,
  iso_driver: true,
  plugins: ["ms0:/seplugins/cwcheat/cwcheat.prx", "ms0:/seplugins/cxmb/cxmb.prx"],
};

const cfwPlan = [
  "Back up PSP/SAVEDATA (saves) and the whole seplugins/ folder to your computer before changing anything.",
  "Install FasterARK (ARK-5) from the install card. It copies the ARK_UPDATE PSP tree and the cIPL installer onto the stick.",
  "Eject safely, then boot the ARK Loader on the console and confirm ARK-5 starts from the launcher.",
  "Run the custom IPL installer inside ARK and choose New cIPL. This writes the firmware to flash and is permanent for the console, so only do it after ARK-5 boots reliably.",
  "After the first clean ARK-5 boot, retire the old loaders: ARK-4 (PSP/SAVEDATA/ARK_01234 and its loader), PRO CFW (PROUPDATE / FastRecovery). Remove only their loader folders, never the saves.",
  "Keep the DC10 recovery folder (and TM/ if present). It is the unbrick path if a firmware write goes wrong.",
  "Re-check plugins in the Plugins tab after the upgrade: ARK-5 still reads the classic seplugins game.txt, vsh.txt and pops.txt lines.",
];

const catalog = [
  { id: "ark-4", name: "ARK-4", description: "Next generation custom firmware for PSP (and Vita ePSP). Live CFW with custom launcher, plugins, Inferno ISO driver.", category: "cfw", repo: "PSP-Archive/ARK-4", layout: "ark", note: "Copies ARK_01234 to PSP/SAVEDATA and loaders to PSP/GAME.", latest_tag: "v4.20.69 r206", zip_url: "https://example.invalid/ark4.zip", published_at: "2026-05-15T00:23:00Z" },
  { id: "fasterark", name: "ARK-5 (FasterARK)", description: "ARK loader and installer for PSP. Ships the ARK_UPDATE PSP tree plus cIPL installers; the successor path from ARK-4.", category: "cfw", repo: "PSP-Arkfive/FasterARK", layout: "psp_root", note: "Installs the ARK_UPDATE PSP folder. On the console run CustomIPL to install New cIPL; the full variant adds DC10.", latest_tag: "r3", zip_url: "https://example.invalid/fasterark.zip", published_at: "2026-09-02T10:00:00Z" },
  { id: "froggba", name: "FrogGBA", description: "GBA emulator for PSP, based on TempGBA4PSP with gpSP Kai optimizations.", category: "emu", repo: "tzubertowski/FrogGBA", layout: "psp_root", note: "Needs gba_bios.bin dropped into the game folder after install.", latest_tag: "v0.4.0-rc.1", zip_url: "https://example.invalid/froggba.zip", published_at: "2026-08-17T19:30:00Z" },
  { id: "tempgba", name: "TempGBA4PSP-mod", description: "Long-maintained GBA emulator with 2026 fixes, 16:9 full screen and a single-game mode.", category: "emu", repo: "andymcca/TempGBA4PSP-mod", layout: "auto", note: "Needs gba_bios.bin in the emulator folder.", latest_tag: "20260807", zip_url: "https://example.invalid/tempgba.zip", published_at: "2026-08-07T17:40:00Z" },
  { id: "daedalusx64", name: "DaedalusX64", description: "N64 emulator for PSP using the Media Engine for asynchronous audio.", category: "emu", repo: "DaedalusX64/daedalus", layout: "auto", note: null, latest_tag: "1.1.8", zip_url: "https://example.invalid/daedalus.zip", published_at: "2026-10-23T01:19:00Z" },
];

const installed = [
  { id: "ark-4", installed: true },
  { id: "fasterark", installed: false },
  { id: "froggba", installed: true },
  { id: "tempgba", installed: false },
  { id: "daedalusx64", installed: false },
];

let jobs: any[] = [];
let nextJobId = 100;

const doctor = [
  { name: "data dir", ok: true, detail: "/Users/demo/Library/Application Support/godsend-psp" },
  { name: "ffmpeg", ok: true, detail: "ffmpeg version 7.1 (built with psp presets)" },
  { name: "ffprobe", ok: true, detail: "found" },
  { name: "device", ok: true, detail: "/Volumes/MEMORY STICK" },
  { name: "catalog cache", ok: true, detail: "cached (2h ago)" },
];

function snapshot() {
  return { volumes, active: pspVolume.mount, config, doctor, jobs };
}

// ---- fake jobs ----

function startJob(kind: string, label: string): number {
  const id = ++nextJobId;
  const job = {
    id,
    kind,
    label,
    state: "running",
    done_bytes: 0,
    total_bytes: 100,
    message: "working",
    started_at: Math.floor(Date.now() / 1000),
    finished_at: null as number | null,
  };
  jobs = [job, ...jobs].slice(0, 25);
  let done = 0;
  const timer = setInterval(() => {
    done += 4 + Math.random() * 14;
    const current = jobs.find((j) => j.id === id);
    if (!current) {
      clearInterval(timer);
      return;
    }
    if (done >= 100) {
      done = 100;
      current.done_bytes = 100;
      current.state = "done";
      current.finished_at = Math.floor(Date.now() / 1000);
      current.message = "installed on MEMORY STICK";
      emit("job://progress", { id, done: 100, total: 100 });
      emit("job://done", { ...current });
      clearInterval(timer);
      refreshCounts();
    } else {
      current.done_bytes = Math.floor(done);
      emit("job://progress", { id, done: Math.floor(done), total: 100 });
    }
  }, 240);
  return id;
}

function refreshCounts() {
  // In mock mode an install flips installed flags after a CFW job.
}

// ---- invoke routing ----

async function invoke(cmd: string, args: Record<string, any> = {}): Promise<unknown> {
  switch (cmd) {
    case "snapshot":
      return snapshot();
    case "rescan":
      return volumes;
    case "homebrew_refresh":
      return catalog;
    case "set_active_device":
      return null;
    case "eject_device":
      return null;
    case "library_scan":
      return { games, images };
    case "icon": {
      const id = args.id as string;
      const serial = SERIAL_FOR[id];
      if (serial) {
        try {
          const res = await fetch(`demo-covers/${serial}.png`);
          if (res.ok) {
            const blob = await res.blob();
            return await new Promise<string>((resolve) => {
              const reader = new FileReader();
              reader.onload = () => resolve(String(reader.result));
              reader.readAsDataURL(blob);
            });
          }
        } catch {
          /* fall through to generated art */
        }
      }
      return iconDataUrl(id);
    }
    case "game_delete": {
      const i = games.findIndex((g) => g.id === args.id);
      if (i >= 0) games.splice(i, 1);
      return null;
    }
    case "homebrew_catalog":
      return catalog;
    case "homebrew_installed":
      return installed;
    case "homebrew_install": {
      const entry = installed.find((e) => e.id === args.id);
      startJob("install", `${catalog.find((c) => c.id === args.id)?.name ?? args.id}`);
      setTimeout(() => {
        if (entry) entry.installed = true;
        emit("device://changed", volumes);
      }, 2600);
      return nextJobId;
    }
    case "homebrew_import":
      return startJob("install", "import homebrew.zip");
    case "cfw_detect":
      return cfw;
    case "cfw_plan":
      return cfwPlan;
    case "music_list":
      return music;
    case "video_list":
      return videos;
    case "animation":
      return null;
    case "video_preview":
      return null;
    case "media_delete": {
      const list = args.kind === "video" ? videos : music;
      const i = list.findIndex((f) => f.rel === args.rel);
      if (i >= 0) list.splice(i, 1);
      return null;
    }
    case "music_push":
      return args.files.map((f: string, i: number) => startJob("music", fileName(f) + (i >= 0 ? "" : "")));
    case "video_push":
      return args.files.map((f: string) => startJob("video", fileName(f)));
    case "iso_import":
      return startJob("iso", fileName(args.file));
    case "iso_convert":
      return startJob("convert", args.image_id);
    case "image_delete": {
      const i = images.findIndex((img) => img.id === args.image_id);
      if (i >= 0) images.splice(i, 1);
      return null;
    }
    case "savedata_icon":
      return iconDataUrl(`save/${args.folder}`);
    case "savedata_list":
      return saves;
    case "savedata_backup":
      return startJob("backup", "saves -> saves-backup.zip");
    case "savedata_restore":
      return startJob("restore", fileName(args.zip));
    case "savedata_delete": {
      const i = saves.findIndex((s) => s.folder === args.folder);
      if (i >= 0) saves.splice(i, 1);
      return null;
    }
    case "savedata_reveal":
      return `/Volumes/MEMORY STICK/PSP/SAVEDATA/${args.folder}`;
    case "plugins_list":
      return plugins;
    case "plugin_toggle": {
      const list = (plugins as any)[args.section as string] as any[];
      const entry = list.find((p) => p.path === args.path);
      if (entry) entry.enabled = args.enable;
      return null;
    }
    case "plugin_add": {
      const list = (plugins as any)[args.section as string] as any[];
      list.push({ path: `ms0:/seplugins/${fileName(args.prx)}`, enabled: true, exists: true, section: args.section });
      return list[list.length - 1];
    }
    case "plugin_remove": {
      const list = (plugins as any)[args.section as string] as any[];
      const i = list.findIndex((p) => p.path === args.path);
      if (i >= 0) list.splice(i, 1);
      return null;
    }
    case "themes_remote":
      return [];
    case "theme_install_remote":
      return { name: fileName(args.name), format: "ctf", size_bytes: 2 * MB };
    case "themes_list":
      return themes;
    case "theme_install":
      themes.push({ name: fileName(args.file), format: "ctf", size_bytes: 2 * MB });
      return themes[themes.length - 1];
    case "theme_delete": {
      const i = themes.findIndex((t) => t.name === args.name);
      if (i >= 0) themes.splice(i, 1);
      return null;
    }
    case "doctor_run":
      return doctor;
    case "debrid_status":
      return { realdebrid: true, torbox: false };
    case "store_search":
      if (args.source === "ia") {
        return {
          kind: "ia",
          items: [
            { id: "psp-iso-collection", title: "PSP ISO Collection", files_count: 240, size_bytes: 180 * GB },
            { id: "psp-rom-set", title: "PSP ROM Set (megathread)", files_count: 620, size_bytes: 420 * GB },
          ],
        };
      }
      if (args.source === "rroms") {
        return {
          kind: "rroms",
          items: [
            { id: "sony_playstation_portable_part1", title: "PSP Redump .iso (Part 1)", url: "https://archive.org/download/sony_playstation_portable_part1", kind: "ia_item" },
            { id: "PSP-DLC/PSP DLC", title: "PSP DLC", url: "https://archive.org/download/PSP-DLC/PSP%20DLC/", kind: "ia_item" },
            { id: "psp-homebrew-pack.zip", title: "PSP homebrew pack", url: "https://example.invalid/psp-homebrew-pack.zip", kind: "zip" },
          ],
        };
      }
      if (args.source === "nps") {
        return {
          kind: "nps",
          items: [
            { title_id: "UCUS98662", title: "LocoRoco", region: "US", url: "https://example.invalid/UCUS98662.pkg", size_bytes: 340 * MB, zrif: "6A1B2C3D4E5F60718293A4B5C6D7E8F9" },
            { title_id: "UCUS98731", title: "LocoRoco 2", region: "US", url: "https://example.invalid/UCUS98731.pkg", size_bytes: 640 * MB, zrif: null },
          ],
        };
      }
      return {
        kind: "gamesearch",
        items: [
          { id: "1", slug: "locoroco-2", title: "LocoRoco 2", platform: "psp", source: "minerva",
            host: "minerva-archive.org", url: "https://example.invalid/locoroco2.zip", linkType: "ddl",
            supportedBy: ["torbox", "realdebrid"], cover: null, size: 640 * MB,
            links: [{ host: "minerva-archive.org", url: "https://example.invalid/locoroco2.zip", linkType: "ddl", supportedBy: ["torbox", "realdebrid"] }] },
          { id: "2", slug: "wipeout-pure", title: "Wipeout Pure", platform: "psp", source: "rroms",
            host: "rroms.org", url: "magnet:?xt=urn:btih:demo", linkType: "magnet",
            supportedBy: ["torbox", "realdebrid"], cover: null, size: 420 * MB,
            links: [{ host: "rroms.org", url: "magnet:?xt=urn:btih:demo", linkType: "magnet", supportedBy: ["torbox", "realdebrid"] }] },
        ],
      };
    case "store_detail":
      if (args.source === "ia") {
        return { kind: "ia", files: [{ name: "LocoRoco 2 (USA).iso", size_bytes: 640 * MB, url: "https://example.invalid/x.iso" }] };
      }
      if (args.source === "rroms") {
        return {
          kind: "rroms",
          files: [
            { name: "LocoRoco 2 (USA).zip", size_bytes: 640 * MB, url: "https://example.invalid/locoroco2.zip" },
            { name: "Wipeout Pure (USA).zip", size_bytes: 420 * MB, url: "https://example.invalid/wipeout.zip" },
          ],
        };
      }
      return { kind: "gamesearch", hit: null };
    case "store_download":
      return startJob("download", String(args.id));
    case "downloads_list":
      return {
        dir: "/Users/demo/Downloads/godsend-psp",
        files: [
          { name: "LocoRoco 2 (USA).iso", path: "/Users/demo/Downloads/godsend-psp/LocoRoco 2 (USA).iso", is_dir: false, size_bytes: 640 * MB, kind: "iso" },
          { name: "FrogGBA.zip", path: "/Users/demo/Downloads/godsend-psp/FrogGBA.zip", is_dir: false, size_bytes: 4 * MB, kind: "other" },
          { name: "mods", path: "/Users/demo/Downloads/godsend-psp/mods", is_dir: true, size_bytes: 0, kind: "other" },
        ],
      };
    case "download_dir_set":
    case "download_open":
      return args.name ?? null;
    case "download_delete":
      return null;
    case "download_push":
      return startJob("copy", `\${args.name} -> PSP`);
    case "fm_list":
      return [
        { name: "PSP", path: `\${args.path}/PSP`, is_dir: true, size_bytes: 0, kind: "other" },
        { name: "ISO", path: `\${args.path}/ISO`, is_dir: true, size_bytes: 0, kind: "other" },
        { name: "seplugins", path: `\${args.path}/seplugins`, is_dir: true, size_bytes: 0, kind: "other" },
      ];
    case "fm_mkdir":
      return `\${args.path}/\${args.name}`;
    case "fm_delete":
      return null;
    case "fm_to_psp":
    case "fm_from_psp":
      return startJob("copy", "transfer");
    case "local_library":
      return [
        { id: "local/Peace Walker.iso", name: "Peace Walker.iso", kind: "iso", path: "/Users/demo/Downloads/godsend-psp/Peace Walker.iso", size_bytes: 1.5 * GB, serial: "ULUS-10509", title: "Metal Gear Solid Peace Walker" },
      ];
    case "game_info":
      return {
        known: { serial: args.serial || "UCUS-98737", title: "God of War: Chains of Olympus", cover: "", animation: "" },
        animation: args.serial ? `https://covers.example.com/psp/\${args.serial}.webm` : null,
        mods: [
          { name: "60 FPS patch", author: "community", version: "1.2", url: "https://example.invalid/60fps.zip", size_bytes: 120 * 1024, notes: "smoother gameplay", kind: "iso_patch" },
        ],
      };
    case "cover":
      return iconDataUrl(String(args.serial || args.id));
    case "mods_list":
      return [];
    case "mods_install":
      return startJob("download", String(args.name));
    case "config_get":
      return config;
    case "config_set":
      Object.assign(config, args.patch);
      return config;
    case "jobs":
      return jobs;
    case "job_cancel":
      return true;
    case "jobs_clear":
      jobs = jobs.filter((j) => j.state === "running");
      return null;
    case "reveal":
    case "open_data_dir":
      return null;
    case "plugin:dialog|open": {
      const options = (args.options ?? {}) as { filters?: { extensions: string[] }[] };
      const ext = options.filters?.[0]?.extensions?.[0] ?? "zip";
      return [`/Users/demo/Downloads/${ext === "prx" ? "cwcheat.prx" : ext === "iso" ? "Persona 2.iso" : "homebrew-pack.zip"}`];
    }
    case "plugin:dialog|save":
      return "/Users/demo/Downloads/saves-backup-mock.zip";
    case "plugin:event|listen": {
      const id = nextCallbackId++;
      listeners.set(id, { event: args.event, handler: args.handler });
      return id;
    }
    case "plugin:event|unlisten":
      listeners.delete(args.eventId);
      return null;
    default:
      console.warn("[mock] unhandled invoke:", cmd, args);
      return null;
  }
}

// Real cover art, by serial, dropped into docs/demo-covers/.
const SERIAL_FOR: Record<string, string> = {
  "game/GodOfWarChainsOfOlympus": "UCUS-98737",
  "game/LocoRoco2": "UCUS-98716",
  "game/WipeoutPure": "UCUS-98712",
  "iso/Metal Gear Solid - Peace Walker.iso": "ULUS-10509",
  "iso/Persona 3 Portable.cso": "ULUS-10512",
  "iso/Daxter.cso": "UCUS-98634",
};

function fileName(path: string): string {
  return path.split("/").pop() ?? path;
}

function iconDataUrl(id: string): string {
  // Draw a deterministic 96x96 tile with a monogram; reads as a real cover at grid size.
  const canvas = document.createElement("canvas");
  canvas.width = 288;
  canvas.height = 160;
  const ctx = canvas.getContext("2d")!;
  let hash = 0;
  for (const ch of id) hash = (hash * 31 + ch.charCodeAt(0)) | 0;
  const hue = Math.abs(hash) % 360;
  const grad = ctx.createLinearGradient(0, 0, 288, 160);
  grad.addColorStop(0, `hsl(${hue} 58% 34%)`);
  grad.addColorStop(1, `hsl(${(hue + 45) % 360} 62% 16%)`);
  ctx.fillStyle = grad;
  ctx.fillRect(0, 0, 288, 160);
  const glow = ctx.createRadialGradient(230, 10, 10, 230, 10, 160);
  glow.addColorStop(0, "rgba(255,255,255,0.22)");
  glow.addColorStop(1, "rgba(255,255,255,0)");
  ctx.fillStyle = glow;
  ctx.fillRect(0, 0, 288, 160);
  ctx.fillStyle = "rgba(255,255,255,0.95)";
  ctx.font = "700 22px 'Chakra Petch', sans-serif";
  ctx.textAlign = "left";
  ctx.textBaseline = "bottom";
  const words = id.replace(/^game\//, "").replace(/^iso\//, "").replace(/\.[a-z]+$/i, "");
  const title = words.replace(/([a-z])([A-Z])/g, "$1 $2").replace(/-/g, " ");
  ctx.fillText(title.slice(0, 26), 16, 128);
  ctx.fillStyle = "rgba(255,255,255,0.6)";
  ctx.font = "500 10px 'JetBrains Mono', monospace";
  ctx.fillText(words.slice(0, 30).toUpperCase(), 16, 146);
  return canvas.toDataURL("image/png");
}

// install the shim
(window as unknown as Record<string, any>).__TAURI_INTERNALS__ = {
  invoke,
  transformCallback(cb: Handler) {
    const id = nextCallbackId++;
    listeners.set(id, { event: "__callback__", handler: cb });
    // plugin:event|listen replaces this entry with the real event name.
    return id;
  },
  metadata: { currentWindow: { label: "main" }, currentWebview: { windowLabel: "main" } },
  plugins: {},
};

console.info("[mock] GODsend PSP fixture bridge installed");

export {}; // make this file a module so dynamic import type-checks

// Seed a couple of finished jobs so the inspector is alive.
jobs = [
  {
    id: 2, kind: "music", label: "01 15 Step.mp3", state: "done",
    done_bytes: 100, total_bytes: 100, message: "on MEMORY STICK as 01 15 Step.mp3",
    started_at: Math.floor(Date.now() / 1000) - 600, finished_at: Math.floor(Date.now() / 1000) - 590,
  },
  {
    id: 1, kind: "install", label: "FrogGBA v0.4.0-rc.1", state: "done",
    done_bytes: 100, total_bytes: 100, message: "installed 2 item(s) onto MEMORY STICK",
    started_at: Math.floor(Date.now() / 1000) - 3600, finished_at: Math.floor(Date.now() / 1000) - 3590,
  },
];
