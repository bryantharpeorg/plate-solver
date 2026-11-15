import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import type { LiveReport, ObservingSession, SolveReport } from "./api";
import { ORION2, SIMILAR_M42 } from "./test/fixtures";
import { SESSION, TIME } from "./test/targets";

class FakeEventSource extends EventTarget {
  static instances: FakeEventSource[] = [];
  close = vi.fn();
  constructor(public url: string) {
    super();
    FakeEventSource.instances.push(this);
  }
  emit(report: LiveReport) {
    this.dispatchEvent(new MessageEvent("live", { data: JSON.stringify(report) }));
  }
}
const CONNECTED: ObservingSession = {
  ...SESSION,
  time: TIME,
  pointing: null,
  node: {
    address: "scope:50051",
    token_set: true,
    connected: true,
    last_frame_age_s: 0,
    offset_s: 1000,
    camera: null,
    stars: 30,
    out_of_range: [],
  },
};
function serve(upload: SolveReport = frame(false)) {
  const started = Date.now();
  vi.stubGlobal("fetch", async (input: RequestInfo | URL) => {
    const url = String(input);
    const body =
      url === "/api/session"
        ? { ...CONNECTED, time: TIME + (Date.now() - started) / 1000 }
        : url === "/api/solve"
          ? { ...upload, pointing_seq: 100 }
          : url.startsWith("/api/similar")
            ? SIMILAR_M42
            : url === "/api/samples"
              ? [{ name: "orion2.jpg", label: "Orion" }]
              : {
                  status: "ok",
                  version: "0.1.0",
                  database: { catalog: "hip_main", stars: 42212, patterns: 1, min_fov: 10, max_fov: 30 },
                  samples: 1,
                };
    return new Response(JSON.stringify(body));
  });
}

// Oracle: saved ORION2 positions/names, with the spec's exact sighting sequence.
// Each event has a distinct exposure time; DOM classes/text have zero tolerance.
function frame(matched: boolean): SolveReport {
  return {
    ...ORION2,
    name: null,
    stars: ORION2.stars.map((s, i) => ({ ...s, matched: i === 0 && matched, detected: false })),
  };
}
async function emit(index: number, matched: boolean) {
  if (index > 0) await act(() => vi.advanceTimersByTimeAsync(3000));
  act(() =>
    FakeEventSource.instances[0]!.emit({
      frame_id: 42 + index,
      time: TIME + index * 3,
      pointing_seq: 8 + index,
      preview_seq: null,
      stars: ORION2.detected.length,
      report: frame(matched),
      guidance: SESSION.guidance,
    }),
  );
}
function mark(container: HTMLElement, seen: boolean) {
  const star = container.querySelector(`[data-star="${ORION2.stars[0]!.id}"]`);
  expect(star).not.toBeNull();
  expect(star!.classList.contains(seen ? "mark-matched" : "mark-expected")).toBe(true);
}
const namedLine = () => screen.getByRole("region", { name: "In the field" }).querySelector("[data-named-stars]");

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date("2040-01-01T00:00:00Z"));
  vi.spyOn(document, "visibilityState", "get").mockReturnValue("visible");
  window.history.replaceState(null, "", "/");
  FakeEventSource.instances = [];
  vi.stubGlobal("EventSource", FakeEventSource);
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe("the live console's star hold", () => {
  it("draws an alternating star unseen after the second, seen after the third and fourth, keeping its name (S5)", async () => {
    serve();
    const { container } = render(<App />);
    await act(async () => {});
    await emit(0, true);
    await emit(1, false);
    mark(container, false);
    await emit(2, true);
    mark(container, true);
    const names = namedLine()!.textContent;
    expect(names).toBe(ORION2.stars[0]!.name);
    await emit(3, false);
    mark(container, true);
    expect(namedLine()!.textContent).toBe(names);
    expect(container.querySelector(`[data-star="${ORION2.stars[0]!.id}"] .mark-label`)?.textContent).toBe(names);
  });

  it("keeps the empty named-stars line through three misses after exactly one report (S6)", async () => {
    serve();
    render(<App />);
    await act(async () => {});
    await emit(0, true);
    expect(namedLine()!.textContent).toBe(ORION2.stars[0]!.name);
    for (let i = 1; i <= 3; i++) {
      await emit(i, false);
      const line = namedLine();
      expect(line).not.toBeNull();
      expect(line!.textContent).toBe("\u00a0");
      expect(line!.querySelector("svg")).toBeNull();
    }
  });

  it.each(["upload", "sample"])("draws a %s from its own report after three live sightings (S7)", async (source) => {
    serve();
    vi.spyOn(URL, "createObjectURL").mockReturnValue("blob:frame");
    vi.spyOn(URL, "revokeObjectURL").mockImplementation(() => {});
    const { container } = render(<App />);
    await act(async () => {});
    for (let i = 0; i < 3; i++) await emit(i, true);
    mark(container, true);
    fireEvent.click(screen.getByRole("button", { name: "Live frame 44" }));
    await act(async () => {});
    const picker = screen.getByRole("dialog", { name: "Choose a frame" });
    if (source === "sample") fireEvent.click(within(picker).getByRole("button", { name: "Orion" }));
    else
      fireEvent.change(picker.querySelector('input[type="file"]')!, {
        target: { files: [new File(["frame"], "frame.jpg", { type: "image/jpeg" })] },
      });
    await act(async () => {});
    mark(container, false);
    expect(namedLine()).toBeNull();
    await emit(100, false);
    mark(container, false);
  });

  it("records raw sightings so three misses drop the star on the sixth report (S8)", async () => {
    serve();
    const { container } = render(<App />);
    await act(async () => {});
    for (let i = 0; i < 6; i++) {
      await emit(i, i < 3);
      mark(container, i < 5);
    }
  });
});
