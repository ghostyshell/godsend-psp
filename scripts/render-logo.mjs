import { chromium } from "@playwright/test";
import { readFileSync, copyFileSync } from "node:fs";

const svg = readFileSync("assets/logo.svg", "utf8");
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1024, height: 1024 } });
await page.setContent(
  `<html><body style="margin:0;background:transparent"><div style="width:1024px;height:1024px">${svg}</div></body></html>`
);
await page.waitForTimeout(400);
await page.screenshot({ path: "app/src-tauri/icons/icon.png", omitBackground: true });
await browser.close();
copyFileSync("app/src-tauri/icons/icon.png", "docs/icon.png");
console.log("rendered app/src-tauri/icons/icon.png and docs/icon.png");
