// US5-S7: real simulated console, on a phone. Oracles: the two-frame Done
// contract, the viewport width and DESIGN.md's red-only night ink.
import { expect, test, type Page } from "@playwright/test";
import type { ImagingView } from "../src/api";

// Kept local: importing imaging.spec.ts would register its tests a second time.
async function offRed(page: Page, png: Buffer): Promise<number> {
  return page.evaluate(async (data) => {
    const image = new Image();
    image.src = `data:image/png;base64,${data}`;
    await image.decode();
    const canvas = document.createElement("canvas");
    canvas.width = image.width;
    canvas.height = image.height;
    const context = canvas.getContext("2d")!;
    context.drawImage(image, 0, 0);
    const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
    let off = 0;
    for (let i = 0; i < pixels.length; i += 4) {
      if (pixels[i]! > 226 || pixels[i + 1]! > 0 || pixels[i + 2]! > 0) off++;
    }
    return off;
  }, png.toString("base64"));
}

test.use({ viewport: { width: 412, height: 915 }, deviceScaleFactor: 2 });

test("Tools aligns the finder camera and stays within a phone's width in day and night", async ({ page, request }) => {
  const track = await request.post("/api/track", { data: { on: false } });
  expect(track.ok(), await track.text()).toBe(true);
  const imaging = async (): Promise<ImagingView> => {
    const response = await request.get("/api/imaging");
    expect(response.ok()).toBe(true);
    return response.json();
  };
  await expect.poll(async () => (await imaging()).state, { timeout: 10_000 }).toBe("ready");
  // A solved imaging frame keeps arcseconds per sensor pixel, as 032 defines
  // scale, so alignment has its field height without changing the server flags.
  const settings = await request.put("/api/imaging/settings", { data: { exposure_ms: 500 } });
  expect(settings.ok(), await settings.text()).toBe(true);
  const take = await request.post("/api/imaging/frame");
  expect(take.ok(), await take.text()).toBe(true);
  await expect.poll(async () => (await imaging()).frame?.state, { timeout: 10_000 }).toBe("saved");
  const solve = await request.post("/api/imaging/solve");
  expect(solve.ok(), await solve.text()).toBe(true);
  await expect.poll(async () => (await imaging()).solve?.state, { timeout: 10_000 }).toBe("solved");
  expect((await imaging()).relation?.scale?.arcsec_per_px).toBeGreaterThan(0);

  await page.addInitScript(() => localStorage.setItem("finder.night", "false"));
  await page.goto("/setup");
  await page.getByRole("link", { name: "Tools", exact: true }).click();
  await expect(page).toHaveURL(/\/tools$/);
  await expect(page.getByRole("heading", { name: "Tools", level: 1 })).toBeVisible();
  const alignment = page.getByRole("region", { name: "Align the finder camera", exact: true });
  await alignment.getByRole("button", { name: "Start aligning" }).click();
  try {
    await expect(alignment.getByRole("status")).toContainText(
      "Done: the imaging camera's centre has held in the middle",
      { timeout: 20_000 },
    );
    await expect(
      alignment.getByRole("img", { name: "The imaging camera's centre on the finder camera's frame" }),
    ).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(412);
    await page.evaluate(() => document.fonts.ready);
    const day = await page.screenshot({ fullPage: true });
    expect(await offRed(page, day)).toBeGreaterThan(10_000);
    const night = page.getByRole("switch", { name: "Night mode: red and black" });
    await night.click();
    await expect(night).toBeChecked();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "night");
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(412);
    const red = await page.screenshot({ fullPage: true, animations: "disabled" });
    expect(await offRed(page, red)).toBe(0);
    await test.info().attach("tools-day", { body: day, contentType: "image/png" });
    await test.info().attach("tools-night", { body: red, contentType: "image/png" });
  } finally {
    await alignment.getByRole("button", { name: "Stop aligning" }).click();
    await expect(alignment.getByRole("button", { name: "Start aligning" })).toBeVisible({ timeout: 15_000 });
  }
});
