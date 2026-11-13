import { expect, test, type Locator, type Page } from "@playwright/test";

test.use({ viewport: { width: 1440, height: 900 } });

async function open(page: Page, night: boolean) {
  await page.addInitScript((night) => {
    localStorage.setItem("finder.night", JSON.stringify(night));
    localStorage.setItem("finder.frame", JSON.stringify("orion2.jpg"));
    localStorage.setItem("finder.site", JSON.stringify({ lat: 40, lon: -75 }));
  }, night);
  await page.goto("/");
  await expect(page.getByText(/Solved in/)).toBeVisible({ timeout: 30_000 });
  await page.evaluate(() => document.fonts.ready);
}

/** WCAG relative luminance, against black (luminance zero); independent of the page. */
function contrastOnBlack(rgb: string): number {
  const channels = /^rgb\((\d+), (\d+), (\d+)\)$/.exec(rgb);
  if (!channels) throw new Error(`Expected an opaque rgb() colour, got ${rgb}`);
  const [r, g, b] = channels.slice(1).map((channel) => {
    const s = Number(channel) / 255;
    return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  });
  return (0.2126 * r! + 0.7152 * g! + 0.0722 * b! + 0.05) / 0.05;
}

/** Spec § Clarifications: SVG user units scale with the viewBox, fixed strokes do not. */
async function onScreen(locator: Locator, property: "strokeWidth" | "fontSize"): Promise<number> {
  return locator.evaluate((element, property) => {
    const svg = (element as SVGElement).ownerSVGElement!;
    const style = getComputedStyle(element);
    const scale =
      property === "strokeWidth" && style.vectorEffect === "non-scaling-stroke"
        ? 1
        : svg.getBoundingClientRect().width / svg.viewBox.baseVal.width;
    return parseFloat(style[property]) * scale;
  }, property);
}

test("US1-S1: night secondary ink clears 3.5 on black while the old secondary red fails", async ({ page }) => {
  // Exact tokens are the approved mock-up's oracle; only night overrides them.
  const tokens = ["ink", "ink-2", "ink-3", "rule", "on-photo-2", "grid"];
  for (const night of [false, true]) {
    await open(page, night);
    const values = await page.locator("html").evaluate((root, tokens) => {
      const style = getComputedStyle(root);
      // The build may omit leading zeroes and spaces around a slash; preserve values.
      return tokens.map((token) =>
        style
          .getPropertyValue(`--color-${token}`)
          .trim()
          .replace(/(?<!\d)\.(\d+)/g, "0.$1")
          .replace(/\s*\/\s*/g, " / "),
      );
    }, tokens);
    expect(values).toEqual(
      night
        ? [
            "#e00000",
            "#d40000",
            "#a80000",
            "#5a0000",
            "#c00000",
            // Minification can encode 45% alpha as round(0.45 * 255) = 115 = 0x73.
            expect.stringMatching(/^(?:rgb\(168 0 0 \/ 0\.45\)|#a8000073)$/),
          ]
        : [
            "oklch(19% 0.02 262)",
            "oklch(41% 0.02 262)",
            "oklch(60% 0.015 262)",
            "oklch(86.5% 0.008 262)",
            "oklch(80% 0.015 95)",
            "oklch(84% 0.03 250 / 0.42)",
          ],
    );
    const quiet = page.locator(".quiet").first();
    await expect(quiet).toHaveCSS("color", night ? "rgb(212, 0, 0)" : "oklch(0.41 0.02 262)");
    if (night) {
      await expect(page.locator("body")).toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
      await expect(page.locator("html")).toHaveCSS("background-color", "rgb(0, 0, 0)");
      const colour = await quiet.evaluate((element) => getComputedStyle(element).color);
      expect(contrastOnBlack(colour)).toBeGreaterThanOrEqual(3.5);
    }
  }
  // Negative control: the old red is readable at the old floor, but fails the new one.
  expect(contrastOnBlack("rgb(168, 0, 0)")).toBeGreaterThanOrEqual(2.5);
  expect(contrastOnBlack("rgb(168, 0, 0)")).toBeLessThan(3.5);
});

test("US1-S2: night body is heavier and small text is one step larger than day", async ({ page }) => {
  for (const night of [false, true]) {
    await open(page, night);
    await expect(page.locator("body")).toHaveCSS("font-weight", night ? "540" : "400");
    await expect(page.locator(".text-sm").first()).toHaveCSS("font-size", night ? "16px" : "14px");
    await expect(page.locator(".text-xs").first()).toHaveCSS("font-size", night ? "14px" : "12px");
  }
});

test("US1-S3: day chart retains its thin rings, dashed unseen stars and 11 px edge labels", async ({ page }) => {
  await open(page, false);
  const chart = page.getByRole("img", { name: /^Chart of/ });
  const matched = chart.locator(".mark-matched circle.ring").first();
  const unseen = chart.locator(".mark-expected circle.ring").first();
  const edge = chart.locator(".edge-label").first();

  // Approved day baseline, with screen widths rounded to 0.1 px and text to 1 px.
  expect(Number((await onScreen(matched, "strokeWidth")).toFixed(1))).toBe(1.3);
  expect(Number((await onScreen(unseen, "strokeWidth")).toFixed(1))).toBe(1.3);
  await expect(matched).toHaveCSS("vector-effect", "none");
  await expect(unseen).not.toHaveCSS("stroke-dasharray", "none");
  await expect(unseen).toHaveCSS("opacity", "0.7");
  expect(Math.round(await onScreen(edge, "fontSize"))).toBe(11);
  await expect(edge).toHaveCSS("fill", "oklch(0.41 0.02 262)");
  await expect(edge).toHaveCSS("font-weight", "520");
  await expect(chart.locator(".mark-label").first()).toHaveCSS("font-weight", "560");
  await expect(chart.locator(".object-label").first()).toHaveCSS("font-weight", "620");
  await expect(chart.locator(".grid-line").first()).toHaveCSS("stroke", "oklch(0.84 0.03 250 / 0.42)");
  await expect(chart.locator(".furniture")).toHaveCSS("vector-effect", "none");

  await chart.locator(".mark-matched .hit").first().click();
  await expect(chart.locator('[data-picked="true"] circle.ring')).toHaveCSS("stroke-width", "2.4px");
});

test("US1-S4: night chart puts fixed-width star rings and brighter labels ahead of the grid", async ({ page }) => {
  await open(page, true);
  await page.waitForTimeout(1500); // The chart inks in before its styles are measured.
  const chart = page.getByRole("img", { name: /^Chart of/ });
  const matched = chart.locator(".mark-matched circle.ring").first();
  const unseen = chart.locator(".mark-expected circle.ring").first();
  const grid = chart.locator(".grid-line").first();
  const edge = chart.locator(".edge-label").first();

  // Approved night mock-up: screen widths to 0.1 px, edge text to the nearest pixel.
  expect(Number((await onScreen(matched, "strokeWidth")).toFixed(1))).toBe(2.2);
  expect(Number((await onScreen(unseen, "strokeWidth")).toFixed(1))).toBe(1.2);
  await expect(matched).toHaveCSS("vector-effect", "non-scaling-stroke");
  await expect(matched).toHaveCSS("opacity", "1");
  await expect(unseen).toHaveCSS("vector-effect", "non-scaling-stroke");
  await expect(unseen).toHaveCSS("stroke-dasharray", "none");
  await expect(unseen).toHaveCSS("opacity", "0.55");
  expect(await onScreen(grid, "strokeWidth")).toBeLessThanOrEqual(1.05);
  await expect(grid).toHaveCSS("stroke", "rgba(168, 0, 0, 0.45)");
  await expect(grid).toHaveCSS("vector-effect", "none");
  expect(Math.round(await onScreen(edge, "fontSize"))).toBe(14);
  await expect(edge).toHaveCSS("fill", "rgb(224, 0, 0)");
  await expect(edge).toHaveCSS("font-weight", "640");
  await expect(chart.locator(".mark-label").first()).toHaveCSS("font-weight", "680");
  await expect(chart.locator(".object-label").first()).toHaveCSS("font-weight", "680");
  await expect(chart.locator(".furniture")).toHaveCSS("stroke-width", "1.8px");
  await expect(chart.locator(".furniture")).toHaveCSS("vector-effect", "non-scaling-stroke");

  await chart.locator(".mark-matched .hit").first().click();
  const picked = chart.locator('[data-picked="true"] circle.ring');
  expect(Number((await onScreen(picked, "strokeWidth")).toFixed(1))).toBe(3.2);
});
