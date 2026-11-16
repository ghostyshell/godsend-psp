#!/usr/bin/env node
// Scrape the hosted PSP catalog sources listed in psp-catalog/sources.json.
//
// Each source is {kind: "themes"|"mods", url, name?, selector?, serial?}.
// themes sources contribute .ctf/.ptf files and rewrite themes.json; mods
// sources need a serial and add entries to mods/<SERIAL>.json. Downloads land
// in psp-catalog/files and are deduplicated by SHA-256 across runs. `name`
// overrides the on-disk filename for a direct file URL.
//
//   node scripts/scrape-psp-catalog.mjs
//
// No sources configured means no network access at all.
import { createHash } from "node:crypto";
import { existsSync } from "node:fs";
import { mkdir, readFile, readdir, rename, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { basename, extname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const REPO = resolve(fileURLToPath(new URL("..", import.meta.url)));
const CATALOG = resolve(REPO, process.env.PSP_CATALOG_DIR || "psp-catalog");
const SOURCES = join(CATALOG, "sources.json");
const PUBLIC_BASE = (process.env.PSP_CATALOG_PUBLIC_BASE || "https://covers.debrid.in/psp").replace(/\/+$/, "");
const THEME_EXT = new Set([".ctf", ".ptf"]);
const MOD_EXT = new Set([".zip", ".7z", ".rar", ".prx", ".pbp"]);
const THEME_FORMATS = new Set(["ctf", "ptf"]);

function log(message) {
  console.log(message);
}

function fail(message) {
  console.error(message);
  process.exit(1);
}

export function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function extensionOf(nameOrUrl) {
  return extname(String(nameOrUrl).split("?")[0].split("#")[0]).toLowerCase();
}

function cleanName(value) {
  return (
    String(value)
      .split(/[\\/]/)
      .pop()
      .replace(/[^\w.\- ]+/g, "-")
      .replace(/\s+/g, " ")
      .replace(/^[.\- ]+|[.\- ]+$/g, "") || "file"
  );
}

function cleanFileName(url) {
  let name;
  try {
    const path = new URL(url).pathname;
    name = decodeURIComponent(path.split("/").pop() || "");
  } catch {
    name = String(url).split("?")[0].split("/").pop() || "";
  }
  return cleanName(name);
}

function publicUrl(rel) {
  return `${PUBLIC_BASE}/files/${rel.split("/").map(encodeURIComponent).join("/")}`;
}

function displayName(rel) {
  return basename(rel);
}

function modKind(name) {
  const ext = extensionOf(name);
  if (ext === ".prx") return "plugin";
  if (ext === ".iso" || ext === ".cso") return "iso_patch";
  return "other";
}

async function readJson(path, fallback) {
  try {
    return JSON.parse(await readFile(path, "utf8"));
  } catch (err) {
    if (err.code === "ENOENT") return fallback;
    throw err;
  }
}

async function writeJson(path, data) {
  const tmp = `${path}.tmp`;
  await writeFile(tmp, `${JSON.stringify(data, null, 2)}\n`);
  await rename(tmp, path);
}

async function walkFiles(dir, base = "") {
  let entries;
  try {
    entries = await readdir(dir, { withFileTypes: true });
  } catch (err) {
    if (err.code === "ENOENT") return [];
    throw err;
  }
  const out = [];
  for (const entry of entries) {
    if (entry.name === ".DS_Store") continue;
    const rel = base ? `${base}/${entry.name}` : entry.name;
    if (entry.isDirectory()) {
      out.push(...(await walkFiles(join(dir, entry.name), rel)));
    } else if (entry.isFile()) {
      out.push(rel);
    }
  }
  return out;
}

async function hashExisting(filesRoot) {
  const byHash = new Map();
  const usedNames = new Set();
  for (const rel of await walkFiles(filesRoot)) {
    const bytes = await readFile(join(filesRoot, rel));
    const hash = sha256(bytes);
    byHash.set(hash, { rel, size: bytes.length });
    usedNames.add(rel);
  }
  return { byHash, usedNames };
}

function uniqueRel(name, hash, usedNames) {
  if (!usedNames.has(name)) return name;
  const ext = extname(name);
  const stem = ext ? name.slice(0, -ext.length) : name;
  return `${stem}-${hash.slice(0, 8)}${ext}`;
}

async function loadSources() {
  if (!existsSync(SOURCES)) return { sources: [] };
  let parsed;
  try {
    parsed = JSON.parse(await readFile(SOURCES, "utf8"));
  } catch (err) {
    fail(`sources.json: invalid JSON: ${err.message}`);
  }
  if (!Array.isArray(parsed)) fail("sources.json: expected a JSON array");
  const sources = [];
  parsed.forEach((raw, index) => {
    if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
      fail(`sources.json[${index}]: expected an object`);
    }
    // _comment keys are documentation only.
    const kind = String(raw.kind || "").toLowerCase();
    if (kind !== "themes" && kind !== "mods") {
      fail(`sources.json[${index}]: kind must be "themes" or "mods"`);
    }
    const url = String(raw.url || "").trim();
    if (!/^https?:\/\//i.test(url)) {
      fail(`sources.json[${index}]: url must start with http:// or https://`);
    }
    const source = { kind, url };
    if (raw.name) source.name = cleanName(raw.name);
    if (raw.selector) source.selector = String(raw.selector);
    if (raw.serial) source.serial = String(raw.serial).trim().toUpperCase();
    sources.push(source);
  });
  return { sources };
}

async function loadChromium() {
  const require = createRequire(import.meta.url);
  const candidates = [
    join(REPO, "app"),
    REPO,
  ];
  for (const moduleName of ["playwright", "@playwright/test"]) {
    try {
      const entry = require.resolve(moduleName, { paths: candidates });
      const namespace = await import(pathToFileURL(entry).href);
      const chromium = namespace.chromium || namespace.default?.chromium;
      if (chromium) return chromium;
    } catch {
      // Try the next module location.
    }
  }
  fail("playwright not found; run npm --prefix app install first");
}

async function collectLinks(page, source, extensions) {
  const directExt = extensionOf(source.url);
  if (extensions.has(directExt)) return [source.url];

  await page.goto(source.url, { waitUntil: "domcontentloaded", timeout: 45_000 });
  await page.waitForTimeout(500);
  const selector = source.selector || "body";
  let hrefs;
  try {
    hrefs = await page.evaluate(
      ({ scope, exts }) => {
        const root = document.querySelector(scope);
        if (!root) return [];
        return Array.from(root.querySelectorAll("a[href]"))
          .map((a) => a.href)
          .filter((href) => {
            const clean = href.split("?")[0].split("#")[0].toLowerCase();
            return exts.some((ext) => clean.endsWith(ext));
          });
      },
      { scope: selector, exts: [...extensions] }
    );
  } catch (err) {
    throw new Error(`selector ${selector}: ${err.message}`);
  }
  return [...new Set(hrefs)];
}

async function download(context, href) {
  const response = await context.request.get(href, { timeout: 90_000 });
  if (!response.ok()) {
    throw new Error(`${response.status()} ${response.statusText()} for ${href}`);
  }
  return response.body();
}

async function main() {
  const { sources } = await loadSources();
  if (sources.length === 0) {
    log("no sources configured");
    return;
  }

  await mkdir(join(CATALOG, "files"), { recursive: true });
  const themesPath = join(CATALOG, "themes.json");
  const existingThemes = await readJson(themesPath, []);
  if (!Array.isArray(existingThemes)) fail("themes.json: expected a JSON array");
  const themesByName = new Map();
  for (const theme of existingThemes) {
    if (theme && typeof theme === "object" && theme.name) themesByName.set(theme.name, theme);
  }

  const { byHash, usedNames } = await hashExisting(join(CATALOG, "files"));
  const known = byHash;
  let written = 0;
  let writtenBytes = 0;
  let reused = 0;
  let themeChanges = 0;
  const modsTouched = new Set();
  const modsCache = new Map();
  let failures = 0;

  const chromium = await loadChromium();
  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext();

  async function persist(href, preferredName) {
    const bytes = await download(context, href);
    const hash = sha256(bytes);
    const prior = known.get(hash);
    if (prior) {
      reused += 1;
      return prior;
    }
    const name = uniqueRel(preferredName || cleanFileName(href), hash, usedNames);
    const dest = join(CATALOG, "files", name);
    await mkdir(join(CATALOG, "files"), { recursive: true });
    await writeFile(dest, bytes);
    usedNames.add(name);
    const entry = { rel: name, size: bytes.length };
    known.set(hash, entry);
    written += 1;
    writtenBytes += bytes.length;
    return entry;
  }

  try {
    for (const source of sources) {
      try {
        const page = await context.newPage();
        const extensions = source.kind === "themes" ? THEME_EXT : MOD_EXT;
        const links = await collectLinks(page, source, extensions);
        log(`${source.kind}: ${links.length} file link(s) from ${source.url}`);

        if (source.kind === "themes") {
          for (const href of links) {
            const preferred = source.name && links.length === 1 ? source.name : undefined;
            const file = await persist(href, preferred);
            const ext = extensionOf(file.rel).replace(".", "");
            if (!THEME_FORMATS.has(ext)) continue;
            const name = displayName(file.rel);
            themesByName.set(name, {
              name,
              url: publicUrl(file.rel),
              format: ext,
              size_bytes: file.size,
            });
            themeChanges += 1;
          }
        } else {
          if (!source.serial) {
            log(`mods: skip ${source.url}, no serial configured`);
            await page.close();
            continue;
          }
          const modsPath = join(CATALOG, "mods", `${source.serial}.json`);
          if (!modsCache.has(source.serial)) {
            const rows = await readJson(modsPath, []);
            if (!Array.isArray(rows)) fail(`mods/${source.serial}.json: expected a JSON array`);
            modsCache.set(source.serial, rows);
          }
          const rows = modsCache.get(source.serial);
          for (const href of links) {
            const file = await persist(href);
            const url = publicUrl(file.rel);
            const name = displayName(file.rel);
            const row = {
              name,
              author: "",
              version: "",
              url,
              size_bytes: file.size,
              notes: "",
              kind: modKind(name),
            };
            const index = rows.findIndex((item) => item && item.url === url);
            if (index >= 0) rows[index] = { ...rows[index], ...row };
            else rows.push(row);
            modsTouched.add(source.serial);
          }
        }
        await page.close();
      } catch (err) {
        failures += 1;
        console.error(`skip ${source.url}: ${err.message}`);
      }
    }
  } finally {
    await browser.close();
  }

  const themes = [...themesByName.values()].sort((a, b) => String(a.name).localeCompare(String(b.name)));
  await writeJson(themesPath, themes);
  for (const serial of modsTouched) {
    const rows = modsCache.get(serial).sort((a, b) => String(a.name).localeCompare(String(b.name)));
    await mkdir(join(CATALOG, "mods"), { recursive: true });
    await writeJson(join(CATALOG, "mods", `${serial}.json`), rows);
  }

  log(`themes: ${themes.length} entries, ${themeChanges} scraped`);
  log(`mods: ${[...modsTouched].length} serial file(s) updated`);
  log(
    `files: ${written} written (${(writtenBytes / (1024 * 1024)).toFixed(2)} MiB), ${reused} reused by sha256`
  );
  if (failures > 0) {
    console.error(`${failures} source(s) failed`);
    process.exit(1);
  }
}

if (process.argv[1] && process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((err) => fail(err.stack || err.message));
}
