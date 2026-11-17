import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import type { LiveReport, ObservingSession } from "./api";
import { ORION2, SIMILAR_M42, VIRGO } from "./test/fixtures";
import { SESSION, TIME } from "./test/targets";

// A copy of App.live.test's harness, with the session's chart rotation set to a quarter
// turn and the live report in the plan's wide 1296 × 972 frame. Oracle: US2-S9's
// hand-computed quarter turn — the group's transform, the page's aspect and the live
// photo's box.

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
const LIVE: LiveReport = {
  frame_id: 42,
  time: TIME,
  pointing_seq: 8,
  preview_seq: 42,
  stars: ORION2.detected.length,
  report: { ...ORION2, name: null, width: 1296, height: 972, pointing_seq: 8 },
  guidance: SESSION.guidance,
};
const CONNECTED: ObservingSession = {
  ...SESSION,
  time: TIME,
  pointing: null,
  chart_rotation_deg: 90,
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
function serve(initial: ObservingSession = CONNECTED) {
  let session = initial;
  const started = Date.now();
  vi.stubGlobal("fetch", async (input: RequestInfo | URL) => {
    const url = String(input);
    const body =
      url === "/api/session"
        ? { ...session, time: (session.time ?? TIME) + (Date.now() - started) / 1000 }
        : url === "/api/solve"
          ? { ...VIRGO, pointing_seq: 9 }
          : url.startsWith("/api/similar")
            ? SIMILAR_M42
            : url === "/api/samples"
              ? [{ name: "virgo.jpg", label: "Virgo" }]
              : {
                  status: "ok",
                  version: "0.1.0",
                  database: { catalog: "hip_main", stars: 42212, patterns: 1, min_fov: 10, max_fov: 30 },
                  samples: 1,
                };
    return new Response(JSON.stringify(body));
  });
  return (next: ObservingSession) => (session = next);
}

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

describe("the turned live page", () => {
  it("turns the chart's frame group and sizes the page and the live photo by the turn (S9)", async () => {
    serve();
    const { container } = render(<App />);
    await act(async () => {});
    act(() => FakeEventSource.instances[0]!.emit(LIVE));
    await act(async () => {});
    const svg = container.querySelector('svg[role="img"]')!;
    expect(
      [...svg.querySelectorAll("g")].some(
        (g) => g.getAttribute("transform") === "translate(486 648) rotate(-90) translate(-648 -486)",
      ),
    ).toBe(true);
    expect(container.firstElementChild!.style.getPropertyValue("--chart-aspect")).toBe("0.75");
    const preview = screen.getByRole("img", { name: /preview/i });
    expect(preview.parentElement!.style.getPropertyValue("aspect-ratio")).toBe("0.75 / 1");
  });
});
