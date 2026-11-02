import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ObservingSession } from "../api";
import captured from "../test/fixtures/session-two-syncs.json";
import { MountSection } from "./MountSection";
import { SetupPage } from "./SetupPage";

// Oracle: true simulator base is 1° toward 130°, with two syncs >=90° apart.
// Display tolerances: 0.1° magnitude and 2° direction; RMS rounds to 0.1″.
afterEach(() => vi.unstubAllGlobals());
describe("Setup's mount and pointing model", () => {
  it("shows the two-sync base tilt, RMS and count inside Setup", () => {
    vi.stubGlobal(
      "EventSource",
      class {
        addEventListener() {}
        removeEventListener() {}
        close() {}
      },
    );
    render(<SetupPage session={captured as ObservingSession} put={async () => {}} onBack={() => {}} />);
    const tilt = screen.getByText(/base tilt [\d.]+° toward [\d.]+°/);
    const values = tilt.textContent!.match(/base tilt ([\d.]+)° toward ([\d.]+)°/)!;
    expect(Math.abs(Number(values[1]) - 1)).toBeLessThanOrEqual(0.1);
    expect(Math.abs(Number(values[2]) - 130)).toBeLessThanOrEqual(2);
    expect(tilt.textContent).toBe("base tilt 1.0° toward 130°");
    expect(screen.getByText("2 syncs")).toBeTruthy();
    expect(screen.getByText(`RMS residual ${captured.model.rms_arcsec.toFixed(1)}″`)).toBeTruthy();
  });

  it("shows mount link, board and firmware, and sends second sync and both reset routes", async () => {
    const fetch = vi.fn(async () => new Response(JSON.stringify({ started: true })));
    vi.stubGlobal("fetch", fetch);
    render(
      <MountSection
        session={
          { ...captured, mount: { ...captured.mount, board: "Sky-Watcher", firmware: "3.39" } } as ObservingSession
        }
      />,
    );
    expect(screen.getByText(/Connected/)).toBeTruthy();
    expect(screen.getByText(/scope.local:1/)).toBeTruthy();
    expect(screen.getByText(/Sky-Watcher/)).toBeTruthy();
    expect(screen.getByText(/3.39/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Second sync" }));
    await waitFor(() => expect(fetch).toHaveBeenCalledWith("/api/model/second-sync", { method: "POST" }));
    await screen.findByText(/Second sync requested/);
    for (const [name, what] of [
      ["Reset night's model", "night"],
      ["Reset fixed terms", "fixed"],
    ]) {
      fireEvent.click(screen.getByRole("button", { name }));
      await waitFor(() =>
        expect(fetch).toHaveBeenCalledWith("/api/model/reset", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ what }),
        }),
      );
      await screen.findByText(what === "night" ? "Night's model reset." : "Fixed terms reset.");
    }
  });

  it("keeps a refused second sync readable and does not report success", async () => {
    vi.stubGlobal("fetch", async () => new Response(JSON.stringify({ error: "Sun is up" }), { status: 409 }));
    render(<MountSection session={captured as ObservingSession} />);
    fireEvent.click(screen.getByRole("button", { name: "Second sync" }));
    expect((await screen.findByRole("alert")).textContent).toContain("Sun is up");
    expect(screen.queryByText(/Second sync requested/)).toBeNull();
  });
});
