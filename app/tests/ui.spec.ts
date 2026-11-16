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

test("library page lists the fixture games", async ({ page }) => {
  await page.getByRole("button", { name: "Library" }).click();
  await expect(page.locator(".tile").first()).toBeVisible();
  await expect(page.locator(".tile .t").first()).toContainText(/God of War/i);
});

test("store page wires the debrid index", async ({ page }) => {
  await page.getByRole("button", { name: "Store", exact: true }).click();
  await expect(page.getByText("LocoRoco 2")).toBeVisible();
  await expect(page.getByText("Wipeout Pure")).toBeVisible();
});

test("store r-roms and NoPayStation tabs list fixtures", async ({ page }) => {
  await page.getByRole("button", { name: "Store", exact: true }).click();
  await page.getByRole("tab", { name: "r-roms" }).click();
  await expect(page.getByText("PSP Redump .iso (Part 1)")).toBeVisible();
  await page.getByRole("tab", { name: "NoPayStation" }).click();
  await expect(page.getByText("LocoRoco 2").first()).toBeVisible();
  await expect(page.getByText("UCUS98662")).toBeVisible();
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
  await expect(page.locator(".queue-card .job-row")).toHaveCount(2);
});

test("settings exposes config controls", async ({ page }) => {
  await page.getByRole("button", { name: "Settings" }).click();
  await expect(page.getByText("Music conversion")).toBeVisible();
  await expect(page.getByText("ISO to CSO compression level")).toBeVisible();
});

test("theme studio creates a pack", async ({ page }) => {
  await page.getByRole("button", { name: "Mods" }).click();
  await page.getByRole("tab", { name: "Themes" }).click();
  await expect(page.getByRole("heading", { name: "Theme Studio" })).toBeVisible();
  await page.getByLabel("Pack name").fill("Test Pack");
  await page.getByRole("button", { name: "Create", exact: true }).click();
  await expect(page.locator(".theme-card .title").filter({ hasText: "Test Pack" })).toBeVisible();
});
