// US4-S6. Oracles: the simulated aid's median of three, phone width, and
// DESIGN.md's red-only night pixels, with the same count as tools.spec.ts.
import { expect, test, type Page } from "@playwright/test";

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

test("Tools focuses either camera within a phone's width in day and night", async ({ page }) => {
  try {
    const settings = await page.request.put("/api/session", {
      data: { camera_settings: { exposure_ms: 200, gain: 1 } },
    });
    expect(settings.status(), await settings.text()).toBe(200);
    await page.addInitScript(() => localStorage.setItem("finder.night", "false"));
    await page.goto("/tools");
    await expect(page.getByRole("heading", { name: "Tools", level: 1 })).toBeVisible();
    const focus = page.getByRole("region", { name: "Focus a camera", exact: true });
    await focus.getByRole("button", { name: "Start the finder camera's focus aid" }).click({ timeout: 15_000 });
    await expect(focus.getByRole("status")).toContainText("Median of the last 3 finder camera frames", {
      timeout: 20_000,
    });
    await expect(focus.getByText(/^\d+\.\d{2} px$/)).toBeVisible();
    await expect(focus.getByRole("img", { name: /Focus readings of the last \d+ finder camera frames/ })).toBeVisible();
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
    await test.info().attach("tools-focus-day", { body: day, contentType: "image/png" });
    await test.info().attach("tools-focus-night", { body: red, contentType: "image/png" });
    await focus.getByRole("combobox", { name: "Camera" }).selectOption("imaging");
    await expect(focus.getByRole("button", { name: "Start focus aid", exact: true })).toBeVisible({ timeout: 15_000 });
    await focus.getByRole("combobox", { name: "Camera" }).selectOption("finder");
    await focus.getByRole("button", { name: "Stop the finder camera's focus aid" }).click({ timeout: 15_000 });
    await expect(focus.getByRole("button", { name: "Start the finder camera's focus aid" })).toBeVisible({
      timeout: 15_000,
    });
  } finally {
    const stop = await page.request.delete("/api/finder-camera/focus");
    expect(stop.status(), await stop.text()).toBe(200);
  }
});
