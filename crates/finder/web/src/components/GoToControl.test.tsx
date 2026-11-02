import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ObservingSession } from "../api";
import ready from "../test/fixtures/session-ready.json";
import below from "../test/fixtures/session-below-horizon.json";
import daylight from "../test/fixtures/session-sun-up.json";
import unsynced from "../test/fixtures/session-no-sync.json";
import approach from "../test/fixtures/session-low-approach.json";
import recorded from "../test/fixtures/session-goto-m42.json";
import { GuidanceBlock } from "./GuidanceBlock";
import { StopButton } from "./StopButton";

// Oracle: captured simulator sessions and the API's motion guards. Exact text,
// correction numbers and 0.1 arcsecond display rounding; no browser clock.
afterEach(() => vi.unstubAllGlobals());
describe("GoTo in guidance", () => {
  it.each([
    [below, /below the horizon/i],
    [daylight, /Sun is up/i],
    [unsynced, /no sync yet/i],
    [approach, /approach.*minimum altitude/i],
  ])("explains a refused session while Stop remains available", (fixture, reason) => {
    const session = fixture as ObservingSession;
    render(
      <>
        <GuidanceBlock session={session} />
        <StopButton mount={{ ...session.mount!, moving: true }} />
      </>,
    );
    const block = screen.getByRole("region", { name: /Target/ });
    expect((within(block).getByRole("button", { name: "GoTo" }) as HTMLButtonElement).disabled).toBe(true);
    expect(block.textContent).toMatch(reason);
    expect((screen.getByRole("button", { name: "Stop" }) as HTMLButtonElement).disabled).toBe(false);
  });

  it("starts an accepted GoTo and shows a server refusal without claiming motion", async () => {
    const fetch = vi.fn(async () => new Response(JSON.stringify({ error: "mount did not answer" }), { status: 409 }));
    vi.stubGlobal("fetch", fetch);
    render(<GuidanceBlock session={ready as ObservingSession} />);
    const go = screen.getByRole("button", { name: "GoTo" }) as HTMLButtonElement;
    expect(go.disabled).toBe(false);
    fireEvent.click(go);
    await waitFor(() => expect(fetch).toHaveBeenCalledWith("/api/goto", { method: "POST" }));
    expect((await screen.findByRole("alert")).textContent).toContain("mount did not answer");
  });

  it("renders the recorded M42 slew, solving, every numbered correction, and final error", () => {
    const { rerender } = render(<GuidanceBlock session={recorded[0] as ObservingSession} />);
    const corrections = new Set<number>();
    for (const session of recorded as ObservingSession[]) {
      rerender(<GuidanceBlock session={session} />);
      const progress = screen.getByRole("status");
      const go = session.goto!;
      if (go.state === "slewing") expect(progress.textContent).toMatch(/slewing/i);
      if (go.state === "solving") expect(progress.textContent).toMatch(/solving/i);
      if (go.correction > 0) {
        corrections.add(go.correction);
        expect(progress.textContent).toContain(`Correction ${go.correction}`);
      }
      if (go.state === "done") {
        expect(progress.textContent).toMatch(/on target/i);
        expect(progress.textContent).toContain(`${go.error_arcsec!.toFixed(1)}″`);
      }
    }
    expect([...corrections]).toEqual(Array.from({ length: corrections.size }, (_, i) => i + 1));
    expect(corrections.size).toBeGreaterThan(0);
    expect(recorded.at(-1)!.goto.state).toBe("done");
  });

  it("renders no GoTo without mount telemetry", () => {
    const { mount: _mount, ...session } = ready;
    render(<GuidanceBlock session={session as ObservingSession} />);
    expect(screen.queryByRole("button", { name: "GoTo" })).toBeNull();
  });
});
