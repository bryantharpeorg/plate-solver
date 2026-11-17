import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { LiveReport, ObservingSession } from "./api";
import { ORION2 } from "./test/fixtures";
import { SESSION, TIME } from "./test/targets";
import { useLive } from "./useLive";

// Oracle: US3-S5's wire bearing, exactly 78°, on a newer sequence than the poll.
// The next event omits it, so the pointing must omit the key, not retain or null it.
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

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date("2040-01-01T00:00:00Z"));
  FakeEventSource.instances = [];
  vi.stubGlobal("EventSource", FakeEventSource);
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

it("keeps the event's up_deg on the live pointing and omits it when the next event has none", () => {
  const session: ObservingSession = {
    ...SESSION,
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
  const live = {
    frame_id: 42,
    time: TIME,
    pointing_seq: 8,
    preview_seq: 42,
    stars: ORION2.detected.length,
    report: { ...ORION2, name: null, pointing_seq: 8 },
    guidance: SESSION.guidance,
    up_deg: 78,
  };
  const { result } = renderHook(() => useLive(session));
  const stream = FakeEventSource.instances[0]!;
  act(() => stream.emit(live));
  expect(result.current.session.pointing?.seq).toBe(8);
  expect(result.current.session.pointing).toHaveProperty("up_deg", 78);

  const { up_deg, ...withoutUp } = live;
  act(() => stream.emit({ ...withoutUp, frame_id: 43, pointing_seq: 9 }));
  expect(result.current.session.pointing?.seq).toBe(9);
  expect(Object.hasOwn(result.current.session.pointing!, "up_deg")).toBe(false);
});
