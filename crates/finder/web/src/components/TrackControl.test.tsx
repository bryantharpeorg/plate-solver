import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ObservingSession } from "../api";
import captures from "../test/fixtures/session-track.json";
import { GuidanceBlock } from "./GuidanceBlock";

const recorded = captures as unknown as Record<keyof typeof captures, ObservingSession>;

// Oracle: unedited simulator/API captures (record-tracking.rs). Expectations below
// are fixed decimal rounding of those values: 1″, 0.01°/min and 0.1 s, exact text.
afterEach(() => vi.unstubAllGlobals());
const session = (name: keyof typeof recorded) => recorded[name] as ObservingSession;

describe("Track in guidance", () => {
  it("turns the switch off and explains too_low inside Track's own region", () => {
    render(<GuidanceBlock session={session("too_low")} />);
    const track = screen.getByRole("region", { name: "Track" });
    expect(within(track).getByRole("switch", { name: "Track" }).getAttribute("aria-checked")).toBe("false");
    expect(track.textContent).toMatch(/tracking ended.*target is too low/i);
  });

  it("rounds the recorded RMS, rotation and exposure independently", () => {
    render(<GuidanceBlock session={session("tracking")} />);
    const track = screen.getByRole("region", { name: "Track" });
    expect(within(track).getByRole("switch", { name: "Track" }).getAttribute("aria-checked")).toBe("true");
    expect(track.textContent).toContain("Tracking error (RMS, last minute)");
    expect(track.textContent).toContain("5″");
    expect(track.textContent).toContain("0.25°/min");
    expect(track.textContent).toContain("Longest exposure (1 px smear)");
    expect(track.textContent).toContain("3.7 s");
  });

  it("shows no limit exactly and omits exposure without an imaging camera", () => {
    const { rerender } = render(<GuidanceBlock session={session("no_limit")} />);
    expect(screen.getByRole("region", { name: "Track" }).textContent).toContain("no limit");
    rerender(<GuidanceBlock session={session("no_camera")} />);
    expect(screen.getByRole("region", { name: "Track" }).textContent).not.toMatch(/Longest exposure/);
  });

  it("offers no Track control without a connected mount", () => {
    const { mount: _mount, ...withoutMount } = session("tracking");
    const { rerender } = render(<GuidanceBlock session={withoutMount} />);
    expect(screen.queryByRole("switch", { name: "Track" })).toBeNull();
    expect(screen.queryByRole("region", { name: "Track" })).toBeNull();
    rerender(<GuidanceBlock session={{ ...withoutMount, mount: { ..._mount!, connected: false } }} />);
    expect(screen.queryByRole("switch", { name: "Track" })).toBeNull();
  });

  it.each([
    ["armed", false],
    ["too_low", true],
  ] as const)("posts Track from %s and refreshes", async (name, on) => {
    const fetch = vi.fn(async () => new Response(JSON.stringify(recorded.tracking)));
    vi.stubGlobal("fetch", fetch);
    const changed = vi.fn();
    render(<GuidanceBlock session={session(name)} onChange={changed} />);
    fireEvent.click(screen.getByRole("switch", { name: "Track" }));
    await waitFor(() =>
      expect(fetch).toHaveBeenCalledWith("/api/track", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ on }),
      }),
    );
    await waitFor(() => expect(changed).toHaveBeenCalledOnce());
  });

  it("retains a refused request without claiming Track is on", async () => {
    vi.stubGlobal("fetch", async () => new Response(JSON.stringify({ error: "Sun is up" }), { status: 409 }));
    render(<GuidanceBlock session={session("too_low")} />);
    fireEvent.click(screen.getByRole("switch", { name: "Track" }));
    expect((await screen.findByRole("alert")).textContent).toContain("Sun is up");
    expect(screen.getByRole("switch", { name: "Track" }).getAttribute("aria-checked")).toBe("false");
  });
});
