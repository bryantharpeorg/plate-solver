// Oracles: FR-015's exact wording, the simulator's horizontal eyepiece truth,
// and DESIGN.md's red-only night palette. No network responses are replaced.
import { expect, test, type APIRequestContext, type Page } from "@playwright/test";
import type { ObservingSession } from "../src/api";

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

async function eyepiece(request: APIRequestContext): Promise<{ alt: number; az: number }> {
  const response = await request.get("/api/sim/telescope");
  expect(response.ok()).toBe(true);
  return (await response.json()).eyepiece;
}

/** Zero off-red pixels, allowing the existing palette's two red rounding levels. */
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
  expect((await request.delete("/api/aim")).ok()).toBe(true);
  await post(request, "/api/model/reset", { what: "night" });
});

test("MCP GoTo announces its source, stays red at night, and the single Stop halts it", async ({ page, request }) => {
  test.setTimeout(90_000);
  await post(request, "/api/stop");
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
  await post(request, "/api/model/reset", { what: "night" });
  await post(request, "/api/sim/frames", { running: true });
  await expect.poll(async () => (await session(request)).model?.syncs).toBe(1);
  await page.addInitScript(() => localStorage.setItem("finder.night", "true"));
  await page.goto("/");
  await page.evaluate(() => document.fonts.ready);
  const block = page.getByRole("region", { name: /^Target M42/ });
  const top = page.getByRole("banner");
  await expect(block.getByRole("button", { name: "GoTo", exact: true })).toBeEnabled();
  const response = await request.post("/mcp", {
    headers: {
      Accept: "application/json, text/event-stream",
      "MCP-Protocol-Version": "2025-06-18",
    },
    data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name: "goto_target", arguments: {} } },
  });
  expect(response.ok(), await response.text()).toBe(true);
  expect((await response.json()).result.structuredContent).toEqual({ started: true });
  // One shared deadline proves both copies arrive within 1 s of the HTTP call.
  await expect
    .poll(
      async () =>
        (await block.getByText("Moved by an MCP client · goto_target", { exact: true }).isVisible()) &&
        (await top.getByText("Moved by an MCP client · goto_target", { exact: true }).isVisible()),
      { timeout: 1000, intervals: [25] },
    )
    .toBe(true);
  const stop = top.getByRole("button", { name: "Stop", exact: true });
  await expect(page.getByRole("button", { name: "Stop", exact: true })).toHaveCount(1);
  await expect(stop).toBeEnabled();
  // Positive control: a connected but inert telescope cannot prove Stop works.
  const before = await eyepiece(request);
  await expect
    .poll(async () => {
      const now = await eyepiece(request);
      return Math.hypot(now.alt - before.alt, now.az - before.az);
    })
    .toBeGreaterThan(0.01);
  for (const note of [block, top].map((where) => where.getByText("Moved by an MCP client · goto_target"))) {
    expect(await offRed(page, await note.screenshot())).toBe(0);
  }
  await stop.click();
  await expect
    .poll(async () => {
      const state = await session(request);
      return { state: state.goto?.state, reason: state.goto?.reason, moving: state.mount?.moving };
    })
    .toEqual({ state: "halted", reason: "the observer stopped it", moving: false });
  const stopped = await eyepiece(request);
  await page.waitForTimeout(2000);
  const later = await eyepiece(request);
  expect(Math.abs(later.alt - stopped.alt)).toBeLessThan(1e-8);
  expect(Math.abs(later.az - stopped.az)).toBeLessThan(1e-8);
  await expect(block.getByText("Moved by an MCP client · goto_target")).toHaveCount(0);
  await expect(top.getByText("Moved by an MCP client · goto_target")).toHaveCount(0);
  await page.getByRole("link", { name: "Setup", exact: true }).click();
  const mcp = page.getByRole("region", { name: "MCP", exact: true });
  await expect(mcp).toBeVisible();
  await expect(mcp.getByText("http://localhost:8099/mcp", { exact: true })).toBeVisible();
  expect(await offRed(page, await mcp.screenshot())).toBe(0);
});
