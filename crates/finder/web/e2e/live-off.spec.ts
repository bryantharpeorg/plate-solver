// The default server has no scope node: opening the console must leave no stream
// request in flight, including while the first five-second session poll runs.
import { expect, test } from "@playwright/test";

test("a console with no scope node opens no live stream and reaches network idle", async ({ page }) => {
  const streams: string[] = [];
  page.on("request", (request) => {
    if (new URL(request.url()).pathname === "/api/live") streams.push(request.url());
  });
  await page.goto("/", { waitUntil: "networkidle" });
  await expect(page.getByRole("heading", { name: "Chart a frame", exact: true })).toBeVisible();
  const response = await page.request.get("/api/session");
  expect(response.ok()).toBe(true);
  expect((await response.json()).node).toBeUndefined();
  await page.waitForResponse((r) => new URL(r.url()).pathname === "/api/session");
  await page.waitForLoadState("networkidle");
  expect(streams).toEqual([]);
});
