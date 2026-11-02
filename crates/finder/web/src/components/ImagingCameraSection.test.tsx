import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "../App";
import type { ObservingSession } from "../api";
import captures from "../test/fixtures/session-track.json";
import { SetupPage } from "./SetupPage";

const recorded = captures as unknown as Record<keyof typeof captures, ObservingSession>;

// Oracle: FR-015's checked ranges and captured sessions before/after a
// 23.5 × 15.6 mm, 3.76 µm camera edit. Wire values and display text are exact.
beforeEach(() =>
  vi.stubGlobal(
    "EventSource",
    class {
      addEventListener() {}
      removeEventListener() {}
      close() {}
    },
  ),
);
afterEach(() => {
  window.history.replaceState(null, "", "/");
  vi.unstubAllGlobals();
});
function fill(width = "23.5", height = "15.6", pitch = "3.76") {
  for (const [label, value] of [
    ["Sensor width", width],
    ["Sensor height", height],
    ["Pixel pitch", pitch],
  ]) {
    fireEvent.change(screen.getByRole("textbox", { name: label }), { target: { value } });
  }
}
function setup(session = recorded.no_camera as ObservingSession) {
  const put = vi.fn(async () => {});
  render(<SetupPage session={session} put={put} onBack={() => {}} />);
  return put;
}

describe("Setup's imaging camera", () => {
  it("PUTs the three camera values and gains the exposure readout on return to guidance", async () => {
    let session = recorded.no_camera as ObservingSession;
    vi.stubGlobal(
      "EventSource",
      class {
        addEventListener() {}
        removeEventListener() {}
        close() {}
      },
    );
    const fetch = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      if (String(input) === "/api/session") {
        if (init?.method === "PUT") session = recorded.tracking as ObservingSession;
        return new Response(JSON.stringify(session));
      }
      return new Response(
        JSON.stringify(
          String(input) === "/api/samples" ? [] : { version: "test", database: { catalog: "fixture", stars: 1 } },
        ),
      );
    });
    vi.stubGlobal("fetch", fetch);
    render(<App />);
    await screen.findByRole("region", { name: "Track" });
    expect(screen.queryByText(/Longest exposure/)).toBeNull();
    fireEvent.click(screen.getByRole("link", { name: "Setup" }));
    fill();
    fireEvent.click(screen.getByRole("button", { name: "Save imaging camera" }));
    await screen.findByText("Imaging camera saved.");
    expect(fetch).toHaveBeenCalledWith("/api/session", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ imaging_camera: { width_mm: 23.5, height_mm: 15.6, pitch_um: 3.76 } }),
    });
    fireEvent.click(screen.getByRole("link", { name: "Back to console" }));
    const track = screen.getByRole("region", { name: "Track" });
    expect(track.textContent).toContain("Longest exposure (1 px smear)");
    expect(track.textContent).toContain("3.7 s");
  });

  it.each([
    ["23.5", "15.6", "0"],
    ["0", "15.6", "3.76"],
    ["23.5", "0", "3.76"],
    ["100.1", "15.6", "3.76"],
    ["23.5", "100.1", "3.76"],
    ["23.5", "15.6", "50.1"],
    ["", "15.6", "3.76"],
    ["23.5", "15.6", "NaN"],
    ["23.5", "15.6", "Infinity"],
  ])("refuses %s × %s mm, %s µm without saving and states allowed ranges", (width, height, pitch) => {
    const put = setup();
    fill(width, height, pitch);
    fireEvent.click(screen.getByRole("button", { name: "Save imaging camera" }));
    expect(put).not.toHaveBeenCalled();
    const section = screen.getByRole("region", { name: "Imaging camera" });
    expect(within(section).getByRole("alert").textContent).toMatch(/greater than 0.*100 mm.*greater than 0.*50 µm/);
  });

  it("accepts upper bounds and can clear a saved camera with null", async () => {
    const put = setup(recorded.tracking as ObservingSession);
    expect((screen.getByLabelText("Pixel pitch") as HTMLInputElement).value).toBe("3.76");
    fill("100", "100", "50");
    fireEvent.click(screen.getByRole("button", { name: "Save imaging camera" }));
    await screen.findByText("Imaging camera saved.");
    expect(put).toHaveBeenCalledWith({ imaging_camera: { width_mm: 100, height_mm: 100, pitch_um: 50 } });
    fireEvent.click(screen.getByRole("button", { name: "Forget imaging camera" }));
    await waitFor(() => expect(put).toHaveBeenLastCalledWith({ imaging_camera: null }));
  });

  it("keeps a refused save visible", async () => {
    render(
      <SetupPage
        session={recorded.no_camera as ObservingSession}
        put={async () => {
          throw new Error("Session could not be saved");
        }}
        onBack={() => {}}
      />,
    );
    fill();
    fireEvent.click(screen.getByRole("button", { name: "Save imaging camera" }));
    expect((await screen.findByRole("alert")).textContent).toBe("Session could not be saved");
    expect(screen.queryByText("Imaging camera saved.")).toBeNull();
  });
});
