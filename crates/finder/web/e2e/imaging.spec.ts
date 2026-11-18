// The Imaging page on the simulated console (031 US7). Oracles: the simulated
// imaging camera's RGGB preview (1236 × 822 superpixeled to 618 × 411) and
// DESIGN.md's #e00000 night ink, whose one held exception this page is.
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

async function takeFrame(page: Page) {
  await page.getByRole("button", { name: "Take imaging frame" }).click();
  const picture = page.getByRole("img", { name: /^Imaging frame / });
  await expect(picture).toBeVisible({ timeout: 15_000 });
  await expect.poll(() => picture.evaluate((img: HTMLImageElement) => img.naturalWidth)).toBe(618);
  await expect.poll(() => picture.evaluate((img: HTMLImageElement) => img.naturalHeight)).toBe(411);
  return picture;
}

test("a frame taken from the Imaging page appears as a 618 × 411 picture", async ({ page }) => {
  await openImaging(page);
  await page.getByRole("textbox", { name: "Exposure", exact: true }).fill("200");
  await page.getByRole("button", { name: "Save imaging settings" }).click();
  await takeFrame(page);
  await expect(page.getByRole("banner").getByRole("link", { name: "Imaging" })).toHaveAttribute("aria-current", "page");
});

test.describe("the held colour exception on a phone", () => {
  test.use({ viewport: { width: 412, height: 915 }, deviceScaleFactor: 2 });

  test("the page is red at night, and the picture alone shows colour while it is held", async ({ page }) => {
    await openImaging(page);
    const picture = await takeFrame(page);
    await page.evaluate(() => document.fonts.ready);
    // The day page is the negative control for this same pixel scanner.
    expect(await offRed(page, await page.screenshot({ fullPage: true }))).toBeGreaterThan(100_000);
    await enableNight(page);
    expect(await offRed(page, await page.screenshot({ fullPage: true, animations: "disabled" }))).toBe(0);

    // Holding the control removes the red-only filter from that one picture.
    const hold = page.getByRole("button", { name: "Hold to see colour" });
    await hold.scrollIntoViewIfNeeded();
    const box = (await hold.boundingBox())!;
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.mouse.down();
    expect(await offRed(page, await picture.screenshot())).toBeGreaterThan(100);
    // A mask paints magenta by default, which the scanner would count: black it.
    expect(await offRed(page, await page.screenshot({ fullPage: true, mask: [picture], maskColor: "#000000" }))).toBe(
      0,
    );
    await page.mouse.up();
    expect(await offRed(page, await page.screenshot({ fullPage: true }))).toBe(0);
  });
});
