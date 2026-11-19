// The Focus block (033 US6). Oracle: the stated names, roles, words, requests and
// bars of US6-S1 to US6-S5, compared exactly; each size is the view's own, written
// to two decimals, and each percentage the scenario's worked example.
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ImagingFocus, ImagingFocusAid, ImagingSettings, ImagingView } from "../api";
import { FocusBlock } from "./FocusBlock";
import { ImagingPage } from "./ImagingPage";

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

const STANDING =
  "A focus reading is the size of the stars in the focus region: smaller is sharper. The console judges it " +
  "against this camera's own focus record and never against a fixed number, because seeing, collimation and a " +
  "warm mirror enlarge stars too. Use a field of stars away from any bright nebula.";
const SPARSE = "Try a longer Focus frame time, or a field with more stars.";

const focus = (over: Partial<ImagingFocus> = {}): ImagingFocus => ({ unit: "arcsec", arcsec_per_px: 2.19, ...over });

const aid = (over: Partial<ImagingFocusAid> = {}): ImagingFocusAid => ({
  state: "running",
  check: false,
  since: 0,
  exposure_ms: 1000,
  readings: 9,
  found: false,
  history: [],
  ...over,
});

const view = (over: Partial<ImagingView> = {}): ImagingView => ({ state: "ready", focus: focus(), ...over });

const region = (over: Partial<NonNullable<ImagingFocus["region"]>> = {}): NonNullable<ImagingFocus["region"]> => ({
  col: 2,
  row: 0,
  x: 824,
  y: 0,
  width: 412,
  height: 274,
  source: "found",
  ...over,
});

// The running aid of US6-S3, with its region and record.
const S3_AID: Partial<ImagingFocusAid> = {
  last: { frame_id: 12, at: 2, state: "reading", value: 5.74, stars: 12 },
  median: { value: 5.7046, readings: 3 },
  best: 5.6512,
  trend: "better",
  verdict: "good",
  reference: { value: 5.7, of: "tonight" },
  history: [5.7, 5.74, 5.65],
};
const s3 = (aidOver: Partial<ImagingFocusAid> = {}, focusOver: Partial<ImagingFocus> = {}): ImagingFocus =>
  focus({
    region: region(),
    record: { tonight: { value: 5.7, at: 1 }, earlier: { value: 5.41, at: 0 } },
    aid: aid({ ...S3_AID, ...aidOver }),
    ...focusOver,
  });

const CAMERA = {
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
};

const settings = (roi: ImagingSettings["roi"]): ImagingSettings => ({
  exposure_ms: 1000,
  gain: 120,
  offset: 10,
  binning: 1,
  roi,
  cooler_on: false,
  set_temp_c: null,
  forward_kib_per_s: null,
  save_at_node: false,
});

const WHOLE = { x: 0, y: 0, width: 1236, height: 822 };

const open = (f: ImagingFocus, over: Partial<ImagingView> = {}) => {
  render(<FocusBlock view={view({ focus: f, ...over })} refresh={vi.fn()} />);
  return screen.getByRole("region", { name: "Focus" });
};

const statuses = (scope: HTMLElement) =>
  within(scope)
    .getAllByRole("status")
    .map((p) => p.textContent);
const alerts = (scope: HTMLElement) =>
  within(scope)
    .getAllByRole("alert")
    .map((p) => p.textContent);

describe("the Focus block on the Imaging page", () => {
  const page = (withFocus: boolean): ImagingView => ({
    state: "ready",
    camera: CAMERA,
    status: { temp_c: 20.0, cooler_power: 0.0, set_temp_c: null, exposing: false },
    settings: settings(WHOLE),
    frame: { id: 3, state: "saved", purpose: "frame", exposure_ms: 2000, file: "frames/2026-01-16/000003.fits" },
    ...(withFocus ? { focus: focus() } : {}),
  });

  it("shows a Focus region after Imaging solve, with the standing words, the select, the field and the two buttons", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response(JSON.stringify(page(true)))),
    );
    render(<ImagingPage night={false} onBack={() => {}} />);
    const solveRegion = await screen.findByRole("region", { name: "Imaging solve" });
    const focusRegion = screen.getByRole("region", { name: "Focus" });
    expect(Boolean(solveRegion.compareDocumentPosition(focusRegion) & Node.DOCUMENT_POSITION_FOLLOWING)).toBe(true);
    expect(within(focusRegion).getByText(STANDING)).toBeTruthy();
    const select = within(focusRegion).getByRole("combobox", { name: "Focus region" }) as HTMLSelectElement;
    expect(Array.from(select.options).map((o) => o.textContent)).toEqual([
      "Kept, or the sharpest tile",
      "Find the sharpest tile again",
      "Top left",
      "Top centre",
      "Top right",
      "Middle left",
      "Centre",
      "Middle right",
      "Bottom left",
      "Bottom centre",
      "Bottom right",
    ]);
    expect((within(focusRegion).getByRole("textbox", { name: "Focus frame time" }) as HTMLInputElement).value).toBe(
      "1000",
    );
    expect(within(focusRegion).getByRole("button", { name: "Start focus aid" })).toBeTruthy();
    expect(within(focusRegion).getByRole("button", { name: "Check focus" })).toBeTruthy();
    expect(within(focusRegion).queryByRole("status")).toBeNull();
    expect(within(focusRegion).queryByRole("alert")).toBeNull();
    expect(within(focusRegion).queryByRole("button", { name: "Stop focus aid" })).toBeNull();
  });

  it("shows no Focus region without a focus object", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response(JSON.stringify(page(false)))),
    );
    render(<ImagingPage night={false} onBack={() => {}} />);
    await screen.findByRole("region", { name: "Imaging solve" });
    expect(screen.queryByRole("region", { name: "Focus" })).toBeNull();
  });
});

describe("the Focus block's requests", () => {
  it("starts the focus aid with the frame time, then refreshes", async () => {
    const fetch = vi.fn(async () => new Response(JSON.stringify(view())));
    vi.stubGlobal("fetch", fetch);
    const refresh = vi.fn();
    render(<FocusBlock view={view()} refresh={refresh} />);
    fireEvent.click(screen.getByRole("button", { name: "Start focus aid" }));
    await waitFor(() => expect(refresh).toHaveBeenCalled());
    expect(fetch).toHaveBeenCalledWith("/api/imaging/focus", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: '{"exposure_ms":1000}',
    });
  });

  it.each([
    ["the sharpest tile again", "find", "2000", '{"exposure_ms":2000,"find":true}'],
    ["the top right tile", "2,0", "1000", '{"exposure_ms":1000,"tile":{"col":2,"row":0}}'],
    ["the bottom left tile", "0,2", "1000", '{"exposure_ms":1000,"tile":{"col":0,"row":2}}'],
  ])("starts on %s", async (_label, option, time, body) => {
    const fetch = vi.fn(async () => new Response(JSON.stringify(view())));
    vi.stubGlobal("fetch", fetch);
    const refresh = vi.fn();
    render(<FocusBlock view={view()} refresh={refresh} />);
    fireEvent.change(screen.getByRole("combobox", { name: "Focus region" }), { target: { value: option } });
    fireEvent.change(screen.getByRole("textbox", { name: "Focus frame time" }), { target: { value: time } });
    fireEvent.click(screen.getByRole("button", { name: "Start focus aid" }));
    await waitFor(() => expect(refresh).toHaveBeenCalled());
    expect(fetch).toHaveBeenCalledWith("/api/imaging/focus", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body,
    });
  });

  it("checks focus with the frame time and check", async () => {
    const fetch = vi.fn(async () => new Response(JSON.stringify(view())));
    vi.stubGlobal("fetch", fetch);
    const refresh = vi.fn();
    render(<FocusBlock view={view()} refresh={refresh} />);
    fireEvent.click(screen.getByRole("button", { name: "Check focus" }));
    await waitFor(() => expect(refresh).toHaveBeenCalled());
    expect(fetch).toHaveBeenCalledWith("/api/imaging/focus", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: '{"exposure_ms":1000,"check":true}',
    });
  });

  it("shows a refusal as an alert and calls no refresh", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response(JSON.stringify({ error: "this console has no imaging solver" }), { status: 409 })),
    );
    const refresh = vi.fn();
    render(<FocusBlock view={view()} refresh={refresh} />);
    fireEvent.click(screen.getByRole("button", { name: "Start focus aid" }));
    expect((await screen.findByRole("alert")).textContent).toBe("this console has no imaging solver");
    expect(refresh).not.toHaveBeenCalled();
  });

  it("stops the running aid with DELETE, hiding every idle control", async () => {
    const fetch = vi.fn(async () => new Response(JSON.stringify(view())));
    vi.stubGlobal("fetch", fetch);
    const refresh = vi.fn();
    render(
      <FocusBlock
        view={view({
          focus: focus({ aid: aid() }),
          camera: CAMERA,
          settings: settings({ x: 412, y: 274, width: 412, height: 274 }),
        })}
        refresh={refresh}
      />,
    );
    const focusRegion = screen.getByRole("region", { name: "Focus" });
    expect(within(focusRegion).getByRole("button", { name: "Stop focus aid" })).toBeTruthy();
    expect(within(focusRegion).queryByRole("combobox")).toBeNull();
    expect(within(focusRegion).queryByRole("textbox")).toBeNull();
    expect(within(focusRegion).queryByRole("button", { name: "Start focus aid" })).toBeNull();
    expect(within(focusRegion).queryByRole("button", { name: "Check focus" })).toBeNull();
    expect(within(focusRegion).queryByRole("button", { name: "Use the whole sensor" })).toBeNull();
    fireEvent.click(within(focusRegion).getByRole("button", { name: "Stop focus aid" }));
    await waitFor(() => expect(refresh).toHaveBeenCalled());
    expect(fetch).toHaveBeenCalledWith("/api/imaging/focus", { method: "DELETE" });
  });
});

describe("the Focus block's reading", () => {
  it("shows the reading in the Display size, with the status, best, trend, turning line, verdict, record and region", () => {
    const focusRegion = open(s3());
    expect(within(focusRegion).getByText("Focus reading")).toBeTruthy();
    const reading = within(focusRegion).getByText("5.70″");
    expect(reading.className).toContain("text-[2.35rem]");
    expect(reading.className).toContain("sm:text-[2.75rem]");
    expect(statuses(focusRegion)).toContain("Median of the last 3 imaging frames · 12 stars");
    expect(within(focusRegion).getByText("Best so far 5.65″")).toBeTruthy();
    expect(within(focusRegion).getByText("Sharper than before the last change")).toBeTruthy();
    expect(
      within(focusRegion).getByText(
        "Turn the focuser one way until the reading has passed its lowest and risen again, then come back to the lowest.",
      ),
    ).toBeTruthy();
    expect(within(focusRegion).getByText("Within 10% of the focus record of 5.70″ tonight.")).toBeTruthy();
    expect(within(focusRegion).getByText("Focus record: 5.70″ tonight · 5.41″ on earlier nights.")).toBeTruthy();
    expect(
      within(focusRegion).getByText("Focus region: the top right tile, found from an imaging frame."),
    ).toBeTruthy();
  });

  it("reads a passed-through aid without the turning line", () => {
    const focusRegion = open(s3({ found: true }));
    expect(within(focusRegion).getByText("Passed through focus and came back to the best so far.")).toBeTruthy();
    expect(within(focusRegion).queryByText(/^Turn the focuser/)).toBeNull();
  });

  it.each([
    ["worse", "Softer than before the last change"],
    ["same", "About the same as before the last change"],
  ] as const)("reads the trend %s", (trend, line) => {
    const focusRegion = open(s3({ trend }));
    expect(within(focusRegion).getByText(line)).toBeTruthy();
  });

  it("shows no trend line without a trend", () => {
    const focusRegion = open(s3({ trend: undefined }));
    expect(within(focusRegion).queryByText(/before the last change/)).toBeNull();
  });

  it("counts the readings held while the median is not full", () => {
    const focusRegion = open(s3({ median: { value: 5.7046, readings: 2 } }));
    expect(statuses(focusRegion)).toContain("2 of 3 imaging frames since the last change · 12 stars");
  });

  it.each([
    ["slightly_soft", 6.5, "Slightly soft: 14% larger than the focus record of 5.70″ from earlier nights."],
    ["clearly_out", 7.68, "Clearly out: 35% larger than the focus record of 5.70″ from earlier nights."],
  ] as const)("reads the verdict %s against earlier nights", (verdict, value, line) => {
    const focusRegion = open(s3({ verdict, reference: { value: 5.7, of: "earlier" }, median: { value, readings: 3 } }));
    expect(within(focusRegion).getByText(line)).toBeTruthy();
  });

  it("reads the verdict no_record", () => {
    const focusRegion = open(s3({ verdict: "no_record", reference: undefined }));
    expect(
      within(focusRegion).getByText(
        "No focus record for this camera yet. Run the focus aid through focus to make one.",
      ),
    ).toBeTruthy();
  });

  it("reads sizes in sensor pixels without a scale", () => {
    const focusRegion = open(s3({}, { unit: "px", arcsec_per_px: undefined }));
    expect(within(focusRegion).getByText("5.70 px")).toBeTruthy();
    expect(within(focusRegion).getByText("Best so far 5.65 px")).toBeTruthy();
    expect(
      within(focusRegion).getByText(
        "No imaging scale is known yet, so sizes are in sensor pixels. Solve an imaging frame, or start the console with --focal-length-mm.",
      ),
    ).toBeTruthy();
  });

  it("reads a record of earlier nights alone", () => {
    const focusRegion = open(s3({}, { record: { earlier: { value: 5.41, at: 0 } } }));
    expect(within(focusRegion).getByText("Focus record: 5.41″ on earlier nights.")).toBeTruthy();
  });

  it("reads a region fixed by hand", () => {
    const focusRegion = open(s3({}, { region: region({ col: 1, row: 1, x: 412, y: 274, source: "fixed" }) }));
    expect(within(focusRegion).getByText("Focus region: the centre tile, set by hand.")).toBeTruthy();
  });

  it("reads a provisional region as that and never as found or set by hand", () => {
    const focusRegion = open(s3({}, { region: region({ col: 1, row: 1, x: 412, y: 274, source: "provisional" }) }));
    expect(
      within(focusRegion).getByText(
        "Focus region: the centre tile for now. The sharpest tile is found once the stars are small enough to measure.",
      ),
    ).toBeTruthy();
    expect(within(focusRegion).queryByText(/set by hand/)).toBeNull();
    expect(within(focusRegion).queryByText(/found from an imaging frame/)).toBeNull();
  });
});

describe("the Focus block's states", () => {
  it.each([
    ["starting", "Starting the focus aid"],
    ["finding", "Finding the focus region on a full imaging frame"],
  ] as const)("reads the state %s", (state, line) => {
    const focusRegion = open(focus({ aid: aid({ state }) }));
    expect(statuses(focusRegion)).toContain(line);
    expect(within(focusRegion).getByRole("button", { name: "Stop focus aid" })).toBeTruthy();
  });

  it("waits for the first imaging frame", () => {
    const focusRegion = open(
      focus({
        aid: aid({
          readings: 0,
          last: undefined,
          median: undefined,
          best: undefined,
          trend: undefined,
          verdict: undefined,
          reference: undefined,
        }),
      }),
    );
    expect(within(focusRegion).getByText("—")).toBeTruthy();
    expect(statuses(focusRegion)).toContain("Waiting for the first imaging frame");
  });

  it("reads a settling frame", () => {
    const focusRegion = open(focus({ aid: aid({ last: { frame_id: 12, at: 2, state: "settling" } }) }));
    expect(within(focusRegion).getByText("—")).toBeTruthy();
    expect(statuses(focusRegion)).toContain("Settling after a change");
  });

  it("reads a frame with too few stars, with the cure and the verdict", () => {
    const focusRegion = open(
      focus({
        aid: aid({
          last: { frame_id: 12, at: 2, state: "too_few_stars", stars: 2 },
          verdict: "unmeasurable",
        }),
      }),
    );
    expect(statuses(focusRegion)).toContain("Not measurable: 2 stars in the focus region, and 5 are needed");
    expect(within(focusRegion).getByText(SPARSE)).toBeTruthy();
    expect(within(focusRegion).getByText("Not measurable: too few stars in the focus region.")).toBeTruthy();
  });

  it("reads a far-out frame, with the rough size and the verdict", () => {
    const focusRegion = open(
      focus({
        aid: aid({
          last: { frame_id: 12, at: 2, state: "far_out", value: 22.3, stars: 9 },
          verdict: "clearly_out",
        }),
      }),
    );
    expect(statuses(focusRegion)).toContain("Far out of focus: stars about 22.30″ across");
    expect(within(focusRegion).getByText("Clearly out: the stars are too large to measure.")).toBeTruthy();
  });

  it("reads an aid stopped by hand, with the two start buttons", () => {
    const focusRegion = open(focus({ aid: aid({ state: "ended", ended_reason: "stopped" }) }));
    expect(statuses(focusRegion)).toContain("Focus aid stopped.");
    expect(within(focusRegion).getByRole("button", { name: "Start focus aid" })).toBeTruthy();
    expect(within(focusRegion).getByRole("button", { name: "Check focus" })).toBeTruthy();
  });

  it("reads a finished check, with its verdict and no turning line", () => {
    const focusRegion = open(s3({ state: "ended", ended_reason: "done" }));
    expect(statuses(focusRegion)).toContain("Focus check done.");
    expect(within(focusRegion).getByText("Within 10% of the focus record of 5.70″ tonight.")).toBeTruthy();
    expect(within(focusRegion).queryByText(/^Turn the focuser/)).toBeNull();
  });

  it("reads a finished check that did not settle", () => {
    const focusRegion = open(
      focus({
        aid: aid({
          state: "ended",
          ended_reason: "done",
          verdict: "unmeasurable",
          last: { frame_id: 12, at: 2, state: "reading", value: 5.74, stars: 12 },
        }),
      }),
    );
    expect(within(focusRegion).getByText("Not measurable: the reading did not settle.")).toBeTruthy();
  });

  it("reads a failed aid as an alert, without the sparse-field line", () => {
    const focusRegion = open(
      focus({
        aid: aid({
          state: "ended",
          ended_reason: "failed",
          error: "the imaging solver gave no answer within 30 s",
        }),
      }),
    );
    expect(alerts(focusRegion)).toContain("Focus aid stopped: the imaging solver gave no answer within 30 s.");
    expect(within(focusRegion).queryByText(SPARSE)).toBeNull();
  });

  it("reads a failure to put the settings back as a second alert", () => {
    const focusRegion = open(
      focus({
        aid: aid({
          state: "ended",
          ended_reason: "failed",
          error: "the imaging solver gave no answer within 30 s",
          restore_error: "the scope node is not connected",
        }),
      }),
    );
    expect(alerts(focusRegion)).toEqual([
      "Focus aid stopped: the imaging solver gave no answer within 30 s.",
      "The camera's settings were not put back: the scope node is not connected.",
    ]);
  });

  it("reads a no_region ending, with the sparse-field line", () => {
    const focusRegion = open(
      focus({
        aid: aid({
          state: "ended",
          ended_reason: "no_region",
          error: "no tile of the imaging frame had five stars to measure",
        }),
      }),
    );
    expect(alerts(focusRegion)).toContain("Focus aid stopped: no tile of the imaging frame had five stars to measure.");
    expect(within(focusRegion).getByText(SPARSE)).toBeTruthy();
  });

  it("offers the whole sensor while idle on a region of it", async () => {
    const fetch = vi.fn(async () => new Response(JSON.stringify(view())));
    vi.stubGlobal("fetch", fetch);
    const refresh = vi.fn();
    render(
      <FocusBlock
        view={view({ camera: CAMERA, settings: settings({ x: 412, y: 274, width: 412, height: 274 }) })}
        refresh={refresh}
      />,
    );
    const focusRegion = screen.getByRole("region", { name: "Focus" });
    expect(within(focusRegion).getByText("The camera is reading a region of its sensor.")).toBeTruthy();
    fireEvent.click(within(focusRegion).getByRole("button", { name: "Use the whole sensor" }));
    await waitFor(() => expect(refresh).toHaveBeenCalled());
    expect(fetch).toHaveBeenCalledWith("/api/imaging/settings", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: '{"roi":null}',
    });
  });

  it("offers the whole sensor only while the region is not the whole sensor", () => {
    const focusRegion = open(focus(), { camera: CAMERA, settings: settings(WHOLE) });
    expect(within(focusRegion).queryByText("The camera is reading a region of its sensor.")).toBeNull();
    expect(within(focusRegion).queryByRole("button", { name: "Use the whole sensor" })).toBeNull();
  });
});

describe("the Focus block's history", () => {
  it("draws one currentColor bar a measured frame, 36 units tall at the largest", () => {
    const focusRegion = open(focus({ aid: aid({ history: [2, 4, null, 3] }) }));
    const chart = within(focusRegion).getByRole("img", { name: "Focus readings of the last 4 imaging frames" });
    expect(chart.getAttribute("data-focus-history")).not.toBeNull();
    expect(chart.getAttribute("viewBox")).toBe("0 0 16 40");
    expect(chart.getAttribute("height")).toBe("40");
    const bars = Array.from(chart.querySelectorAll("rect"));
    expect(bars.map((b) => b.getAttribute("x"))).toEqual(["0", "4", "12"]);
    expect(bars.map((b) => b.getAttribute("height"))).toEqual(["18.0", "36.0", "27.0"]);
    expect(bars.map((b) => b.getAttribute("y"))).toEqual(["22.0", "4.0", "13.0"]);
    expect(bars.every((b) => b.getAttribute("fill") === "currentColor")).toBe(true);
  });

  it("holds the last 60 of a longer history", () => {
    const focusRegion = open(focus({ aid: aid({ history: Array.from({ length: 70 }, (_, i) => i + 1) }) }));
    const chart = within(focusRegion).getByRole("img", { name: "Focus readings of the last 60 imaging frames" });
    expect(chart.querySelectorAll("rect")).toHaveLength(60);
  });

  it("sets no style and no fill or stroke but currentColor or none", () => {
    const focusRegion = open(focus({ aid: aid({ history: [2, 4, null, 3] }) }));
    for (const el of Array.from(focusRegion.querySelectorAll("*"))) {
      expect(el.getAttribute("style")).toBeNull();
      for (const attr of ["fill", "stroke"]) {
        const value = el.getAttribute(attr);
        if (value != null) expect(["currentColor", "none"]).toContain(value);
      }
    }
  });
});
