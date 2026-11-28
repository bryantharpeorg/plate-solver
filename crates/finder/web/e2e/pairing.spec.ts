import { expect, test } from "@playwright/test";

test.use({ viewport: { width: 412, height: 915 }, deviceScaleFactor: 2 });

test("US3-S6: the pairing prompt keeps every computed colour red or black", async ({ page }) => {
  // Oracle: DESIGN.md's night tokens, exact RGB channels and 44 CSS px targets.
  const response = await page.goto("/pair");
  expect(response?.status()).toBe(200);
  const heading = page.getByRole("heading", { name: "Pair with the finder console", exact: true });
  await expect(heading).toBeVisible();
  const field = page.getByLabel("Console token", { exact: true });
  const pair = page.getByRole("button", { name: "Pair", exact: true });
  await field.focus();
  const colours = await page.evaluate(() => {
    const properties = [
      "color",
      "background-color",
      "border-top-color",
      "border-bottom-color",
      "outline-color",
      "caret-color",
    ];
    return [document.documentElement, ...document.querySelectorAll("body, body *")]
      .flatMap((element) => {
        const style = getComputedStyle(element);
        return properties.map((property) => style.getPropertyValue(property));
      })
      .filter((colour) => colour !== "rgba(0, 0, 0, 0)");
  });
  expect(colours.length).toBeGreaterThan(10);
  for (const colour of colours) {
    expect(colour).toMatch(/^rgba?\((\d+), 0, 0(?:, [\d.]+)?\)$/);
  }
  await expect(page.locator("html")).toHaveCSS("background-color", "rgb(0, 0, 0)");
  await expect(field).toHaveCSS("outline-color", "rgb(224, 0, 0)");
  expect((await field.boundingBox())?.height).toBeGreaterThanOrEqual(44);
  expect((await pair.boundingBox())?.height).toBeGreaterThanOrEqual(44);
  await field.fill("NOPE-NOPE-NOPE");
  await pair.click();
  await expect(heading).toBeVisible();
  await expect(page.getByRole("status")).toHaveText("That is not the console token.");
});
