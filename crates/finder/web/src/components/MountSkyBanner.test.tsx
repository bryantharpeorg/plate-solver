import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "../App";
import type { MountSky } from "../api";
import { SESSION } from "../test/targets";

// Oracles: US3-S1's required wording, read as text from the served session, whose
// mount_sky decides each test. The banner is outside the page switch, so /setup shows
// it too; the guidance region is the page's own signal that the session has loaded.

const SKY: MountSky = { tube_alt_deg: 30, zero_counts: [1, 2] };
const HEALTH = {
  status: "ok",
  version: "0.1.0",
  database: { catalog: "hip_main", stars: 42212, patterns: 1, min_fov: 10, max_fov: 30 },
  samples: 1,
};

function serve(sky: MountSky | null) {
  vi.stubGlobal("fetch", async (input: RequestInfo | URL) => {
    const url = String(input);
    const body =
      url === "/api/session"
        ? sky
          ? { ...SESSION, mount_sky: sky }
          : { ...SESSION }
        : url === "/api/samples"
          ? []
          : HEALTH;
    return new Response(JSON.stringify(body));
  });
}

/** One status, once the session's target guidance has shown. */
async function bannerText() {
  await screen.findByRole("region", { name: /Target/ });
  const statuses = screen.getAllByRole("status");
  expect(statuses).toHaveLength(1);
  return statuses[0]!.textContent ?? "";
}

beforeEach(() => window.history.replaceState(null, "", "/"));
afterEach(() => vi.unstubAllGlobals());

describe("the drawn-sky banner", () => {
  it("says the stars are drawn from the mount, not seen by the camera, and forbids hand pushes (S1)", async () => {
    serve(SKY);
    const { container } = render(<App />);
    const text = await bannerText();
    expect(text).toContain("Drawn sky");
    expect(text).toContain("not seen by the camera");
    expect(text).toContain("No hand pushes");
    expect(container.textContent).not.toContain("Night clock");
    expect(container.textContent).not.toContain("power-cycled");
  });

  it("shows the same banner at /setup, which has no guidance region (S1)", async () => {
    window.history.replaceState(null, "", "/setup");
    serve(SKY);
    const { container } = render(<App />);
    const statuses = await screen.findAllByRole("status");
    expect(statuses).toHaveLength(1);
    const text = statuses[0]!.textContent ?? "";
    expect(text).toContain("Drawn sky");
    expect(text).toContain("not seen by the camera");
    expect(text).toContain("No hand pushes");
    expect(container.textContent).not.toContain("Night clock");
    expect(container.textContent).not.toContain("power-cycled");
    expect(screen.queryByRole("region", { name: /Target/ })).toBeNull();
  });

  it("adds the night clock and the covers when the session has night_utc (S2)", async () => {
    serve({ ...SKY, night_utc: "2026-01-16T02:00:00Z" });
    render(<App />);
    const text = await bannerText();
    expect(text).toContain("Night clock from 2026-01-16T02:00:00Z");
    expect(text).toContain("Keep the mirror cover and the finder cap on");
  });

  it("says the drawn sky stopped after a power cycle and asks for a fresh --tube-alt (S4)", async () => {
    serve({ ...SKY, power_cycled: true });
    render(<App />);
    const text = await bannerText();
    expect(text).toContain("The mount was power-cycled");
    expect(text).toContain("restart the console with a fresh --tube-alt");
  });

  it("shows no banner on a session without mount_sky (S3)", async () => {
    serve(null);
    const { container } = render(<App />);
    await screen.findByRole("region", { name: /Target/ });
    expect(screen.queryAllByRole("status")).toHaveLength(0);
    expect(container.textContent).not.toContain("Drawn sky");
  });
});
