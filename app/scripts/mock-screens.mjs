// Screenshot smoke test against the mock bridge (vite dev server).
import { chromium } from "@playwright/test";

const BASE = process.env.BASE_URL ?? "http://localhost:1420";
const PAGES = ["home", "games", "store", "mods", "music"];

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });

for (const route of PAGES) {
  await page.goto(`${BASE}/?mock=1#/`, { waitUntil: "load", timeout: 30000 });
  await page.waitForTimeout(1200);
  // click the nav item by label
  const labels = { home: "Console", games: "Games", store: "Homebrew", mods: "Mods", music: "Music" };
  await page.getByRole("button", { name: labels[route] }).click();
  await page.waitForTimeout(900);
  await page.screenshot({ path: `screens/mock-${route}.png` });
  console.log(`captured ${route}`);
}

await browser.close();
