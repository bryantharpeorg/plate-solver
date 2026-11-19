// US9-S1–S6. Oracles: the spec's literal views, UTC dates, request bodies and
// observer-facing words. Text and numbers are exact; the hook's clock is injected
// by fake timers and a deferred HTTP answer, so no elapsed real time is assumed.
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { CalibrationMaster, CalibrationSet, ImagingCalibration, ImagingView } from "../api";
import { CalibrationBlock } from "./CalibrationBlock";
import { ImagingPage } from "./ImagingPage";

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

// 031 US1-S1, without calibration: compatibility with the already shipped page.
const READY: ImagingView = {
  state: "ready",
  camera: {
    model: "simulated imaging camera",
    serial: "SIM-0001",
    width: 1236,
    height: 822,
    pixel_um: 19.12,
    bits: 16,
    bayer: "RGGB",
    exposure_ms: { min: 1, max: 3600000 },
    gain: { min: 0, max: 300 },
    offset: { min: 0, max: 100 },
    binnings: [1, 2],
    cooler: true,
  },
  status: { temp_c: 20, cooler_power: 0, set_temp_c: null, exposing: false },
  settings: {
    exposure_ms: 1000,
    gain: 120,
    offset: 10,
    binning: 1,
    roi: { x: 0, y: 0, width: 1236, height: 822 },
    cooler_on: false,
    set_temp_c: null,
    forward_kib_per_s: null,
    save_at_node: false,
  },
};
const DARK: CalibrationMaster = {
  id: "dark-20260116T020003Z",
  kind: "dark",
  made_at: 1768528803.7,
  frames: 20,
  checked: true,
  gain: 120,
  offset: 10,
  binning: 1,
  set_temp_c: -10,
  temp_c: -10,
  exposure_ms: 5000,
  bad_pixels: 137,
  label: null,
  imaging_angle_deg: null,
};
const FLAT: CalibrationMaster = {
  id: "flat-20260116T030000Z",
  kind: "flat",
  made_at: 1768532400,
  frames: 20,
  checked: true,
  gain: 120,
  offset: 10,
  binning: 1,
  set_temp_c: -10,
  temp_c: -10,
  exposure_ms: 132.93,
  bad_pixels: null,
  label: "desk",
  imaging_angle_deg: 17,
};
const DARK_LINE =
  "Master dark · 5000 ms · gain 120 · offset 10 · 1 × 1 · set -10.0 °C · 20 frames · 137 bad pixels · 2026-01-16";
const FLAT_LINE =
  "Master flat desk · gain 120 · offset 10 · 1 × 1 · set -10.0 °C · 20 frames · 2026-01-16 · camera at 17.0°";
const COVERED = "Only if the telescope is covered: a warm sensor or a long exposure can read above the offset level.";
const SET: CalibrationSet = { id: "dark-x", kind: "dark", state: "offset", wanted: 4, taken: 0, exposure_ms: 200 };
const FLATS: CalibrationSet = {
  id: "flat-x",
  kind: "flat",
  state: "needs_darks",
  wanted: 3,
  taken: 3,
  exposure_ms: 366.634,
  label: "desk",
};
const response = (body: unknown, status = 200) => new Response(JSON.stringify(body), { status });
function serve(body: unknown = { masters: [] }, status = 200) {
  const fetch = vi.fn(async (_input: RequestInfo | URL, _init?: RequestInit) => response(body, status));
  vi.stubGlobal("fetch", fetch);
  return fetch;
}
const button = (name: string) => screen.getByRole("button", { name }) as HTMLButtonElement;
const field = (name: string) => screen.getByLabelText(name) as HTMLInputElement;
const post = (body: object) => ({
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify(body),
});
function block(calibration: ImagingCalibration = { masters: [] }) {
  const onChange = vi.fn();
  return { ...render(<CalibrationBlock calibration={calibration} onChange={onChange} />), onChange };
}
async function clickRequest(
  name: string,
  url: string,
  init: RequestInit,
  onChange: ReturnType<typeof vi.fn>,
  fetch: ReturnType<typeof serve>,
) {
  fireEvent.click(button(name));
  await waitFor(() => expect(onChange).toHaveBeenCalledTimes(1));
  expect(fetch).toHaveBeenCalledExactlyOnceWith(url, init);
}

describe("US9-S1: the block and its placement", () => {
  it("shows an empty library, two default forms and the standing hints, with no status or alert", () => {
    block();
    expect(screen.getAllByRole("region", { name: "Calibration" })).toHaveLength(1);
    const region = within(screen.getByRole("region", { name: "Calibration" }));
    for (const words of [
      "Take an imaging frame to see which masters match it.",
      "No master darks or flats yet.",
      "Cover the telescope first. Darks use the exposure, gain, offset, binning and set temperature in force.",
      "Light the flat panel first. The console finds the exposure; the flats' darks follow with the telescope covered.",
    ])
      expect(region.getByText(words).textContent).toBe(words);
    expect((region.getByRole("spinbutton", { name: "Dark frames" }) as HTMLInputElement).value).toBe("20");
    expect((region.getByRole("spinbutton", { name: "Flat frames" }) as HTMLInputElement).value).toBe("20");
    expect((region.getByRole("textbox", { name: "Flat label" }) as HTMLInputElement).value).toBe("");
    expect(button("Take darks").disabled).toBe(false);
    expect(button("Take flats").disabled).toBe(true);
    expect(region.queryByRole("status")).toBeNull();
    expect(region.queryByRole("alert")).toBeNull();
  });

  it.each<ImagingView>([
    READY,
    { state: "off" },
    { state: "no_link" },
    { state: "no_service" },
    { state: "no_camera" },
    { state: "error", error: "cannot reach the INDI server" },
  ])("shows no Calibration for $state without calibration", async (view) => {
    serve(view);
    render(<ImagingPage night={false} onBack={() => {}} />);
    await act(async () => {});
    expect(screen.queryByRole("region", { name: "Calibration" })).toBeNull();
  });

  it("places Calibration after Imaging frame and Focus in a ready view that carries it", async () => {
    serve({ ...READY, focus: { unit: "arcsec", arcsec_per_px: 2.19 }, calibration: { masters: [] } });
    render(<ImagingPage night={false} onBack={() => {}} />);
    const calibration = await screen.findByRole("region", { name: "Calibration" });
    expect(screen.getAllByRole("region", { name: "Calibration" })).toHaveLength(1);
    for (const name of ["Imaging frame", "Focus"]) {
      expect(
        Boolean(
          screen.getByRole("region", { name }).compareDocumentPosition(calibration) & Node.DOCUMENT_POSITION_FOLLOWING,
        ),
      ).toBe(true);
    }
  });
});

describe("US9-S2: the masters", () => {
  it("lists both exact lines and removes the chosen master, then calls onChange", async () => {
    const fetch = serve();
    const { onChange } = block({ masters: [DARK, FLAT] });
    const items = within(screen.getByRole("list")).getAllByRole("listitem");
    expect(items).toHaveLength(2);
    for (const [i, master, line] of [
      [0, DARK, DARK_LINE],
      [1, FLAT, FLAT_LINE],
    ] as const) {
      expect(within(items[i]!).getByText(line).textContent).toBe(line);
      expect(within(items[i]!).getByRole("button", { name: `Remove ${master.id}` })).toBeTruthy();
    }
    await clickRequest(
      `Remove ${DARK.id}`,
      `/api/imaging/calibration/${DARK.id}`,
      { method: "DELETE" },
      onChange,
      fetch,
    );
  });

  it.each([
    [
      { ...DARK, gain: null, offset: null, set_temp_c: null, temp_c: 20, bad_pixels: null },
      "Master dark · 5000 ms · 1 × 1 · sensor 20.0 °C · 20 frames · no bad-pixel map · 2026-01-16",
    ],
    [{ ...FLAT, imaging_angle_deg: null }, FLAT_LINE.split(" · camera at")[0]!],
    [{ ...DARK, checked: false }, `${DARK_LINE} · dark test skipped`],
    [{ ...FLAT, checked: false }, `${FLAT_LINE} · dark test skipped`],
    [
      { ...FLAT, gain: null, offset: null, set_temp_c: null, temp_c: null, imaging_angle_deg: null },
      "Master flat desk · 1 × 1 · 20 frames · 2026-01-16",
    ],
  ] as const)("formats nullable settings and dark-test metadata: %j", (master, line) => {
    block({ masters: [master] });
    expect(within(screen.getByRole("listitem")).getByText(line).textContent).toBe(line);
  });

  it("removes the master's id even while a different set is taking frames", async () => {
    const fetch = serve();
    const { onChange } = block({ masters: [DARK, FLAT], set: { ...SET, state: "frames" } });
    await clickRequest(
      `Remove ${DARK.id}`,
      `/api/imaging/calibration/${DARK.id}`,
      { method: "DELETE" },
      onChange,
      fetch,
    );
  });
});

describe("US9-S3: taking darks and flats", () => {
  it("posts the dark count and calls onChange", async () => {
    const fetch = serve();
    const { onChange } = block();
    fireEvent.change(field("Dark frames"), { target: { value: "5" } });
    await clickRequest("Take darks", "/api/imaging/calibration/darks", post({ count: 5 }), onChange, fetch);
  });

  it("posts the flat count and label and calls onChange", async () => {
    const fetch = serve();
    const { onChange } = block();
    fireEvent.change(field("Flat frames"), { target: { value: "4" } });
    fireEvent.change(field("Flat label"), { target: { value: "desk" } });
    expect(button("Take flats").disabled).toBe(false);
    await clickRequest(
      "Take flats",
      "/api/imaging/calibration/flats",
      post({ count: 4, label: "desk" }),
      onChange,
      fetch,
    );
  });

  it.each(["Take darks", "Take flats", `Remove ${DARK.id}`, "Take the flats' darks"])(
    "shows a refusal unchanged for %s without onChange",
    async (name) => {
      serve({ error: "an exposure is in progress" }, 409);
      const { onChange } = block({ masters: [DARK], ...(name === "Take the flats' darks" ? { set: FLATS } : {}) });
      fireEvent.change(field("Flat label"), { target: { value: "desk" } });
      fireEvent.click(button(name));
      expect((await screen.findByRole("alert")).textContent).toBe("an exposure is in progress");
      expect(onChange).not.toHaveBeenCalled();
    },
  );
});

describe("US9-S4: progress, endings and controls", () => {
  it.each<[Partial<CalibrationSet>, string, boolean]>([
    [{}, "Taking darks: reading the offset level", true],
    [{ state: "frames", taken: 2 }, "Taking darks: 2 of 4 at 200 ms", true],
    [{ kind: "flat", state: "search" }, "Taking flats: finding the exposure", true],
    [{ ...FLATS, state: "frames", taken: 1 }, "Taking flats: 1 of 3 at 366.634 ms", true],
    [{ ...FLATS, state: "flat_darks", darks_taken: 1 }, "Taking the flats' darks: 1 of 3", true],
    [{ state: "done" }, "Master dark saved from 4 darks.", false],
    [{ ...FLATS, state: "done" }, "Master flat desk saved from 3 flats.", false],
    [{ state: "stopped" }, "Darks stopped.", false],
    [{ ...FLATS, state: "stopped" }, "Flats stopped.", false],
  ])("shows %j with its exact status and Stop only while running", async (over, words, running) => {
    const fetch = serve();
    const set = { ...SET, ...over };
    const { onChange } = block({ masters: [], set });
    expect(screen.getByRole("status").textContent).toBe(words);
    expect(screen.queryByRole("alert")).toBeNull();
    fireEvent.change(field("Flat label"), { target: { value: "desk" } });
    expect(button("Take darks").disabled).toBe(running);
    expect(button("Take flats").disabled).toBe(running);
    if (running)
      await clickRequest("Stop", `/api/imaging/calibration/${set.id}`, { method: "DELETE" }, onChange, fetch);
    else expect(screen.queryByRole("button", { name: "Stop" })).toBeNull();
  });

  it.each(["dark", "flat"] as const)("shows a failed %s without an anyway button when not_dark is absent", (kind) => {
    block({ masters: [], set: { ...SET, kind, state: "failed", error: "the camera fell over" } });
    expect(screen.getByRole("alert").textContent).toBe(
      `${kind === "dark" ? "Darks" : "Flats"} stopped: the camera fell over.`,
    );
    expect(screen.queryByRole("status")).toBeNull();
    expect(screen.queryByRole("button", { name: /anyway$/ })).toBeNull();
    expect(screen.queryByRole("button", { name: "Stop" })).toBeNull();
  });

  it("offers a dark retry from not_dark and posts the original count with anyway", async () => {
    const fetch = serve();
    const { onChange } = block({
      masters: [],
      set: { ...SET, state: "failed", not_dark: true, error: "the darks are not dark" },
    });
    expect(screen.getByRole("alert").textContent).toBe("Darks stopped: the darks are not dark.");
    expect(screen.getByText(COVERED).textContent).toBe(COVERED);
    await clickRequest(
      "Take them anyway",
      "/api/imaging/calibration/darks",
      post({ count: 4, anyway: true }),
      onChange,
      fetch,
    );
  });

  it("does not infer not_dark from an error's words", () => {
    block({ masters: [], set: { ...SET, state: "failed", error: "the darks are not dark" } });
    expect(screen.queryByRole("button", { name: /anyway$/ })).toBeNull();
  });

  it.each(["Take the flats' darks", "Drop these flats"])("waits for the cover and sends %s", async (name) => {
    const fetch = serve();
    const { onChange } = block({ masters: [], set: FLATS });
    expect(screen.getByRole("status").textContent).toBe(
      "Flats in hand: 3 at 366.634 ms. Cover the telescope, then take their darks.",
    );
    expect(button("Take the flats' darks")).toBeTruthy();
    expect(button("Drop these flats")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Stop" })).toBeNull();
    expect(screen.queryByRole("alert")).toBeNull();
    fireEvent.change(field("Flat label"), { target: { value: "desk" } });
    expect(button("Take darks").disabled).toBe(true);
    expect(button("Take flats").disabled).toBe(true);
    await clickRequest(
      name,
      name === "Drop these flats" ? "/api/imaging/calibration/flat-x" : "/api/imaging/calibration/flats",
      name === "Drop these flats" ? { method: "DELETE" } : post({ darks: true }),
      onChange,
      fetch,
    );
  });

  it("keeps flats after their darks failed, and offers anyway only from not_dark", async () => {
    const fetch = serve();
    const calibration = { masters: [], set: { ...FLATS, error: "the darks are not dark" } };
    const { onChange, rerender } = block(calibration);
    expect(screen.getByRole("alert").textContent).toBe("The flats' darks stopped: the darks are not dark.");
    expect(screen.queryByRole("button", { name: /anyway$/ })).toBeNull();
    rerender(
      <CalibrationBlock
        calibration={{ ...calibration, set: { ...calibration.set, not_dark: true } }}
        onChange={onChange}
      />,
    );
    expect(screen.getByRole("alert").textContent).toBe("The flats' darks stopped: the darks are not dark.");
    expect(screen.getByText(COVERED).textContent).toBe(COVERED);
    await clickRequest(
      "Take the flats' darks anyway",
      "/api/imaging/calibration/flats",
      post({ darks: true, anyway: true }),
      onChange,
      fetch,
    );
  });
});

describe("US9-S5: matching and warnings", () => {
  const match = { frame: 12, dark: DARK.id, flat: FLAT.id, bad_pixels: 137, missing: [] };
  it("says exactly what the imaging frame would get, including a missing map", () => {
    const { onChange, rerender } = block({ masters: [DARK, FLAT], match });
    const words = "Imaging frame 12 would get: its master dark · the master flat desk · a bad-pixel map of 137";
    expect(screen.getByText(words).textContent).toBe(words);
    expect(screen.queryByRole("alert")).toBeNull();
    rerender(
      <CalibrationBlock
        calibration={{
          masters: [],
          match: { frame: 12, dark: null, flat: null, bad_pixels: null, missing: ["dark", "flat", "bad_pixels"] },
        }}
        onChange={onChange}
      />,
    );
    const missing = "Imaging frame 12 would get: no master dark · no master flat · hot pixels found in the sub itself";
    expect(screen.getByText(missing).textContent).toBe(missing);
  });

  it("warns when the camera has turned and still names the matched masters", () => {
    block({ masters: [DARK, FLAT], match: { ...match, flat_turned_deg: 2.5 } });
    expect(screen.getByRole("alert").textContent).toBe(
      "The camera has turned 2.5° since the flats desk were taken. Take new flats.",
    );
    expect(
      screen.getByText("Imaging frame 12 would get: its master dark · the master flat desk · a bad-pixel map of 137"),
    ).toBeTruthy();
  });

  it("reports the unreadable library's error", () => {
    block({ masters: [], error: "the calibration index could not be read: expected value" });
    expect(screen.getByRole("alert").textContent).toBe(
      "The calibration library is unreadable: the calibration index could not be read: expected value.",
    );
  });
});

it("US9-S6: reads the imaging view immediately after the answer, displays progress and preserves drafts across polls", async () => {
  vi.useFakeTimers();
  let answer!: (value: Response) => void;
  const accepted = new Promise<Response>((resolve) => {
    answer = resolve;
  });
  let taking = false;
  const fetch = vi.fn(async (input: RequestInfo | URL, _init?: RequestInit) => {
    if (String(input) === "/api/imaging/calibration/darks") return accepted;
    return response({
      ...READY,
      calibration: { masters: [], ...(taking ? { set: { ...SET, state: "frames", taken: 2 } } : {}) },
    });
  });
  vi.stubGlobal("fetch", fetch);
  render(<ImagingPage night={false} onBack={() => {}} />);
  await act(async () => {});
  fireEvent.change(field("Dark frames"), { target: { value: "5" } });
  fireEvent.change(field("Flat frames"), { target: { value: "4" } });
  fireEvent.change(field("Flat label"), { target: { value: "desk" } });
  fireEvent.click(button("Take darks"));
  await act(async () => {});
  expect(fetch.mock.calls.map(([url]) => String(url))).toEqual(["/api/imaging", "/api/imaging/calibration/darks"]);
  taking = true;
  await act(async () => {
    answer(response({ masters: [] }));
  });
  expect(fetch.mock.calls.map(([url]) => String(url))).toEqual([
    "/api/imaging",
    "/api/imaging/calibration/darks",
    "/api/imaging",
  ]);
  expect(within(screen.getByRole("region", { name: "Calibration" })).getByRole("status").textContent).toBe(
    "Taking darks: 2 of 4 at 200 ms",
  );
  await act(() => vi.advanceTimersByTimeAsync(1000));
  expect(fetch.mock.calls.filter(([url]) => String(url) === "/api/imaging")).toHaveLength(3);
  expect(field("Dark frames").value).toBe("5");
  expect(field("Flat frames").value).toBe("4");
  expect(field("Flat label").value).toBe("desk");
});
