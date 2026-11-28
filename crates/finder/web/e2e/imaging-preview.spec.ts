// Preview frames on a phone, on the simulated console (058 US4-S6). Oracles: the simulated
// imaging camera's RGGB preview (1236 × 822 superpixeled to 618 × 411), the 200 × 200 crop
// the spec fixes, and DESIGN.md's #e00000 night ink, counted as imaging.spec.ts counts it.
import { expect, test, type Page } from "@playwright/test";

/** Any green or blue is forbidden; red may exceed ink (224) by only two rounding levels. */
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

async function enableNight(page: Page) {
  const toggle = page.getByRole("switch", { name: "Night mode: red and black" });
  await toggle.click();
  await expect(toggle).toBeChecked();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "night");
}

/** Open the Imaging page by day and wait for the camera it relays. */
async function openImaging(page: Page) {
  await page.addInitScript(() => localStorage.setItem("finder.night", "false"));
  await page.goto("/imaging");
  await expect(page.getByRole("region", { name: "Camera" })).toContainText("simulated imaging camera");
}

type ImagingView = { frame?: { id: number; state: string; purpose: string; file?: string } };

async function readImaging(page: Page): Promise<ImagingView> {
  const response = await page.request.get("/api/imaging");
  expect(response.ok()).toBe(true);
  return (await response.json()) as ImagingView;
}

/** The frame number in the block's status, or NaN while it shows none. */
const frameNumber = (text: string | null) => Number(/Preview frame (\d+)/.exec(text ?? "")?.[1] ?? Number.NaN);

test.describe("preview frames on a phone", () => {
  test.use({ viewport: { width: 412, height: 915 }, deviceScaleFactor: 2 });

  test("the block shows frames and their crop, saves none, and fits the phone by day and by night", async ({
    page,
  }) => {
    try {
      await openImaging(page);
      const settings = page.getByRole("region", { name: "Settings" });
      await settings.getByRole("textbox", { name: "Exposure", exact: true }).fill("200");
      await settings.getByRole("button", { name: "Save imaging settings" }).click();

      const preview = page.getByRole("region", { name: "Preview frames" });
      await preview.getByRole("button", { name: "Start preview frames" }).click();

      const crop = page.getByRole("img", { name: /^Crop of preview frame \d+ around its brightest part$/ });
      await expect(crop).toBeVisible({ timeout: 15_000 });
      const first = frameNumber(await preview.getByRole("status").textContent());
      await expect
        .poll(async () => crop.evaluate((img: HTMLImageElement) => [img.naturalWidth, img.naturalHeight]), {
          timeout: 15_000,
        })
        .toEqual([200, 200]);

      await expect
        .poll(async () => frameNumber(await preview.getByRole("status").textContent()), { timeout: 20_000 })
        .toBeGreaterThanOrEqual(first + 3);
      await expect(preview.getByRole("status")).toHaveText(/^Preview frame \d+ · every \d+\.\d s · not saved$/);

      const seen = await readImaging(page);
      expect(seen.frame?.purpose).toBe("preview");
      expect(seen.frame?.file).toBeUndefined();

      const day = await page.screenshot({ fullPage: true });
      expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(412);
      expect(await offRed(page, day)).toBeGreaterThan(10_000);

      await enableNight(page);
      const night = await page.screenshot({ fullPage: true, animations: "disabled" });
      expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(412);
      expect(await offRed(page, night)).toBe(0);

      await preview.getByRole("button", { name: "Stop preview frames" }).click();
      await expect(preview.getByRole("status")).toHaveCount(0, { timeout: 15_000 });
      await expect(preview.getByRole("button", { name: "Start preview frames" })).toBeVisible();

      // Read only once the block has stopped: while frames run, the section's picture comes and goes.
      const picture = page
        .getByRole("region", { name: "Imaging frame" })
        .getByRole("img", { name: /^Imaging frame \d+$/ });
      await expect
        .poll(() => picture.evaluate((img: HTMLImageElement) => img.naturalWidth), { timeout: 15_000 })
        .toBe(618);

      const stopped = await readImaging(page);
      expect(stopped.frame?.purpose).toBe("preview");
    } finally {
      // Leave the page, then wait until the console holds no frame in flight.
      await page.goto("about:blank");
      await expect
        .poll(async () => (await readImaging(page)).frame?.state ?? "none", { timeout: 15_000 })
        .not.toMatch(/^(exposing|receiving)$/);
      const idAfterQuiet = (await readImaging(page)).frame?.id;
      await page.waitForTimeout(1_500);
      expect((await readImaging(page)).frame?.id).toBe(idAfterQuiet);
    }
  });
});
