// Oracles: the simulator's independent true directions, its virtual UTC clock,
// the specified (700, 420) eyepiece pixel, and DESIGN.md's #e00000 night ink.
import { expect, test as base, type APIRequestContext, type Page } from "@playwright/test";
import type { LiveReport, ObservingSession } from "../src/api";

// Observe the real EventSource responses consumed by the page, without replacing
// EventSource, injecting reports, or opening an extra stream.
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

async function session(request: APIRequestContext): Promise<ObservingSession> {
  const response = await request.get("/api/session");
  expect(response.ok()).toBe(true);
  return response.json();
}

async function running(request: APIRequestContext, value: boolean) {
  const response = await request.post("/api/sim/frames", { data: { running: value } });
  expect(response.ok()).toBe(true);
}

interface Direction {
  ra: number;
  dec: number;
}

async function truth(request: APIRequestContext): Promise<{ camera: Direction; eyepiece: Direction }> {
  const response = await request.get("/api/sim/telescope");
  expect(response.ok()).toBe(true);
  return response.json();
}

// Chord distance, then 2 asin(d/2); no small-angle or RA-wrap approximation.
function arcseconds(a: Direction, b: Direction) {
  const vector = ({ ra, dec }: Direction) => {
    const r = (ra * Math.PI) / 180;
    const d = (dec * Math.PI) / 180;
    return [Math.cos(d) * Math.cos(r), Math.cos(d) * Math.sin(r), Math.sin(d)];
  };
  const av = vector(a);
  const bv = vector(b);
  const chord = Math.hypot(...av.map((v, i) => v - bv[i]!));
  return ((2 * Math.asin(Math.min(1, chord / 2)) * 180) / Math.PI) * 3600;
}

async function chartDirection(page: Page): Promise<Direction> {
  // Read the coordinates actually rendered in the chart's title block. Parsing
  // all three components also fails if the chart disappears or stops displaying RA/Dec.
  const text = await page.locator('[aria-labelledby="pointing-title"] dd').allTextContents();
  expect(text).toHaveLength(2);
  const parts = text.map((s) => [...s.replaceAll("−", "-").matchAll(/-?\d+(?:\.\d+)?/g)].map(Number));
  expect(parts[0]).toHaveLength(3);
  expect(parts[1]).toHaveLength(3);
  const [h, m, s] = parts[0]!;
  const [d, dm, ds] = parts[1]!;
  return {
    ra: 15 * (h! + m! / 60 + s! / 3600),
    dec: (text[1]!.includes("−") || text[1]!.includes("-") ? -1 : 1) * (Math.abs(d!) + dm! / 60 + ds! / 3600),
  };
}

async function open(page: Page, path = "/") {
  await page.addInitScript(() => localStorage.setItem("finder.night", "false"));
  await page.goto(path);
  await page.evaluate(() => document.fonts.ready);
}

function preview(page: Page) {
  return page.getByRole("img", { name: /^Live preview, frame / });
}

async function decoded(page: Page) {
  await expect
    .poll(() => preview(page).evaluate((img: HTMLImageElement) => img.complete && img.naturalWidth > 0))
    .toBe(true);
}

/** 012's offRed helper: zero green/blue; ink red (224) plus two rounding levels. */
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

test.beforeAll(async ({ request }) => {
  const response = await request.put("/api/session", {
    data: {
      site: { lat_deg: 40, lon_deg: -75 },
      target: "M78",
      // A known exposure makes preview mid-exposure a start-time oracle too.
      camera_settings: { exposure_ms: 200, gain: 1 },
    },
  });
  expect(response.ok()).toBe(true);
  expect((await session(request)).aim).toBeUndefined();
});

test.afterEach(async ({ request }) => {
  await running(request, true);
});

test("a push reaches the chart in 2 s and a post-push preview in 5.5 s", async ({ page, request, frames }) => {
  await open(page);
  const chart = page.getByRole("img", { name: /^Chart of Live frame / });
  await expect(chart).toBeVisible();
  await expect(preview(page)).toBeVisible();
  const before = await chartDirection(page);
  const started = performance.now();
  const pushed = await request.post("/api/sim/telescope", { data: { alt_deg: 40, az_deg: 200 } });
  expect(pushed.ok()).toBe(true);
  const pushedAt = (await session(request)).time!;
  const camera = (await truth(request)).camera;
  expect(arcseconds(before, camera)).toBeGreaterThan(3600); // A frozen chart cannot pass.
  await expect
    .poll(async () => arcseconds(await chartDirection(page), (await truth(request)).camera), {
      timeout: 2000,
      intervals: [50],
    })
    .toBeLessThan(30);
  expect(performance.now() - started).toBeLessThanOrEqual(2000);

  await expect
    .poll(
      async () => {
        const seq = Number((await preview(page).getAttribute("src"))?.split("seq=")[1]);
        const exposure = frames.find((f) => f.frame_id === seq && f.preview_seq === seq);
        // The photograph belongs to an exposure STARTED after the push, not just
        // an old photograph whose URL arrived later. Exposure is fixed at 0.2 s.
        return (
          exposure !== undefined &&
          exposure.time - 0.1 > pushedAt &&
          (await preview(page).evaluate((img: HTMLImageElement) => img.complete && img.naturalWidth > 0))
        );
      },
      { timeout: Math.max(1, 5500 - (performance.now() - started)), intervals: [50] },
    )
    .toBe(true);
  expect(performance.now() - started).toBeLessThanOrEqual(5500);
  const legend = await page.getByRole("group", { name: "Show on the chart" }).boundingBox();
  const photo = await preview(page).boundingBox();
  expect(legend).not.toBeNull();
  expect(photo).not.toBeNull();
  expect(photo!.y).toBeGreaterThanOrEqual(legend!.y + legend!.height);
});

test("stopped frames make M78 guidance and the chart stale within 6 s of mid-exposure", async ({
  page,
  request,
  frames,
}) => {
  await open(page);
  const guidance = page.getByRole("region", { name: /^Target M78/ });
  const header = page.locator('[aria-labelledby="pointing-title"]');
  await expect(guidance).toContainText(/turn .*tilt /);
  await expect.poll(() => frames.length).toBeGreaterThan(0);
  await running(request, false);
  await expect
    .poll(
      async () =>
        (await guidance.innerText()).includes("Pointing stale") &&
        (await header.innerText()).includes("Pointing stale"),
      { timeout: 6000, intervals: [50] },
    )
    .toBe(true);
  await expect(guidance).not.toContainText(/\bturn\b|\btilt\b/i);
  await expect(header).not.toContainText(/\bturn\b|\btilt\b/i);
  const read = await session(request);
  expect(read.site).toEqual({ lat_deg: 40, lon_deg: -75 });
  expect(read.target?.messier).toBe("M78");
  expect(read.pointing?.source).toBe("live");
  const last = frames.at(-1)!;
  expect(read.pointing?.frame_id).toBe(last.frame_id);
  expect(read.pointing?.time).toBeCloseTo(last.time, 6);
  const age = read.time! - last.time;
  expect(age).toBeGreaterThan(5);
  expect(age).toBeLessThanOrEqual(6); // Simulator time, never the host's calendar.
  expect(read.guidance?.stale).toBe(true);
  expect(read.guidance?.turn_deg).toBeUndefined();
  expect(read.guidance?.tilt_deg).toBeUndefined();
});

test("Setup shows the simulated camera and each frame's star count within 2 s", async ({ page, request, frames }) => {
  expect(
    (
      await request.put("/api/session", {
        data: { camera_settings: { exposure_ms: 20, gain: 1 } },
      })
    ).ok(),
  ).toBe(true);
  await open(page, "/setup");
  const node = page.getByRole("region", { name: "Scope node and camera" });
  await expect(node.getByText("Connected", { exact: true })).toBeVisible();
  await expect(node.getByText("simulator", { exact: true })).toBeVisible();
  await expect.poll(() => frames.at(-1)?.stars).toBeLessThan(25);
  const initial = frames.at(-1)!.stars;
  await expect(node.getByText(`${initial} stars`, { exact: true })).toBeVisible();
  // Hold back page polls after the initial camera description. The star count
  // must change through real live events as automatic exposure restores 25–40.
  await page.route("**/api/session", (route) => route.abort());
  expect(
    (
      await request.put("/api/session", {
        data: { camera_settings: { exposure_ms: "auto", gain: "auto" } },
      })
    ).ok(),
  ).toBe(true);
  let id = frames.at(-1)!.frame_id;
  let settled = 0;
  let changed = false;
  for (let i = 0; i < 10 && settled < 3; i++) {
    await expect.poll(() => frames.at(-1)?.frame_id, { timeout: 2000, intervals: [25] }).toBeGreaterThan(id);
    const frame = frames.at(-1)!;
    await expect(node.getByText(`${frame.stars} stars`, { exact: true })).toBeVisible({ timeout: 2000 });
    expect((await session(request)).time! - frame.time).toBeLessThanOrEqual(2);
    changed ||= frame.stars !== initial;
    settled = frame.stars >= 25 && frame.stars <= 40 ? settled + 1 : 0;
    id = frame.frame_id;
  }
  expect(changed).toBe(true);
  expect(settled).toBe(3);
});

test("the live chart and decoded preview obey the red-only night rule", async ({ page, frames }) => {
  await page.setViewportSize({ width: 412, height: 915 });
  await open(page);
  await expect(page.getByRole("img", { name: /^Chart of Live frame / })).toBeVisible();
  await expect.poll(() => frames.length).toBeGreaterThan(0);
  await expect(preview(page)).toBeVisible();
  await decoded(page);
  // Negative control: the same live page in day mode must fail the pixel rule.
  expect(await offRed(page, await page.screenshot({ fullPage: true }))).toBeGreaterThan(100_000);
  const toggle = page.getByRole("switch", { name: "Night mode: red and black" });
  await toggle.click();
  await expect(toggle).toBeChecked();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "night");
  await page.waitForTimeout(1500); // Same ink-in allowance as 012's night check.
  expect(await offRed(page, await page.screenshot({ fullPage: true, path: test.info().outputPath("night.png") }))).toBe(
    0,
  );
});

// Calibration is last: preceding chart comparisons use the true camera direction.
test("tapping the eyepiece star on Setup saves (700, 420) within 0.5 px", async ({ page, request, frames }) => {
  expect(
    (
      await request.put("/api/session", {
        data: { camera_settings: { exposure_ms: 200, gain: 1 } },
      })
    ).ok(),
  ).toBe(true);
  await open(page, "/setup");
  await expect(preview(page)).toBeVisible();
  await running(request, false);
  // Drain the in-flight exposure and wait out the five-second preview interval.
  // The next photograph must be taken immediately after centring the star.
  await page.waitForTimeout(6000);
  const response = await request.post("/api/sim/telescope", { data: { eyepiece_on_star: true } });
  expect(response.ok()).toBe(true);
  const centredAt = (await session(request)).time!;
  await running(request, true);
  await expect
    .poll(() => frames.find((f) => f.time - 0.1 > centredAt && f.frame_id === f.preview_seq)?.frame_id, {
      timeout: 2000,
      intervals: [25],
    })
    .toBeDefined();
  const frame = frames.find((f) => f.time - 0.1 > centredAt && f.frame_id === f.preview_seq)!;
  await expect(preview(page)).toHaveAttribute("src", `/api/live/preview.jpg?seq=${frame.frame_id}`);
  await decoded(page);
  // Tap six full-frame pixels off the oracle star, exercising server-side snapping
  // and the browser's displayed-preview to full-frame coordinate conversion.
  await preview(page).scrollIntoViewIfNeeded();
  const photo = await preview(page).boundingBox();
  expect(photo).not.toBeNull();
  const saved = page.waitForResponse((r) => r.url().endsWith("/api/aim") && r.request().method() === "POST");
  await page.mouse.click(
    photo!.x + (706 / frame.report.width) * photo!.width,
    photo!.y + (420 / frame.report.height) * photo!.height,
  );
  expect((await saved).ok()).toBe(true);
  await expect(page.getByRole("status")).toHaveText("Aim point set.");
  const read = await session(request);
  expect(read.aim).toBeDefined();
  expect(Math.abs(read.aim!.x - 700)).toBeLessThanOrEqual(0.5);
  expect(Math.abs(read.aim!.y - 420)).toBeLessThanOrEqual(0.5);
  await expect
    .poll(
      async () => {
        const pointing = (await session(request)).pointing!;
        return arcseconds({ ra: pointing.ra_deg, dec: pointing.dec_deg }, (await truth(request)).eyepiece);
      },
      { timeout: 2000, intervals: [50] },
    )
    .toBeLessThan(30);
});
