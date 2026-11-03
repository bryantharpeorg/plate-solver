// Real finder --simulate, session responses and rendered pixels. Oracles: the
// session's unrounded readout and DESIGN.md's #e00000 night ink. No responses or
// live solves are replaced. Earlier end-to-end files remain unchanged.
import { expect, test, type APIRequestContext, type Page } from "@playwright/test";
import type { ObservingSession } from "../src/api";

test.describe.configure({ mode: "serial" });
test.use({ viewport: { width: 412, height: 915 }, deviceScaleFactor: 2 });

async function session(request: APIRequestContext): Promise<ObservingSession> {
  const response = await request.get("/api/session");
  expect(response.ok()).toBe(true);
  return response.json();
}

async function post(request: APIRequestContext, path: string, data?: unknown) {
  const response = await request.post(path, { data });
  expect(response.ok(), await response.text()).toBe(true);
}

// 012's offRed helper, kept local because importing a spec would register its
// tests again. Zero green/blue; ink red (224) plus two rounding levels at most.
async function offRed(page: Page, png: Buffer): Promise<number> {
  return page.evaluate(async (data) => {
    const img = new Image();
    img.src = `data:image/png;base64,${data}`;
    await img.decode();
    const canvas = document.createElement("canvas");
    canvas.width = img.width;
    canvas.height = img.height;
    const g = canvas.getContext("2d") as CanvasRenderingContext2D;
    g.drawImage(img, 0, 0);
    const d = g.getImageData(0, 0, canvas.width, canvas.height).data;
    let off = 0;
    for (let i = 0; i < d.length; i += 4) {
      if ((d[i] ?? 0) > 226 || (d[i + 1] ?? 0) > 0 || (d[i + 2] ?? 0) > 0) off++;
    }
    return off;
  }, png.toString("base64"));
}

test.beforeAll(async ({ request }) => {
  await post(request, "/api/track", { on: false });
  const response = await request.put("/api/session", {
    data: {
      site: { lat_deg: 40, lon_deg: -75 },
      imaging_camera: { width_mm: 23.5, height_mm: 15.6, pitch_um: 3.76 },
      target: "M42",
      min_alt_deg: 15,
      camera_settings: { exposure_ms: 200, gain: 1 },
    },
  });
  expect(response.ok()).toBe(true);
  await post(request, "/api/sim/telescope", { alt_deg: 50, az_deg: 120 });
  await post(request, "/api/model/reset", { what: "night" });
  await post(request, "/api/sim/frames", { running: true });
  await expect.poll(async () => (await session(request)).model?.syncs, { timeout: 15_000 }).toBeGreaterThan(0);
});

test.beforeEach(async ({ page, request }) => {
  test.setTimeout(150_000);
  await page.addInitScript(() => localStorage.setItem("finder.night", "false"));
  await page.goto("/");
  await page.evaluate(() => document.fonts.ready);
  const block = page.getByRole("region", { name: /^Target M42/ });
  const toggle = block.getByRole("switch", { name: "Track", exact: true });
  await expect(toggle).toBeVisible();
  await expect(toggle).not.toBeChecked();
  await toggle.click();
  await expect(toggle).toBeChecked();
  await expect.poll(async () => (await session(request)).track?.state, { timeout: 120_000 }).toBe("tracking");
  await expect(block.getByRole("region", { name: "Track", exact: true }).getByRole("status")).toHaveText(
    "Tracking M42",
  );
  const read = await session(request);
  expect(read.mount?.connected).toBe(true);
  expect(read.mount?.moving).toBe(true);
  expect(read.track?.wanted).toBe(true);
  expect(read.track?.target?.messier).toBe("M42");
  expect(read.imaging_camera).toEqual({ width_mm: 23.5, height_mm: 15.6, pitch_um: 3.76 });
});

test.afterEach(async ({ request }) => {
  await post(request, "/api/track", { on: false });
  await expect.poll(async () => (await session(request)).track?.state).toBe("off");
  await expect.poll(async () => (await session(request)).mount?.moving).toBe(false);
});

test.afterAll(async ({ request }) => {
  await post(request, "/api/track", { on: false });
  const response = await request.put("/api/session", { data: { imaging_camera: null, target: null } });
  expect(response.ok()).toBe(true);
});

test("Tracking M42: guidance rounds the session error, field rotation and longest exposure", async ({ page }) => {
  // Read an actual poll received by this page, so the three expectations come
  // from the same session snapshot that React displays, even while it tracks.
  const response = await page.waitForResponse(async (response) => {
    if (!response.url().endsWith("/api/session") || response.request().method() !== "GET" || !response.ok()) {
      return false;
    }
    const read = (await response.json()) as ObservingSession;
    return read.track?.state === "tracking" && read.track.readout?.longest_exposure_s !== undefined;
  });
  const read = (await response.json()) as ObservingSession;
  const readout = read.track!.readout!;
  expect(Number.isFinite(readout.rms_arcsec)).toBe(true);
  expect(Number.isFinite(readout.field_rotation_deg_per_min)).toBe(true);
  expect(typeof readout.longest_exposure_s).toBe("number"); // M42 has nonzero rotation here.
  const exposure = readout.longest_exposure_s as number;
  expect(exposure).toBeGreaterThan(0);
  const block = page.getByRole("region", { name: /^Target M42/ }).getByRole("region", { name: "Track", exact: true });
  const value = (label: string) => block.getByText(label, { exact: true }).locator("..").locator("dd");
  await expect(value("Tracking error (RMS, last minute)")).toHaveText(`${Math.round(readout.rms_arcsec)}″`);
  await expect(value("Field rotation")).toHaveText(`${readout.field_rotation_deg_per_min.toFixed(2)}°/min`);
  await expect(value("Longest exposure (1 px smear)")).toHaveText(`${exposure.toFixed(1)} s`);
});

test("M42 tracked in night mode has no off-red pixel", async ({ page, request }) => {
  const block = page.getByRole("region", { name: /^Target M42/ }).getByRole("region", { name: "Track", exact: true });
  await expect(block.getByText("Longest exposure (1 px smear)", { exact: true })).toBeVisible();
  await expect(page.getByRole("img", { name: /^Chart of Live frame / })).toBeVisible();
  // Negative control: this exact scanner must reject the day edition while tracking.
  expect(await offRed(page, await page.screenshot({ fullPage: true }))).toBeGreaterThan(100_000);
  const night = page.getByRole("switch", { name: "Night mode: red and black" });
  await night.click();
  await expect(night).toBeChecked();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "night");
  await page.waitForTimeout(1500); // 012's allowance for the chart's ink-in animation.
  await expect(block.getByRole("switch", { name: "Track", exact: true })).toBeChecked();
  expect((await session(request)).track?.state).toBe("tracking");
  const png = await page.screenshot({ fullPage: true, path: test.info().outputPath("tracking-night.png") });
  expect(await offRed(page, png)).toBe(0);
  expect((await session(request)).track?.state).toBe("tracking");
});
