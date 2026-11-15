import { chromium } from "@playwright/test";
import { readFileSync } from "node:fs";
const icon = readFileSync("src-tauri/icons/icon.png").toString("base64");
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1200, height: 630 } });
await page.setContent(`<html><body style="margin:0;background:#04060c;font-family:sans-serif">
<div style="position:relative;width:1200px;height:630px;display:flex;align-items:center;gap:28px;padding:0 80px;box-sizing:border-box;color:#eef2fb">
<div style="position:absolute;inset:0;background:radial-gradient(700px 400px at 80% 0%,rgba(109,139,255,.18),transparent 60%),radial-gradient(500px 300px at 0% 100%,rgba(86,232,245,.12),transparent 60%)"></div>
<img src="data:image/png;base64,${icon}" style="width:170px;height:170px;position:relative"/>
<div style="position:relative">
<div style="font:700 58px sans-serif">GODsend PSP</div>
<div style="margin-top:10px;font:600 22px monospace;color:#56e8f5;letter-spacing:.18em">STORE · HOMEBREW · MEDIA · MODS</div>
<div style="margin-top:16px;font:400 24px sans-serif;color:#a5b0c7">Manage the whole memory stick from one window.</div>
</div></div></body></html>`);
await page.waitForTimeout(300);
await page.screenshot({ path: "../docs/og.png" });
await browser.close();
console.log("og.png rendered");
