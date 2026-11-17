import { test, expect } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/?mock=1", { waitUntil: "load" });
  await page.waitForTimeout(1500);
});

test("shell landmarks and nav", async ({ page }) => {
  await expect(page.locator("header.topbar")).toBeVisible();
  await expect(page.locator("nav.rail")).toBeVisible();
  await expect(page.locator("footer.statusbar")).toBeVisible();
  await expect(page.getByRole("link", { name: "Skip to content" })).toHaveCount(1);
  const current = page.locator('.rail-item[aria-current="page"]');
  await expect(current).toHaveText(/Console/);
});

test("macOS header is a drag region", async ({ page }) => {
  await expect(page.locator("header.topbar[data-tauri-drag-region]")).toHaveCount(1);
  await expect(page.locator("header.topbar .page-title[data-tauri-drag-region]")).toHaveCount(1);
  await expect(page.locator("nav.rail button[data-tauri-drag-region]")).toHaveCount(0);
});

test("buttons stay legible on hover", async ({ page }) => {
  const primary = page.getByRole("button", { name: "Rescan" });
  await expect(primary).toHaveClass(/primary/);
  await primary.hover();
  const primaryStyle = await primary.evaluate((el) => {
    const s = getComputedStyle(el);
    return { color: s.color, bg: s.backgroundImage };
  });
  expect(primaryStyle.bg).not.toBe("none");
  expect(primaryStyle.bg).toContain("gradient");

  const ghost = page.getByRole("button", { name: "Run Checks" });
  await ghost.hover();
  const ghostColor = await ghost.evaluate((el) => getComputedStyle(el).color);
  const rgb = ghostColor.match(/\d+/g)?.map(Number) ?? [0, 0, 0];
  const luminance = (rgb[0] + rgb[1] + rgb[2]) / 3;
  expect(luminance).toBeGreaterThan(120);
});

test("library page lists the fixture games", async ({ page }) => {
  await page.getByRole("button", { name: "Library" }).click();
  await expect(page.locator(".tile").first()).toBeVisible();
  await expect(page.locator(".tile .t").first()).toContainText(/God of War/i);
});

test("store page wires the debrid index", async ({ page }) => {
  await page.getByRole("button", { name: "Store", exact: true }).click();
  await page.getByRole("tab", { name: "Game Debrid" }).click();
  await expect(page.getByText("LocoRoco 2")).toBeVisible();
  await expect(page.getByText("Wipeout Pure")).toBeVisible();
});

test("store opens on the Minerva index", async ({ page }) => {
  await page.getByRole("button", { name: "Store", exact: true }).click();
  await expect(page.getByRole("tab", { name: "Minerva" })).toHaveAttribute(
    "aria-selected",
    "true",
  );
  // Individual game rows from the Minerva fixtures, not the torrent set names.
  await expect(page.getByText("Daxter (USA)")).toBeVisible();
  await expect(page.getByText("God of War - Chains of Olympus (USA)")).toBeVisible();
  await expect(page.getByText(/^Minerva_Myrient - /)).toHaveCount(0);
});

test("store r-roms and NoPayStation tabs list fixtures", async ({ page }) => {
  await page.getByRole("button", { name: "Store", exact: true }).click();
  await page.getByRole("tab", { name: "r-roms" }).click();
  await expect(page.getByText("Ape Escape - On the Loose (USA)")).toBeVisible();
  await page.getByRole("tab", { name: "NoPayStation" }).click();
  await expect(page.getByText("LocoRoco 2").first()).toBeVisible();
  await expect(page.getByText("UCUS98662")).toBeVisible();
});

test("store Minerva lists individual games and filters them", async ({ page }) => {
  await page.getByRole("button", { name: "Store", exact: true }).click();
  await page.getByRole("tab", { name: "Minerva" }).click();
  await expect(page.getByText("007 - From Russia with Love (Europe, Australia)")).toBeVisible();
  await expect(page.getByText("Daxter (USA)")).toBeVisible();
  await expect(
    page.getByText("Minerva_Myrient - Redump - Sony - PlayStation Portable.torrent#17"),
  ).toHaveCount(0);
  await page.getByLabel("Store search").fill("russia");
  await expect(page.getByText("Daxter (USA)")).toHaveCount(0);
  await expect(page.getByText("007 - From Russia with Love (Europe, Australia)")).toBeVisible();
});

test("downloads page lists local files", async ({ page }) => {
  await page.getByRole("button", { name: "Downloads" }).click();
  await expect(page.getByText("LocoRoco 2 (USA).iso")).toBeVisible();
});

test("homebrew catalog renders with install buttons", async ({ page }) => {
  await page.getByRole("button", { name: "Homebrew" }).click();
  await expect(page.locator(".store-card")).toHaveCount(5);
  await expect(page.locator(".store-card .name").filter({ hasText: "ARK-4" })).toBeVisible();
});

test("jailbreak page shows firmware detection", async ({ page }) => {
  await page.getByRole("button", { name: "Jailbreak" }).click();
  await expect(page.getByRole("heading", { name: "Firmware detected" })).toBeVisible();
  await expect(page.locator(".chip").filter({ hasText: "ARK-4" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Migration plan" })).toBeVisible();
  await expect(page.getByText("New cIPL is permanent")).toBeVisible();
});

test("inspector queue shows the seeded jobs", async ({ page }) => {
  await expect(page.locator(".queue-card .job-row")).toHaveCount(4);
});

test("queue card keeps the cancel button and the full progress line inside the card", async ({ page }) => {
  const card = page
    .locator(".queue-card .job-row")
    .filter({ hasText: "Naruto - Ultimate Ninja Heroes 2" })
    .first();
  await expect(card).toBeVisible();
  const cancel = card.getByRole("button", { name: "Cancel" });
  await expect(cancel).toBeVisible();

  const cardBox = await card.boundingBox();
  const cancelBox = await cancel.boundingBox();
  expect(cardBox && cancelBox).toBeTruthy();
  if (cardBox && cancelBox) {
    expect(cancelBox.x + cancelBox.width).toBeLessThanOrEqual(cardBox.x + cardBox.width + 0.5);
  }

  const msg = card.locator(".msg");
  await expect(msg).toContainText("%");
  await expect(msg).toContainText("/s");
  await expect(msg).toContainText("left");

  // Nothing may be clipped horizontally or vertically.
  const overflow = await card.evaluate((el) => {
    const msg = el.querySelector(".msg") as HTMLElement | null;
    return {
      cardX: el.scrollWidth > el.clientWidth + 1,
      msgX: msg ? msg.scrollWidth > msg.clientWidth + 1 : false,
      msgY: msg ? msg.scrollHeight > msg.clientHeight + 1 : false,
    };
  });
  expect(overflow).toEqual({ cardX: false, msgX: false, msgY: false });
});

test("running download shows percent, bytes, speed and time left", async ({ page }) => {
  await page.getByRole("button", { name: "Downloads" }).click();
  const row = page.locator(".rows .row").filter({ hasText: "Ridge Racer 2 (Europe)" });
  await expect(row).toBeVisible();
  await expect(row.locator(".sub")).toContainText("%");
  await expect(row.locator(".sub")).toContainText("/s");
  await expect(row.locator(".sub")).toContainText("of");
});

test("settings exposes config controls", async ({ page }) => {
  await page.getByRole("button", { name: "Settings" }).click();
  await expect(page.getByText("Music conversion")).toBeVisible();
  await expect(page.getByText("ISO to CSO compression level")).toBeVisible();
});

test("settings updates section shows the current version and status", async ({ page }) => {
  await page.getByRole("button", { name: "Settings" }).click();
  await expect(page.getByRole("heading", { name: "Updates" })).toBeVisible();
  await expect(page.getByText(/^godsend-psp \d+\.\d+\.\d+$/)).toBeVisible();
  await page.getByRole("button", { name: "Check for updates now" }).click();
  await expect(page.getByText("You are up to date")).toBeVisible();
});

test("store container pill stays compact and expands on hover", async ({ page }) => {
  await page.getByRole("button", { name: "Store", exact: true }).click();
  await page.getByRole("tab", { name: "Minerva" }).click();
  const pill = page.locator(".chip.container-pill").first();
  await expect(pill).toBeVisible();
  await expect(pill).toHaveAttribute(
    "title",
    "Redump - Sony - PlayStation Portable",
  );

  const rest = await pill.boundingBox();
  expect(rest).toBeTruthy();
  if (rest) expect(rest.width).toBeLessThan(130);

  await pill.hover();
  await page.waitForTimeout(400);
  const expanded = await pill.boundingBox();
  expect(expanded).toBeTruthy();
  if (rest && expanded) expect(expanded.width).toBeGreaterThan(rest.width + 20);

  // At the minimum window width the expanded pill must still leave the
  // Download action inside the row and inside the viewport.
  await page.setViewportSize({ width: 1000, height: 680 });
  await page.waitForTimeout(200);
  await pill.hover();
  await page.waitForTimeout(400);
  const row = page.locator(".rows .row").filter({ has: pill }).first();
  const download = row.getByRole("button", { name: "Download" });
  const box = await download.boundingBox();
  expect(box).toBeTruthy();
  if (box) expect(box.x + box.width).toBeLessThanOrEqual(1000);
  const overflow = await row.evaluate((el) => el.scrollWidth > el.clientWidth + 1);
  expect(overflow).toBe(false);
});

test("update banner offers install and dismiss", async ({ page }) => {
  await page.evaluate(async () => {
    const api = (window as unknown as Record<string, any>).__TAURI_INTERNALS__;
    await api.invoke("update_check", { simulate: true });
  });
  const banner = page.locator(".update-banner");
  await expect(banner).toBeVisible();
  await expect(banner).toContainText("Update 0.2.47 available");
  await banner.getByRole("button", { name: "Dismiss" }).click();
  await expect(banner).toHaveCount(0);
});

test("update banner actions stay inside the window at the minimum width", async ({ page }) => {
  await page.setViewportSize({ width: 1000, height: 680 });
  await page.goto("/?mock=1", { waitUntil: "load" });
  await page.waitForTimeout(500);
  await page.evaluate(async () => {
    const api = (window as unknown as Record<string, any>).__TAURI_INTERNALS__;
    await api.invoke("update_check", { simulate: true });
  });
  const banner = page.locator(".update-banner");
  await expect(banner).toBeVisible();
  expect(page.viewportSize()).toEqual({ width: 1000, height: 680 });

  for (const name of ["Install", "Dismiss"]) {
    const button = banner.getByRole("button", { name });
    await expect(button).toBeVisible();
    await expect(button).toBeEnabled();
    const box = await button.boundingBox();
    expect(box).toBeTruthy();
    if (box) {
      expect(box.x).toBeGreaterThanOrEqual(0);
      expect(box.y).toBeGreaterThanOrEqual(0);
      expect(box.x + box.width).toBeLessThanOrEqual(1000);
      expect(box.y + box.height).toBeLessThanOrEqual(680);
    }
    // The button's own centre must hit the button, not the inspector over it.
    const hit = await button.evaluate((el) => {
      const r = el.getBoundingClientRect();
      const at = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2);
      return !!at && (el === at || el.contains(at));
    });
    expect(hit).toBe(true);
  }

  await banner.getByRole("button", { name: "Install" }).click();
  await banner.getByRole("button", { name: "Dismiss" }).click();
  await expect(banner).toHaveCount(0);
});

test("theme studio creates a pack", async ({ page }) => {
  await page.getByRole("button", { name: "Mods" }).click();
  await page.getByRole("tab", { name: "Themes" }).click();
  await expect(page.getByRole("heading", { name: "Theme Studio" })).toBeVisible();
  await page.getByLabel("Pack name").fill("Test Pack");
  await page.getByRole("button", { name: "Create", exact: true }).click();
  await expect(page.locator(".theme-card .title").filter({ hasText: "Test Pack" })).toBeVisible();
});
