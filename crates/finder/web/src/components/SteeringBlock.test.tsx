// US8-S3–S10: literal view, requests and words from plan.md § The page.
// Readouts round to one decimal; all text and request bodies are exact.
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ImagingSteering, ImagingView, StackDropped, StackView } from "../api";
import { ImagingPage } from "./ImagingPage";
import { StackBlock } from "./StackBlock";
import { SteeringBlock } from "./SteeringBlock";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

const STEERING: ImagingSteering = {
  smear_px: 6,
  on: true,
  step_share: 0.5,
  rate_share: 0.3,
  source: "subs",
  holding: true,
  steady: true,
  waiting: false,
  subs: 12,
  dropped_not_steady: 0,
  exposure_s: 5,
  limited: false,
  over_limit: false,
  longest_exposure_s: 9.29,
  would_take_s: 5,
  error_rms_arcsec: 0.9037,
  error_rms_px: 0.41838,
  drift_arcsec: 0.58583,
  drift_px: 0.27122,
};
// 031's ready camera and settings, with US8's literal steering view.
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
  steering: STEERING,
};
const ZERO: StackDropped = {
  not_solved: 0,
  too_few_stars: 0,
  solver_failed: 0,
  soft: 0,
  off_field: 0,
  solve_differs: 0,
  settings: 0,
  frame_failed: 0,
  unreadable: 0,
  behind: 0,
  unprocessed: 0,
};
const STACK: StackView = {
  state: "stacking",
  id: "2026-01-16T02-00-05Z-M42",
  started_at: 1768528805,
  options: { reject: true, focus_gate: true, gains: [1, 1, 1], stretch: 10 },
  taken: 14,
  stacked: 12,
  queued: 0,
  dropped: ZERO,
  exposure_s: 60,
};
function view(edit: Partial<ImagingSteering> = {}): ImagingView {
  return { ...READY, steering: { ...STEERING, ...edit } };
}
function block(edit: Partial<ImagingSteering> = {}) {
  const onChange = vi.fn();
  return { ...render(<SteeringBlock view={view(edit)} onChange={onChange} />), onChange };
}
function serve(body: unknown = READY, status = 200) {
  const fetch = vi.fn(
    async (_input: RequestInfo | URL, _init?: RequestInit) => new Response(JSON.stringify(body), { status }),
  );
  vi.stubGlobal("fetch", fetch);
  return fetch;
}
const field = (name: string) => screen.getByRole("spinbutton", { name }) as HTMLInputElement;
const save = () => fireEvent.click(screen.getByRole("button", { name: "Save tracking settings" }));
function term(name: string) {
  // dt has no implicit accessible name; its exact visible text is the oracle.
  return screen.queryAllByRole("term").find((element) => element.textContent === name);
}
function reads(name: string, words: string) {
  expect(term(name)?.nextElementSibling?.textContent).toBe(words);
}
const ERROR = "Tracking error from the subs (RMS, last 10)";
const LONGEST = "Longest exposure (6 px corner smear)";
const WOULD = "A stack started now would take";

describe("US8-S3/S4: the readout", () => {
  it("shows one region, its status and exactly the five worked terms, without an alert", () => {
    block();
    expect(screen.getAllByRole("region", { name: "Tracking from the subs" })).toHaveLength(1);
    const region = within(screen.getByRole("region", { name: "Tracking from the subs" }));
    expect(region.getByRole("status").textContent).toBe("Steered by the subs · 12 used");
    expect(region.getAllByRole("term").map((element) => element.textContent)).toEqual([
      ERROR,
      "Drift per sub",
      "Exposure",
      LONGEST,
      WOULD,
    ]);
    reads(ERROR, "0.9″ · 0.4 px");
    reads("Drift per sub", "0.6″ · 0.3 px");
    reads("Exposure", "5.0 s");
    reads(LONGEST, "9.3 s");
    reads(WOULD, "5.0 s");
    expect(region.queryByRole("alert")).toBeNull();
    expect(region.getByRole("heading", { name: "Tuning", level: 3 })).toBeTruthy();
    expect(
      region.getByText(
        "The corner smear is used when a stack starts. After a restart of the console it is 6 px again and the switch is on.",
      ).textContent,
    ).toBe(
      "The corner smear is used when a stack starts. After a restart of the console it is 6 px again and the switch is on.",
    );
    expect(
      region.getByText(
        "For tuning in the field: the share of a tracking error one gap steps out, and of a drift it takes out of the rate. After a restart of the console they are 0.5 and 0.3 again.",
      ),
    ).toBeTruthy();
  });

  it("names a shortened exposure, the chosen smear, and an unlimited exposure", () => {
    const { rerender } = block({ limited: true, exposure_s: 30, longest_exposure_s: 37.2 });
    reads("Exposure", "30.0 s, shortened to keep corner stars round");
    reads(LONGEST, "37.2 s");
    rerender(<SteeringBlock view={view({ smear_px: 1.5 })} />);
    reads("Longest exposure (1.5 px corner smear)", "9.3 s");
    expect(term(LONGEST)).toBeUndefined();
    rerender(<SteeringBlock view={view({ longest_exposure_s: "no limit" })} />);
    reads(LONGEST, "none");
  });

  it("warns when the held exposure trails, with the exact action to take", () => {
    block({ over_limit: true, exposure_s: 30, longest_exposure_s: 25.1 });
    expect(screen.getByRole("alert").textContent).toBe(
      "Corner stars are trailing: the longest exposure for round stars is now 25.1 s, and this stack takes 30.0 s. It keeps running. To shorten the exposure, stop it and start a new stack.",
    );
  });

  it("omits unknown figures and leaves dropped subs in the stack's own line", () => {
    const { rerender } = block({ would_take_s: undefined });
    expect(term(WOULD)).toBeUndefined();
    rerender(
      <SteeringBlock
        view={view({
          exposure_s: undefined,
          would_take_s: 15,
          error_rms_px: undefined,
          drift_px: undefined,
          longest_exposure_s: undefined,
          dropped_not_steady: 2,
        })}
      />,
    );
    reads(WOULD, "15.0 s");
    for (const name of ["Exposure", ERROR, "Drift per sub", LONGEST]) expect(term(name)).toBeUndefined();
    expect(screen.getAllByRole("term").map((element) => element.textContent)).toEqual([WOULD]);
  });

  it("shows only the form with four defaults before steering is present", () => {
    render(<SteeringBlock view={{ state: "ready" }} />);
    expect(screen.getByRole("region", { name: "Tracking from the subs" })).toBeTruthy();
    expect(screen.queryByRole("status")).toBeNull();
    expect(screen.queryByRole("term")).toBeNull();
    expect(field("Corner smear (px)").value).toBe("6");
    expect(
      (screen.getByRole("checkbox", { name: "Steer the tracker from the subs" }) as HTMLInputElement).checked,
    ).toBe(true);
    expect(field("Step share").value).toBe("0.5");
    expect(field("Rate share").value).toBe("0.3");
  });
});

describe("US8-S5/S6: saving settings", () => {
  it("PUTs the four keys in order, then calls onChange after success", async () => {
    let answer!: (value: Response) => void;
    const fetch = vi.fn(
      () =>
        new Promise<Response>((resolve) => {
          answer = resolve;
        }),
    );
    vi.stubGlobal("fetch", fetch);
    const { onChange } = block();
    fireEvent.change(field("Corner smear (px)"), { target: { value: "1.5" } });
    fireEvent.click(screen.getByRole("checkbox", { name: "Steer the tracker from the subs" }));
    save();
    expect(fetch).toHaveBeenCalledExactlyOnceWith("/api/imaging/steering", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: '{"on":false,"smear_px":1.5,"step_share":0.5,"rate_share":0.3}',
    });
    expect(onChange).not.toHaveBeenCalled();
    await act(async () => {
      answer(new Response(JSON.stringify(READY)));
    });
    expect(onChange).toHaveBeenCalledTimes(1);
  });

  it("sends both typed tuning shares", async () => {
    const fetch = serve();
    const { onChange } = block();
    fireEvent.change(field("Step share"), { target: { value: "0.25" } });
    fireEvent.change(field("Rate share"), { target: { value: "0.1" } });
    save();
    await waitFor(() => expect(onChange).toHaveBeenCalledOnce());
    expect(fetch).toHaveBeenCalledExactlyOnceWith("/api/imaging/steering", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: '{"on":true,"smear_px":6,"step_share":0.25,"rate_share":0.1}',
    });
  });

  it("shows the route's refusal and calls no onChange", async () => {
    serve({ error: "the corner smear must be a number from 0.1 to 20" }, 400);
    const { onChange } = block();
    save();
    expect((await screen.findByRole("alert")).textContent).toBe("the corner smear must be a number from 0.1 to 20");
    expect(onChange).not.toHaveBeenCalled();
  });

  it("is after the stack on the Imaging page and saving reads /api/imaging again", async () => {
    const fetch = serve({ ...READY, stack: STACK });
    render(<ImagingPage night={false} onBack={() => {}} />);
    const steering = await screen.findByRole("region", { name: "Tracking from the subs" });
    const stack = screen.getByRole("region", { name: "Stack" });
    expect(Boolean(stack.compareDocumentPosition(steering) & Node.DOCUMENT_POSITION_FOLLOWING)).toBe(true);
    expect(fetch.mock.calls.filter(([url]) => url === "/api/imaging")).toHaveLength(1);
    save();
    await waitFor(() => expect(fetch.mock.calls.filter(([url]) => url === "/api/imaging")).toHaveLength(2));
  });
});

describe("US8-S7/S8: what steers, or why exposures wait", () => {
  it.each([
    [{ source: "tracker", holding: false }, "Steered by the tracker's own corrections"],
    [
      { source: "tracker", holding: true, on: true },
      "No sub accepted lately: the tracker's own corrections steer until one is. See Dropped in the stack.",
    ],
    [{ source: "tracker", holding: true, on: false }, "The switch is off: the tracker's own corrections steer"],
    [
      { source: "tracker", far_arcsec: 476.4 },
      "The target is 7.9′ from the set point: the tracker steers on its own. Centre it, or turn Track off and on to apply the measured aim point.",
    ],
    [{ source: "none", holding: false }, "Not tracking: nothing steers"],
    [{ steady: false, waiting: false }, "Steered by the subs · 12 used"],
  ] satisfies [Partial<ImagingSteering>, string][])("reads %j", (edit, words) => {
    block(edit);
    expect(screen.getByRole("status").textContent).toBe(words);
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("warns only when nothing tracks while a stack still holds the steering", () => {
    block({ source: "none", holding: true });
    expect(screen.getByRole("status").textContent).toBe("Not tracking: nothing steers");
    expect(screen.getByRole("alert").textContent).toBe(
      "Nothing is tracking, and the stack is still exposing. Turn Track on or stop the stack.",
    );
  });

  it.each([
    ["starting", "Exposures paused: tracking is starting"],
    ["reversing", "Exposures paused for a moment: the mount is reversing"],
    ["measuring", "Exposures paused: the mount is measuring its gear play, once, for a few minutes"],
    ["overhead", "Exposures paused: the target is too near overhead to follow. They resume when it has passed"],
    ["pushed", "Exposures paused: the telescope is being moved by hand"],
    ["stalled", "Exposures paused for a moment: the mount stopped and is being started again"],
    [
      "limit",
      "Exposures paused: corner stars would trail. They resume when the longest exposure for round stars reaches 15.0 s",
    ],
    [undefined, "Exposures paused: the tracker is not steady"],
    ["flipping", "Exposures paused: the tracker is not steady"],
  ])("reads the pause %s before the source", (pause, words) => {
    block({ waiting: true, pause, exposure_s: 30 });
    expect(screen.getByRole("status").textContent).toBe(words);
  });
});

describe("US8-S9: the block owns editable drafts", () => {
  it("keeps drafts on the same settings and replaces them when settings change", () => {
    const { rerender } = block();
    fireEvent.change(field("Corner smear (px)"), { target: { value: "1.5" } });
    fireEvent.change(field("Step share"), { target: { value: "0.25" } });
    fireEvent.change(field("Rate share"), { target: { value: "0.1" } });
    fireEvent.click(screen.getByRole("checkbox", { name: "Steer the tracker from the subs" }));
    rerender(<SteeringBlock view={view({ subs: 13 })} />);
    expect(field("Corner smear (px)").value).toBe("1.5");
    expect(field("Step share").value).toBe("0.25");
    expect(field("Rate share").value).toBe("0.1");
    expect((screen.getByRole("checkbox") as HTMLInputElement).checked).toBe(false);
    rerender(<SteeringBlock view={view({ smear_px: 3 })} />);
    expect(field("Corner smear (px)").value).toBe("3");
    expect(field("Step share").value).toBe("0.5");
    expect(field("Rate share").value).toBe("0.3");
    expect((screen.getByRole("checkbox") as HTMLInputElement).checked).toBe(true);
  });

  it("takes values when steering first arrives, even when they match the defaults", () => {
    const { rerender } = render(<SteeringBlock view={{ state: "ready" }} />);
    fireEvent.change(field("Corner smear (px)"), { target: { value: "3" } });
    rerender(<SteeringBlock view={READY} />);
    expect(field("Corner smear (px)").value).toBe("6");
  });
});

it("US8-S10: names not_steady last and still reads an older dropped object", () => {
  const props = { night: false, onChange: vi.fn() };
  const { rerender } = render(
    <StackBlock {...props} view={{ ...READY, stack: { ...STACK, dropped: { ...ZERO, not_steady: 2 } } }} />,
  );
  expect(screen.getByText(/^Dropped/).textContent).toBe("Dropped 2: 2 while the mount was not steady");
  rerender(<StackBlock {...props} view={{ ...READY, stack: { ...STACK, dropped: { ...ZERO, behind: 4 } } }} />);
  expect(screen.getByText(/^Dropped/).textContent).toBe("Dropped 4: 4 behind");
  rerender(
    <StackBlock {...props} view={{ ...READY, stack: { ...STACK, dropped: { ...ZERO, behind: 4, not_steady: 2 } } }} />,
  );
  expect(screen.getByText(/^Dropped/).textContent).toBe("Dropped 6: 4 behind, 2 while the mount was not steady");
});
