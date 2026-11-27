// The Imaging page's last picture and the Focus block's readings and altitudes (054 US4).
// Oracle: the words, roles, sources and states of US4-S1 to US4-S4, compared exactly; each
// size is the view's own, written to two decimals, and each percentage the scenario's worked
// example.
import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ImagingFocus, ImagingFocusAid, ImagingFrame, ImagingSettings, ImagingView } from "../api";
import { FocusBlock } from "./FocusBlock";
import { ImagingPage } from "./ImagingPage";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
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

const SETTINGS: ImagingSettings = {
  exposure_ms: 1000,
  gain: 120,
  offset: 10,
  binning: 1,
  roi: { x: 0, y: 0, width: 1236, height: 822 },
  cooler_on: false,
  set_temp_c: null,
  forward_kib_per_s: null,
  save_at_node: false,
};

// The page rig of US4-S1: the simulated imaging camera, ready, with its settings in force.
const READY: ImagingView = {
  state: "ready",
  camera: CAMERA,
  status: { temp_c: 20.0, cooler_power: 0.0, set_temp_c: null, exposing: false },
  settings: SETTINGS,
};

const page = (frame: ImagingFrame): ImagingView => ({ ...READY, frame });

const frame = (over: Partial<ImagingFrame>): ImagingFrame => ({
  id: 1,
  state: "saved",
  purpose: "frame",
  exposure_ms: 1000,
  ...over,
});

const preview = (id: number) => `/api/imaging/frames/${id}/preview.jpg`;

describe("the Imaging page's last picture (054 US4-S1)", () => {
  it("keeps the last picture until the next one is ready, and says that it is the last", async () => {
    let answer: ImagingView = page(frame({ id: 1, preview: preview(1) }));
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response(JSON.stringify(answer))),
    );
    render(<ImagingPage night={false} onBack={() => {}} />);
    const region = () => within(screen.getByRole("region", { name: "Imaging frame" }));
    await waitFor(() =>
      expect(region().getByRole("img", { name: "Imaging frame 1" }).getAttribute("src")).toBe(preview(1)),
    );

    // Frame 2 is exposing and has no preview yet: frame 1 stays, and says it is the last picture.
    answer = page(frame({ id: 2, state: "exposing" }));
    await waitFor(() => expect(region().getByRole("status").textContent).toBe("Exposing for 1000 ms"), {
      timeout: 3000,
    });
    expect(region().getByRole("img", { name: "Imaging frame 1" }).getAttribute("src")).toBe(preview(1));
    expect(region().getByText("Frame 1, the last picture")).toBeTruthy();

    // Frame 2's preview is ready: it replaces frame 1, and no text calls frame 1 the last picture.
    answer = page(frame({ id: 2, preview: preview(2) }));
    await waitFor(
      () => expect(region().getByRole("img", { name: "Imaging frame 2" }).getAttribute("src")).toBe(preview(2)),
      { timeout: 3000 },
    );
    expect(region().queryByRole("img", { name: "Imaging frame 1" })).toBeNull();
    expect(screen.queryAllByText(/the last picture/)).toHaveLength(0);
  });
});

// The unit arcsec at 0.55″ a pixel, as the scenarios give it.
const FOCUS_RIG: Pick<ImagingFocus, "unit" | "arcsec_per_px"> = { unit: "arcsec", arcsec_per_px: 0.55 };

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

const focusView = (aidOver: Partial<ImagingFocusAid>, focusOver: Partial<ImagingFocus> = {}): ImagingView => ({
  state: "ready",
  focus: { ...FOCUS_RIG, aid: aid(aidOver), ...focusOver },
});

const focusRegion = () => screen.getByRole("region", { name: "Focus" });

describe("the Focus block's readings while the aid finds its region again (054 US4-S2)", () => {
  it("shows the settled reading and its line while finding with readings, and none with none", () => {
    const withReadings = render(
      <FocusBlock
        view={focusView({
          state: "finding",
          readings: 9,
          median: { value: 6.7, readings: 3 },
          last: { frame_id: 12, at: 2, state: "reading", value: 6.7, stars: 12 },
          best: 6.7,
          history: [6.7],
        })}
        refresh={vi.fn()}
      />,
    );
    expect(within(focusRegion()).getByText("6.70″")).toBeTruthy();
    expect(within(focusRegion()).getByText("Finding the focus region on a full imaging frame")).toBeTruthy();
    withReadings.unmount();

    render(<FocusBlock view={focusView({ state: "finding", readings: 0, history: [] })} refresh={vi.fn()} />);
    expect(within(focusRegion()).queryByText("Focus reading")).toBeNull();
  });
});

describe("the Focus block's altitudes (054 US4-S3 and US4-S4)", () => {
  it("names the record's altitude, and the verdict's record carried to the reading's altitude", () => {
    render(
      <FocusBlock
        view={focusView(
          {
            median: { value: 6.7, readings: 3 },
            last: { frame_id: 12, at: 2, state: "reading", value: 6.7, stars: 12, alt_deg: 37 },
            verdict: "slightly_soft",
            reference: {
              value: 5.98,
              of: "tonight",
              carried: { from: 4.6, from_alt_deg: 74, alt_deg: 37 },
            },
            history: [6.7],
          },
          { record: { tonight: { value: 4.6, at: 1, alt_deg: 74 } } },
        )}
        refresh={vi.fn()}
      />,
    );
    expect(within(focusRegion()).getByText("Focus record: 4.60″ at 74° tonight.")).toBeTruthy();
    expect(
      within(focusRegion()).getByText(
        "Slightly soft: 12% larger than the focus record of 4.60″ at 74° tonight, as it reads at 37°: 5.98″.",
      ),
    ).toBeTruthy();
  });

  it("names a record with no altitude as such while the reading has one, and judges it as read", () => {
    render(
      <FocusBlock
        view={focusView(
          {
            median: { value: 6.7, readings: 3 },
            last: { frame_id: 12, at: 2, state: "reading", value: 6.7, stars: 12, alt_deg: 37 },
            verdict: "clearly_out",
            reference: { value: 4.6, of: "tonight" },
            history: [6.7],
          },
          { record: { tonight: { value: 4.6, at: 1 } } },
        )}
        refresh={vi.fn()}
      />,
    );
    expect(within(focusRegion()).getByText("Focus record: 4.60″ with no altitude tonight.")).toBeTruthy();
    expect(
      within(focusRegion()).getByText("Clearly out: 46% larger than the focus record of 4.60″ tonight."),
    ).toBeTruthy();
  });
});
