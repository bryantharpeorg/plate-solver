// Oracles: real simulator directions, its (700, 420) eyepiece pixel, the
// rendered chart's plate scale, and DESIGN.md's #e00000 night ink. No responses
// or EventSource messages are replaced. One telescope serves this serial night.
import { expect, test as base, type APIRequestContext, type Page } from "@playwright/test";
import type { LiveReport, ObservingSession } from "../src/api";

const test = base.extend<{ frames: LiveReport[] }>({
  frames: async ({ page }, use) => {
    const frames: LiveReport[] = [];
    const network = await page.context().newCDPSession(page);
    await network.send("Network.enable");
    network.on("Network.eventSourceMessageReceived", ({ eventName, data }) => {
      if (eventName === "live") frames.push(JSON.parse(data) as LiveReport);
    });
    await use(frames);
    await network.detach();
  },
});
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

async function truth(
  request: APIRequestContext,
): Promise<{ eyepiece: { alt: number; az: number; ra: number; dec: number } }> {
  const response = await request.get("/api/sim/telescope");
  expect(response.ok()).toBe(true);
  return response.json();
}

// Chord distance and 2 asin(d/2), including RA wrap and polar directions.
function arcseconds(a: { ra: number; dec: number }, b: { ra: number; dec: number }) {
  const vector = ({ ra, dec }: typeof a) => {
    const r = (ra * Math.PI) / 180;
    const d = (dec * Math.PI) / 180;
    return [Math.cos(d) * Math.cos(r), Math.cos(d) * Math.sin(r), Math.sin(d)];
  };
  const av = vector(a);
  const bv = vector(b);
  return (2 * Math.asin(Math.min(1, Math.hypot(...av.map((v, i) => v - bv[i]!)) / 2)) * 180 * 3600) / Math.PI;
}

async function open(page: Page, path = "/") {
  await page.addInitScript(() => localStorage.setItem("finder.night", "false"));
  await page.goto(path);
  await page.evaluate(() => document.fonts.ready);
}

async function choose(page: Page, name: string) {
  await page.getByRole("button", { name: "Choose target", exact: true }).click();
  const sheet = page.getByRole("dialog", { name: "Choose a target" });
  await sheet.getByRole("searchbox", { name: "Search catalog" }).fill(name);
  await sheet
    .getByRole("list", { name: "Matching targets" })
    .getByRole("button", { name: new RegExp(`\\(${name}\\)`) })
    .click();
  await expect(sheet).not.toBeVisible();
  await expect(page.getByRole("region", { name: new RegExp(`^Target ${name}`) })).toBeVisible();
}

async function exposedStop(page: Page) {
  const stop = page.getByRole("button", { name: "Stop", exact: true });
  await expect(stop).toBeVisible();
  await expect(stop).toBeEnabled();
  await stop.scrollIntoViewIfNeeded();
  const box = await stop.boundingBox();
  expect(box).not.toBeNull();
  expect(box!.height).toBeGreaterThanOrEqual(44);
  expect(box!.width).toBeGreaterThan(0);
  expect(
    await stop.evaluate((button) => {
      const b = button.getBoundingClientRect();
      return document.elementFromPoint(b.x + b.width / 2, b.y + b.height / 2) === button;
    }),
  ).toBe(true);
  return stop;
}

/** Zero green/blue; at most ink red (224) plus two compositing rounding levels. */
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

test.afterAll(async ({ request }) => {
  await post(request, "/api/stop");
  await post(request, "/api/sim/frames", { running: true });
  // The older simulate.spec.ts starts without an aim point. Restore that
  // precondition rather than changing its calibration or any of its assertions.
  expect((await request.delete("/api/aim")).ok()).toBe(true);
  await post(request, "/api/model/reset", { what: "night" });
});

test("one sync on Setup: Second sync slews, solves, and displays the base tilt", async ({ page, request, frames }) => {
  test.setTimeout(120_000);
  expect(
    (
      await request.put("/api/session", {
        data: {
          site: { lat_deg: 40, lon_deg: -75 },
          target: "M42",
          camera_settings: { exposure_ms: 200, gain: 1 },
        },
      })
    ).ok(),
  ).toBe(true);
  await post(request, "/api/sim/telescope", { alt_deg: 50, az_deg: 120 });
  await open(page, "/setup");
  const preview = page.getByRole("img", { name: /^Live preview, frame / });
  await expect(preview).toBeVisible();
  await post(request, "/api/sim/frames", { running: false });
  await page.waitForTimeout(6000); // Drain the exposure and the 5 s preview cadence.
  await post(request, "/api/sim/telescope", { eyepiece_on_star: true });
  const centredAt = (await session(request)).time!;
  await post(request, "/api/sim/frames", { running: true });
  await expect
    .poll(() => frames.find((f) => f.time - 0.1 > centredAt && f.preview_seq === f.frame_id), {
      timeout: 3000,
      intervals: [25],
    })
    .toBeDefined();
  const frame = frames.find((f) => f.time - 0.1 > centredAt && f.preview_seq === f.frame_id)!;
  await expect(preview).toHaveAttribute("src", `/api/live/preview.jpg?seq=${frame.frame_id}`);
  expect((await request.post("/api/aim", { data: { frame_id: frame.frame_id, x: 700, y: 420 } })).ok()).toBe(true);
  const aim = (await session(request)).aim!;
  expect(Math.abs(aim.x - 700)).toBeLessThanOrEqual(0.5);
  expect(Math.abs(aim.y - 420)).toBeLessThanOrEqual(0.5);
  await post(request, "/api/model/reset", { what: "night" });
  await expect.poll(async () => (await session(request)).model?.syncs).toBe(1);
  expect((await session(request)).model?.base_tilt).toBeUndefined();
  const model = page.getByRole("region", { name: "Mount and pointing model" });
  await expect(model.getByText("1 sync", { exact: true })).toBeVisible();
  const before = (await truth(request)).eyepiece;
  await model.getByRole("button", { name: "Second sync", exact: true }).click();
  await expect.poll(async () => (await session(request)).mount?.moving).toBe(true);
  await exposedStop(page); // Setup's top bar during the second-sync slew.
  await expect.poll(async () => (await session(request)).goto?.state, { timeout: 90_000 }).toBe("done");
  const after = await session(request);
  expect(after.model!.syncs).toBeGreaterThanOrEqual(2);
  expect(after.goto!.correction).toBe(0);
  expect(after.mount!.moving).toBe(false);
  const moved = (await truth(request)).eyepiece;
  expect(arcseconds({ ra: before.az, dec: before.alt }, { ra: moved.az, dec: moved.alt })).toBeGreaterThan(3600);
  await expect(model.getByText(/^base tilt \d+\.\d° toward \d+°$/)).toBeVisible();
});

test("M42 GoTo shows every correction and on target; its running night view is red only", async ({
  page,
  request,
  frames,
}) => {
  test.setTimeout(150_000);
  await open(page);
  await choose(page, "M42");
  const block = page.getByRole("region", { name: /^Target M42/ });
  // Observe DOM changes so short lived phases cannot disappear between assertions.
  await block.evaluate((element) => {
    const history: string[] = [];
    (window as unknown as { gotoText: string[] }).gotoText = history;
    const record = () => history.push(element.querySelector('[role="status"]')?.textContent ?? "");
    new MutationObserver(record).observe(element, { subtree: true, childList: true, characterData: true });
    record();
  });
  const initial = await session(request);
  expect(
    arcseconds((await truth(request)).eyepiece, { ra: initial.target!.ra_deg, dec: initial.target!.dec_deg }),
  ).toBeGreaterThan(3600);
  await block.getByRole("button", { name: "GoTo", exact: true }).click();
  await expect(block.getByRole("status")).toContainText("Slewing");
  await expect.poll(async () => (await session(request)).mount?.moving).toBe(true);
  await exposedStop(page); // Console's top bar while a GoTo runs.
  // Negative control: this same scanner rejects the daylight screenshot.
  expect(await offRed(page, await page.screenshot({ fullPage: true }))).toBeGreaterThan(100_000);
  const toggle = page.getByRole("switch", { name: "Night mode: red and black" });
  await toggle.click();
  await expect(toggle).toBeChecked();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "night");
  await page.waitForTimeout(1500); // Finish the page's ink-in animation.
  expect((await session(request)).goto!.state).toBe("slewing");
  const png = await page.screenshot({ fullPage: true, path: test.info().outputPath("goto-night.png") });
  expect((await session(request)).mount!.moving).toBe(true);
  expect(await offRed(page, png)).toBe(0);

  // A known physical push after the first slew forces a real correction; no
  // progress value is injected. The subsequent solve and GoTo must recover it.
  await expect
    .poll(async () => (await session(request)).goto?.state, { timeout: 90_000, intervals: [25] })
    .toBe("solving");
  await post(request, "/api/sim/telescope", { turn_deg: 0.15, tilt_deg: 0 });
  await expect(block.getByRole("status")).toContainText("On target", { timeout: 60_000 });
  const final = await session(request);
  expect(final.goto!.state).toBe("done");
  expect(final.goto!.error_arcsec!).toBeLessThan(60);
  expect(final.goto!.correction).toBeGreaterThan(0);
  expect(final.goto!.correction).toBeLessThanOrEqual(5);
  await expect(block.getByRole("status")).toContainText(`${final.goto!.error_arcsec!.toFixed(1)}″`);
  const history = await page.evaluate(() => (window as unknown as { gotoText: string[] }).gotoText);
  expect(history.some((s) => s.includes("Slewing"))).toBe(true);
  expect(history.some((s) => s.includes("Solving"))).toBe(true);
  for (let n = 1; n <= final.goto!.correction; n++) {
    expect(
      history.some((s) => s.startsWith(`Correction ${n} ·`)),
      history.join("\n"),
    ).toBe(true);
  }
  expect(
    arcseconds((await truth(request)).eyepiece, { ra: final.target!.ra_deg, dec: final.target!.dec_deg }),
  ).toBeLessThan(60);

  // The marker's actual SVG transform is in full-frame pixels. Compare it
  // with the calibrated eyepiece pixel, using the real live plate scale.
  const marker = page.locator('[data-target="M42"]:has(.target-reticle)');
  await expect(marker).toBeAttached();
  await expect
    .poll(
      async () => {
        const position = await marker.evaluate((g: SVGGraphicsElement) => {
          const m = g.transform.baseVal.consolidate()!.matrix;
          return { x: m.e, y: m.f };
        });
        const report = frames.at(-1)!.report;
        expect(report.pointing).not.toBeNull();
        return Math.hypot(position.x - 700, position.y - 420) * report.pointing!.arcsec_per_px;
      },
      { timeout: 5000, intervals: [50] },
    )
    .toBeLessThan(60);
});

test("Stop from the console and Setup halts the axes and reports the observer", async ({ page, request }) => {
  test.setTimeout(60_000);
  await open(page);
  for (const setup of [false, true]) {
    await choose(page, "M31");
    const block = page.getByRole("region", { name: /^Target M31/ });
    await block.getByRole("button", { name: "GoTo", exact: true }).click();
    await expect(block.getByRole("status")).toContainText("Slewing");
    await expect.poll(async () => (await session(request)).mount?.moving).toBe(true);
    const before = (await truth(request)).eyepiece;
    // A disconnected or inert telescope cannot satisfy the stop assertions.
    await expect
      .poll(async () => {
        const now = (await truth(request)).eyepiece;
        return arcseconds({ ra: before.az, dec: before.alt }, { ra: now.az, dec: now.alt });
      })
      .toBeGreaterThan(60);
    if (setup) await page.getByRole("link", { name: "Setup", exact: true }).click();
    await (await exposedStop(page)).click();
    await expect
      .poll(async () => (await session(request)).mount?.moving, { timeout: 3000, intervals: [50] })
      .toBe(false);
    if (setup) await page.getByRole("link", { name: "Back to console" }).click();
    await expect(block.getByRole("status")).toContainText("the observer stopped it");
    expect((await session(request)).goto!.state).toBe("halted");
    const stopped = (await truth(request)).eyepiece;
    await page.waitForTimeout(1500);
    const later = (await truth(request)).eyepiece;
    // Horizontal truth is fixed at rest (RA/Dec continue with the sky clock).
    expect(Math.abs(later.alt - stopped.alt)).toBeLessThan(1e-8);
    expect(Math.abs(later.az - stopped.az)).toBeLessThan(1e-8);
    expect((await session(request)).mount!.moving).toBe(false);
  }
});
