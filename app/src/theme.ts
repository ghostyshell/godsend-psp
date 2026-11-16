// Runtime theme application for Theme Studio packs.
//
// A selected pack is persisted in localStorage and re-applied on load so the
// app keeps its accent between sessions. The wallpaper file itself stays on
// the stick; here we tint the canvas from the pack accent.

import type { ThemePack } from "./types";

const STORAGE_KEY = "godsend.theme";
const DEFAULT_ACCENT = "#6d8bff";
const CANVAS = "#04060c";

export interface StoredTheme extends ThemePack {
  folder: string;
}

/** Mirror of core ``theme_pack::slug`` so the UI targets the same folder. */
export function slugify(name: string): string {
  let out = "";
  let lastDash = true;
  for (const ch of name) {
    if (/[a-z0-9]/i.test(ch)) {
      out += ch.toLowerCase();
      lastDash = false;
    } else if (!lastDash) {
      out += "-";
      lastDash = true;
    }
  }
  const trimmed = out.replace(/^-+|-+$/g, "") || "theme";
  return trimmed.slice(0, 48).replace(/-+$/g, "");
}

function parseHex(value: string): [number, number, number] | null {
  const hex = value.trim().replace(/^#/, "");
  if (!/^[0-9a-f]{3}([0-9a-f]{3})?$/i.test(hex)) return null;
  const full = hex.length === 3 ? hex.split("").map((c) => c + c).join("") : hex;
  return [
    parseInt(full.slice(0, 2), 16),
    parseInt(full.slice(2, 4), 16),
    parseInt(full.slice(4, 6), 16),
  ];
}

function toHex(rgb: [number, number, number]): string {
  return `#${rgb.map((n) => Math.round(n).toString(16).padStart(2, "0")).join("")}`;
}

/** Blend ``accent`` into the near-black canvas (weight 0..1). */
export function tint(accent: string, weight: number): string {
  const a = parseHex(accent);
  const b = parseHex(CANVAS);
  if (!a || !b) return CANVAS;
  return toHex([
    b[0] + (a[0] - b[0]) * weight,
    b[1] + (a[1] - b[1]) * weight,
    b[2] + (a[2] - b[2]) * weight,
  ]);
}

export function loadStoredTheme(): StoredTheme | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Partial<StoredTheme>;
    if (!parsed.name || !parsed.accent) return null;
    return {
      name: parsed.name,
      accent: parsed.accent,
      background: parsed.background ?? null,
      created_at: parsed.created_at ?? "",
      folder: parsed.folder ?? slugify(parsed.name),
    };
  } catch {
    return null;
  }
}

export function applyTheme(pack: Pick<ThemePack, "accent">): void {
  const root = document.documentElement;
  const accent = parseHex(pack.accent) ? pack.accent : DEFAULT_ACCENT;
  root.style.setProperty("--accent", accent);
  root.style.setProperty("--app-bg", tint(accent, 0.14));
}

export function resetTheme(): void {
  document.documentElement.style.removeProperty("--accent");
  document.documentElement.style.removeProperty("--app-bg");
}

export function selectTheme(pack: ThemePack, folder?: string): StoredTheme {
  const stored: StoredTheme = { ...pack, folder: folder ?? slugify(pack.name) };
  localStorage.setItem(STORAGE_KEY, JSON.stringify(stored));
  applyTheme(stored);
  return stored;
}

export function clearTheme(): void {
  localStorage.removeItem(STORAGE_KEY);
  resetTheme();
}

/** Called once from App on mount. */
export function applyStoredTheme(): void {
  const stored = loadStoredTheme();
  if (stored) applyTheme(stored);
}
