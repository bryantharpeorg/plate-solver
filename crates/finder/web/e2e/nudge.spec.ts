// The nudge arrows on the simulated console (055 US5-S5 to S8). Real finder --simulate, its
// API answers and rendered pixels. Oracles: the answers the scenarios write out exactly, the
// session's own axis_move and mount.moving, and DESIGN.md's #e00000 night ink. No responses
// are replaced. The session is put back as tracking.spec.ts puts it back.
import { expect, test, type APIRequestContext, type Locator, type Page, type Response } from "@playwright/test";
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

const isNudge = (response: Response) =>
  response.url().endsWith("/api/mount/nudge") && response.request().method() === "POST";
const nudgeBlock = (page: Page) => page.getByRole("region", { name: "Nudge", exact: true });

/** Press a button and return the answer the page received for it. */
async function press(page: Page, button: Locator): Promise<Response> {
  const answered = page.waitForResponse(isNudge);
  await button.click();
  return answered;
}

// 012's offRed helper, kept local because importing a spec would register its tests again.
// Zero green/blue; ink red (224) plus two rounding levels at most.
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
  await expect(nudgeBlock(page)).toBeVisible();
  await expect.poll(async () => (await session(request)).mount?.moving).toBe(false);
});

test.afterEach(async ({ request }) => {
  await post(request, "/api/track", { on: false });
  // A session that never started Track has no track key at all: that is off too.
  await expect.poll(async () => (await session(request)).track?.state ?? "off").toBe("off");
  await expect.poll(async () => (await session(request)).mount?.moving).toBe(false);
});

test.afterAll(async ({ request }) => {
  await post(request, "/api/track", { on: false });
  const response = await request.put("/api/session", { data: { imaging_camera: null, target: null } });
  expect(response.ok()).toBe(true);
});

test("with Track off, a fine press moves the mount 30″ and a refused press says why (US5-S5)", async ({
  page,
  request,
}) => {
  const block = nudgeBlock(page);
  await expect(block.getByRole("combobox", { name: "Arrows follow" })).toHaveValue("mount");

  const right = await press(page, block.getByRole("button", { name: "Nudge right" }));
  expect(right.status()).toBe(200);
  expect(await right.json()).toEqual({ up_arcsec: 0, right_arcsec: 30, moved: "axes" });
  await expect.poll(async () => (await session(request)).axis_move?.state, { timeout: 60_000 }).toBe("done");
  await expect.poll(async () => (await session(request)).mount?.moving).toBe(false);
  await expect(block.getByRole("status")).toHaveText("Nudged right 30″");

  await block.getByRole("button", { name: "Coarse 4′" }).click();
  const down = await press(page, block.getByRole("button", { name: "Nudge down" }));
  expect(down.status()).toBe(200);
  expect(await down.json()).toEqual({ up_arcsec: -240, right_arcsec: 0, moved: "axes" });
  // Playwright waits for the arrows to be enabled again, so this press lands while the move runs.
  const up = await press(page, block.getByRole("button", { name: "Nudge up" }));
  expect(up.status()).toBe(409);
  await expect(block.getByRole("alert")).toHaveText("the mount is already moving");
  await expect.poll(async () => (await session(request)).axis_move?.state, { timeout: 60_000 }).toBe("done");
  await expect.poll(async () => (await session(request)).mount?.moving).toBe(false);

  await block.getByRole("button", { name: "Coarse 4′" }).click();
  const left = await press(page, block.getByRole("button", { name: "Nudge left" }));
  expect(left.status()).toBe(200);
  await page.waitForTimeout(1500);
  expect((await session(request)).axis_move?.state).toBe("moving");
  await expect(block.getByRole("status")).toHaveText("Nudging left 4′…");
  await expect.poll(async () => (await session(request)).axis_move?.state, { timeout: 120_000 }).toBe("done");
  await expect.poll(async () => (await session(request)).mount?.moving).toBe(false);
});

test("with Track on, a coarse press moves the held place and Track holds it (US5-S6)", async ({ page, request }) => {
  const block = nudgeBlock(page);
  const track = page.getByRole("region", { name: "Track", exact: true });
  await track.getByRole("switch", { name: "Track", exact: true }).click();
  await expect.poll(async () => (await session(request)).track?.state, { timeout: 120_000 }).toBe("tracking");

  await block.getByRole("button", { name: "Coarse 4′" }).click();
  const answered = page.waitForResponse(isNudge);
  await block.getByRole("button", { name: "Nudge up" }).click();
  const answer = await answered;
  expect(answer.status()).toBe(200);
  expect(await answer.json()).toEqual({ up_arcsec: 240, right_arcsec: 0, moved: "track" });

  // Ten seconds of reads, every 500 ms: Track keeps tracking and no push by hand appears.
  for (let read = 0; read < 20; read++) {
    const now = await session(request);
    expect(now.track?.state).toBe("tracking");
    expect(now.track?.push).toBeUndefined();
    await page.waitForTimeout(500);
  }
  await expect(block.getByRole("status")).toHaveText("Nudged up 4′ · Track holds the new place", { timeout: 20_000 });
  await track.getByRole("switch", { name: "Track", exact: true }).click();
  await expect.poll(async () => (await session(request)).track?.state, { timeout: 30_000 }).toBe("off");
});

test("the Nudge block is light by day and red and black by night (US5-S7)", async ({ page }) => {
  const block = nudgeBlock(page);
  await expect(block).toBeVisible();
  // Negative control: this exact scanner must count the day edition's paper.
  expect(await offRed(page, await block.screenshot())).toBeGreaterThan(1_000);

  const night = page.getByRole("switch", { name: "Night mode: red and black" });
  await night.click();
  await expect(night).toBeChecked();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "night");
  await page.waitForTimeout(1500); // 012's allowance for the ink-in animation.
  expect(await offRed(page, await block.screenshot())).toBe(0);
});

test("the Imaging page shows the Nudge block with the imaging camera chosen (US5-S8)", async ({ page }) => {
  await page.goto("/imaging");
  const block = nudgeBlock(page);
  await expect(block).toBeVisible({ timeout: 20_000 });
  await expect(block.getByRole("combobox", { name: "Arrows follow" })).toHaveValue("imaging_camera");
});
