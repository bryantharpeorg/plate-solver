// US9-S7. Oracle: DESIGN.md's Night Ink and the Red Torch Rule's held-picture
// exception. The scanner is session.spec.ts's: zero off-red pixels outside the
// held picture, with >100 in that picture as the scanner's negative control.
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

test("the stack's picture alone can show colour at night while its control is held", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("finder.night", "false"));
  await page.goto("/imaging");
  const region = page.getByRole("region", { name: "Stack", exact: true });
  await region.getByRole("button", { name: "Start stacking" }).click();
  try {
    const picture = region.getByRole("img", { name: /^Stack of / });
    await expect(picture).toBeVisible({ timeout: 30_000 });
    await expect.poll(() => picture.evaluate((img: HTMLImageElement) => img.naturalWidth)).toBeGreaterThan(0);
    await page.evaluate(() => document.fonts.ready);
    const night = page.getByRole("switch", { name: "Night mode: red and black" });
    await night.click();
    await expect(night).toBeChecked();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "night");
    expect(await offRed(page, await page.screenshot({ fullPage: true, animations: "disabled" }))).toBe(0);

    const hold = region.getByRole("button", { name: "Hold to see colour" });
    await hold.scrollIntoViewIfNeeded();
    const box = (await hold.boundingBox())!;
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.mouse.down();
    try {
      await expect(hold).toHaveAttribute("aria-pressed", "true");
      expect(await offRed(page, await picture.screenshot())).toBeGreaterThan(100);
      expect(await offRed(page, await page.screenshot({ fullPage: true, mask: [picture], maskColor: "#000000" }))).toBe(
        0,
      );
    } finally {
      await page.mouse.up();
    }
    await expect(hold).toHaveAttribute("aria-pressed", "false");
    expect(await offRed(page, await page.screenshot({ fullPage: true }))).toBe(0);
  } finally {
    await region.getByRole("button", { name: "Stop stacking" }).click();
    await expect(region.getByRole("status")).toHaveText(/^Stack /, { timeout: 30_000 });
  }
});
