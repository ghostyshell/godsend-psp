import { chromium } from "@playwright/test";
import { readFileSync, writeFileSync } from "node:fs";

const src = process.argv[2];
const buf = readFileSync(src).toString("base64");
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1024, height: 1024 } });
const result = await page.evaluate(async (b64) => {
  const img = new Image();
  img.src = "data:image/png;base64," + b64;
  await img.decode();
  const c = document.createElement("canvas");
  c.width = img.width; c.height = img.height;
  const ctx = c.getContext("2d");
  ctx.drawImage(img, 0, 0);
  const data = ctx.getImageData(0, 0, c.width, c.height);
  const px = data.data;
  let minX = c.width, minY = c.height, maxX = 0, maxY = 0;
  for (let i = 0; i < px.length; i += 4) {
    const r = px[i], g = px[i+1], b = px[i+2];
    // pure-white background to transparent, keep the tinted highlights
    if (r > 246 && g > 246 && b > 246) { px[i+3] = 0; continue; }
    const x = (i/4) % c.width, y = Math.floor((i/4) / c.width);
    if (x < minX) minX = x; if (x > maxX) maxX = x;
    if (y < minY) minY = y; if (y > maxY) maxY = y;
  }
  ctx.putImageData(data, 0, 0);
  // square crop around the art with a small margin, then scale to 1024
  const w = maxX - minX, h = maxY - minY;
  const side = Math.max(w, h) * 1.06;
  const cx = (minX + maxX) / 2, cy = (minY + maxY) / 2;
  const out = document.createElement("canvas");
  out.width = 1024; out.height = 1024;
  const octx = out.getContext("2d");
  octx.imageSmoothingQuality = "high";
  octx.drawImage(c, cx - side/2, cy - side/2, side, side, 0, 0, 1024, 1024);
  return out.toDataURL("image/png");
}, buf);
await browser.close();
const png = Buffer.from(result.split(",")[1], "base64");
writeFileSync("src-tauri/icons/icon.png", png);
writeFileSync("../docs/icon.png", png);
console.log("iconized", png.length, "bytes");
