// The Imaging solve block (032 US6). Oracle: the stated names, roles, words and
// request of US6-S1 to US6-S4, compared exactly; each number is the view's own,
// written to the stated count of decimals.
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ImagingFrame, ImagingRelation, ImagingSolve, ImagingView } from "../api";
import { ImagingPage } from "./ImagingPage";
import { ImagingSolveBlock } from "./ImagingSolveBlock";

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

// The view of a ready imaging camera with frame 3 saved and nothing solved.
const frame = (over: Partial<ImagingFrame> = {}): ImagingFrame => ({
  id: 3,
  state: "saved",
  purpose: "frame",
  exposure_ms: 2000,
  ...over,
});

const view = (over: Partial<ImagingView> = {}): ImagingView => ({ state: "ready", frame: frame(), ...over });

const solve = (over: Partial<ImagingSolve> = {}): ImagingSolve => ({
  frame_id: 3,
  state: "solving",
  requested_at: 1768528800,
  solver: "simulated",
  hint: { source: "none", radius_deg: 180 },
  ...over,
});

const relation = (over: Partial<ImagingRelation> = {}): ImagingRelation => ({ ...over });

describe("the Imaging solve block's button", () => {
  it("solves the saved frame with one POST, then refreshes", async () => {
    const fetch = vi.fn(async () => new Response(JSON.stringify(view())));
    vi.stubGlobal("fetch", fetch);
    const refresh = vi.fn();
    render(<ImagingSolveBlock view={view()} refresh={refresh} />);
    const region = screen.getByRole("region", { name: "Imaging solve" });
    const button = within(region).getByRole("button", { name: "Solve imaging frame" }) as HTMLButtonElement;
    expect(button.disabled).toBe(false);
    expect(within(region).queryByRole("status")).toBeNull();
    expect(within(region).queryByRole("alert")).toBeNull();
    fireEvent.click(button);
    await waitFor(() => expect(refresh).toHaveBeenCalled());
    expect(fetch).toHaveBeenCalledWith("/api/imaging/solve", { method: "POST" });
  });

  it("shows a refusal as an alert and calls no refresh", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response(JSON.stringify({ error: "this console has no imaging solver" }), { status: 409 })),
    );
    const refresh = vi.fn();
    render(<ImagingSolveBlock view={view()} refresh={refresh} />);
    fireEvent.click(screen.getByRole("button", { name: "Solve imaging frame" }));
    expect((await screen.findByRole("alert")).textContent).toBe("this console has no imaging solver");
    expect(refresh).not.toHaveBeenCalled();
  });

  it.each([
    ["no frame", view({ frame: undefined })],
    ["an exposing frame", view({ frame: frame({ state: "exposing" }) })],
  ])("keeps the button disabled with %s", (_label, rendered) => {
    render(<ImagingSolveBlock view={rendered} refresh={vi.fn()} />);
    expect((screen.getByRole("button", { name: "Solve imaging frame" }) as HTMLButtonElement).disabled).toBe(true);
  });
});

describe("the Imaging solve block's solve", () => {
  it("reads the solving state, with the button disabled", () => {
    render(<ImagingSolveBlock view={view({ solve: solve() })} refresh={vi.fn()} />);
    const region = screen.getByRole("region", { name: "Imaging solve" });
    expect(within(region).getByRole("status").textContent).toBe("Solving imaging frame 3");
    expect((within(region).getByRole("button", { name: "Solve imaging frame" }) as HTMLButtonElement).disabled).toBe(
      true,
    );
  });

  it("reads the solved frame exactly", () => {
    render(
      <ImagingSolveBlock
        view={view({
          solve: solve({
            state: "solved",
            ra_deg: 83.8221,
            dec_deg: -5.3911,
            up_east_of_north_deg: 30.04,
            arcsec_per_px: 2.16,
            mirrored: false,
            stars: 118,
          }),
        })}
        refresh={vi.fn()}
      />,
    );
    expect(screen.getByRole("status").textContent).toBe(
      "Solved imaging frame 3: RA 83.8221° Dec -5.3911° · up 30.0° east of north · 2.16″ per pixel · 118 stars",
    );
  });

  it("reads the solved frame without a star count", () => {
    render(
      <ImagingSolveBlock
        view={view({
          solve: solve({
            state: "solved",
            ra_deg: 83.8221,
            dec_deg: -5.3911,
            up_east_of_north_deg: 30.04,
            arcsec_per_px: 2.16,
          }),
        })}
        refresh={vi.fn()}
      />,
    );
    expect(screen.getByRole("status").textContent).toBe(
      "Solved imaging frame 3: RA 83.8221° Dec -5.3911° · up 30.0° east of north · 2.16″ per pixel",
    );
  });

  it("reads the mirrored solved frame", () => {
    render(
      <ImagingSolveBlock
        view={view({
          solve: solve({
            state: "solved",
            ra_deg: 83.8221,
            dec_deg: -5.3911,
            up_east_of_north_deg: 30.04,
            arcsec_per_px: 2.16,
            mirrored: true,
            stars: 118,
          }),
        })}
        refresh={vi.fn()}
      />,
    );
    expect(screen.getByRole("status").textContent).toBe(
      "Solved imaging frame 3: RA 83.8221° Dec -5.3911° · up 30.0° east of north · 2.16″ per pixel · 118 stars · mirrored",
    );
  });

  it("reads the failed solve as an alert, with the button enabled", () => {
    render(
      <ImagingSolveBlock
        view={view({ solve: solve({ state: "failed", error: "the imaging solver found no solution" }) })}
        refresh={vi.fn()}
      />,
    );
    const region = screen.getByRole("region", { name: "Imaging solve" });
    expect(within(region).getByRole("alert").textContent).toBe(
      "Imaging solve failed: the imaging solver found no solution.",
    );
    expect((within(region).getByRole("button", { name: "Solve imaging frame" }) as HTMLButtonElement).disabled).toBe(
      false,
    );
  });
});

describe("the Imaging solve block's relation", () => {
  const open = (over: Partial<ImagingRelation>) => {
    render(<ImagingSolveBlock view={view({ relation: relation(over) })} refresh={vi.fn()} />);
    return screen.getByRole("region", { name: "Imaging solve" });
  };

  it("reads the kept scale", () => {
    const region = open({ scale: { arcsec_per_px: 2.1846, frame_id: 3 } });
    expect(within(region).getByText("Scale 2.185″ per sensor pixel, from imaging frame 3.")).toBeTruthy();
    expect(within(region).queryByRole("status")).toBeNull();
    expect(within(region).queryByRole("alert")).toBeNull();
  });

  it("reads the measured aim point", () => {
    const region = open({ aim: { x: 700.12, y: 419.84 }, angle_deg: 17.04, readings: 3 });
    expect(
      within(region).getByText(
        "Aim point measured at (700.1, 419.8) px of the finder camera's frame · imaging angle 17.0° · from 3 imaging solves.",
      ),
    ).toBeTruthy();
    expect(within(region).queryByRole("status")).toBeNull();
    expect(within(region).queryByRole("alert")).toBeNull();
  });

  it.each([
    ["moving", "the telescope was moving."],
    ["no_live_pointing", "no live pointing shared its moment."],
    ["off_frame", "its centre is outside the finder camera's frame."],
    ["set_aside", "it disagreed with the earlier ones and was set aside."],
  ] as const)("reads the last outcome %s", (outcome, why) => {
    const region = open({ last: { frame_id: 3, outcome } });
    expect(within(region).getByText(`The last imaging solve did not measure the aim point: ${why}`)).toBeTruthy();
    expect(within(region).queryByRole("status")).toBeNull();
    expect(within(region).queryByRole("alert")).toBeNull();
  });

  it.each(["measured", "replaced"] as const)("shows no paragraph for the outcome %s", (outcome) => {
    const region = open({ last: { frame_id: 3, outcome } });
    expect(within(region).queryByText(/^The last imaging solve did not measure/)).toBeNull();
  });

  it("reads the held aim point", () => {
    const region = open({ held: true });
    expect(within(region).getByText("The measured aim point waits until Track is off.")).toBeTruthy();
    expect(within(region).queryByRole("status")).toBeNull();
    expect(within(region).queryByRole("alert")).toBeNull();
  });

  it("shows no held paragraph without one held", () => {
    const region = open({ scale: { arcsec_per_px: 2.1846, frame_id: 3 } });
    expect(within(region).queryByText("The measured aim point waits until Track is off.")).toBeNull();
  });
});

describe("the Imaging page's solve block", () => {
  it("places the Imaging solve region after the Imaging frame region", async () => {
    const page: ImagingView = {
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
      status: { temp_c: 20.0, cooler_power: 0.0, set_temp_c: null, exposing: false },
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
      frame: frame({ file: "frames/2026-01-16/000003.fits" }),
    };
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response(JSON.stringify(page))),
    );
    render(<ImagingPage night={false} onBack={() => {}} />);
    const frameRegion = await screen.findByRole("region", { name: "Imaging frame" });
    const solveRegion = screen.getByRole("region", { name: "Imaging solve" });
    expect(Boolean(frameRegion.compareDocumentPosition(solveRegion) & Node.DOCUMENT_POSITION_FOLLOWING)).toBe(true);
  });
});
