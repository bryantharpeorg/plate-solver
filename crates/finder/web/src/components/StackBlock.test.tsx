// US9-S1–S6. Oracle: the literal views, requests and words in plan.md § The page.
// Counts, strings and exposure truncation are exact; colour's 10 s limit and
// immediate reads use fake timers and a deferred HTTP answer, never wall time.
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ImagingView, StackDropped, StackView } from "../api";
import { ImagingPage } from "./ImagingPage";
import { StackBlock } from "./StackBlock";

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

// 031's ready view, with no stack or calibration.
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
  stacked: 9,
  queued: 0,
  dropped: ZERO,
  exposure_s: 90,
};
const ENDED: StackView = {
  ...STACK,
  state: "ended",
  ended_reason: "stopped",
  dropped: { ...ZERO, not_solved: 2, behind: 2, unprocessed: 1 },
  files: {
    fits: "runs/2026-01-16T02-00-05Z-M42/stack.fits",
    png: "runs/2026-01-16T02-00-05Z-M42/stack.png",
    json: "runs/2026-01-16T02-00-05Z-M42/stack.json",
  },
};
const HINT = "Takes subs one after another and stacks them into one picture. Turn Track on first.";
const NIGHT_HINT = "The picture is red at night. Hold the control to see its colours for up to 10 seconds.";
const END_STATUS = "Stack 2026-01-16T02-00-05Z-M42: 9 subs stacked, 5 dropped.";
const response = (body: unknown, status = 200) => new Response(JSON.stringify(body), { status });
function serve(body: unknown = READY, status = 200) {
  const fetch = vi.fn(async (_input: RequestInfo | URL, _init?: RequestInit) => response(body, status));
  vi.stubGlobal("fetch", fetch);
  return fetch;
}
const button = (name: string) => screen.getByRole("button", { name });
const field = (name: string) => screen.getByRole("spinbutton", { name }) as HTMLInputElement;
function block(stack?: StackView, night = false) {
  const onChange = vi.fn();
  return { ...render(<StackBlock view={{ ...READY, stack }} night={night} onChange={onChange} />), onChange };
}

describe("US9-S1: before the first stack", () => {
  it("has one Stack region, four default fields and its hint, with no status, alert or picture", () => {
    block();
    expect(screen.getAllByRole("region", { name: "Stack" })).toHaveLength(1);
    const region = within(screen.getByRole("region", { name: "Stack" }));
    expect(region.getByRole("button", { name: "Start stacking" }).classList.contains("button-primary")).toBe(true);
    expect(region.getByText(HINT).textContent).toBe(HINT);
    for (const name of ["Red gain", "Green gain", "Blue gain"])
      expect((region.getByRole("spinbutton", { name }) as HTMLInputElement).value).toBe("1");
    expect((region.getByRole("spinbutton", { name: "Stretch" }) as HTMLInputElement).value).toBe("10");
    for (const role of ["status", "alert", "img"]) expect(region.queryByRole(role)).toBeNull();
    for (const name of ["Stop stacking", "Apply colour", "Hold to see colour"])
      expect(region.queryByRole("button", { name })).toBeNull();
  });

  it("posts the defaults and calls onChange only after the request succeeds", async () => {
    let answer!: (value: Response) => void;
    const accepted = new Promise<Response>((resolve) => {
      answer = resolve;
    });
    const fetch = vi.fn(() => accepted);
    vi.stubGlobal("fetch", fetch);
    const { onChange } = block();
    fireEvent.click(button("Start stacking"));
    expect(fetch).toHaveBeenCalledExactlyOnceWith("/api/imaging/stack", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: '{"gains":[1,1,1],"stretch":10}',
    });
    expect(onChange).not.toHaveBeenCalled();
    await act(async () => {
      answer(response(READY));
    });
    expect(onChange).toHaveBeenCalledTimes(1);
  });

  it("shows the refusal's exact words and calls no onChange", async () => {
    serve({ error: "this console has no imaging solver" }, 409);
    const { onChange } = block();
    fireEvent.click(button("Start stacking"));
    expect((await screen.findByRole("alert")).textContent).toBe("this console has no imaging solver");
    expect(onChange).not.toHaveBeenCalled();
  });

  it("starts with the observer's colour fields", async () => {
    const fetch = serve();
    const { onChange } = block();
    fireEvent.change(field("Blue gain"), { target: { value: "0.5" } });
    fireEvent.change(field("Stretch"), { target: { value: "20" } });
    fireEvent.click(button("Start stacking"));
    await waitFor(() => expect(onChange).toHaveBeenCalledTimes(1));
    expect(fetch).toHaveBeenCalledExactlyOnceWith("/api/imaging/stack", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: '{"gains":[1,1,0.5],"stretch":20}',
    });
  });
});

describe("US9-S2: a running stack", () => {
  it.each([
    [90, "1 min 30 s"],
    [45, "45 s"],
    [119.9, "1 min 59 s"],
    [60, "1 min 0 s"],
  ])("reads %s seconds as %s, rounded down", (exposure_s, words) => {
    block({ ...STACK, exposure_s });
    expect(screen.getByRole("status").textContent).toBe(`Stacking · 9 of 14 subs stacked · ${words} of exposure`);
    expect(button("Stop stacking")).toBeTruthy();
    expect(button("Apply colour")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Start stacking" })).toBeNull();
    expect(screen.queryByText(/^Dropped/)).toBeNull();
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("stops with DELETE then calls onChange", async () => {
    const fetch = serve({ ...READY, stack: ENDED });
    const { onChange } = block(STACK);
    fireEvent.click(button("Stop stacking"));
    await waitFor(() => expect(onChange).toHaveBeenCalledTimes(1));
    expect(fetch).toHaveBeenCalledExactlyOnceWith("/api/imaging/stack", { method: "DELETE" });
  });

  it("lists every nonzero dropped count in order and the missing masters", () => {
    const stack: StackView = {
      ...STACK,
      dropped: {
        not_solved: 2,
        too_few_stars: 1,
        solver_failed: 1,
        soft: 3,
        off_field: 1,
        solve_differs: 1,
        settings: 1,
        frame_failed: 1,
        unreadable: 1,
        behind: 4,
        unprocessed: 0,
      },
      missing: ["dark", "flat", "bad_pixels"],
    };
    const { onChange, rerender } = block(stack);
    const words =
      "Dropped 16: 2 not solved, 1 too few stars, 1 solver failed, 3 soft, 1 off the field, 1 solved differently, 1 settings changed, 1 not received, 1 unreadable, 4 behind";
    const region = within(screen.getByRole("region", { name: "Stack" }));
    expect(region.getByText(words).textContent).toBe(words);
    expect(region.getByText(/^Stacked without/).textContent).toBe(
      "Stacked without a master dark, a master flat, a bad-pixel map.",
    );
    rerender(
      <StackBlock
        view={{ ...READY, stack: { ...stack, missing: [], dropped: { ...ZERO, unprocessed: 1 } } }}
        night={false}
        onChange={onChange}
      />,
    );
    expect(region.queryByText(/^Stacked without/)).toBeNull();
    expect(region.getByText(/^Dropped/).textContent).toBe("Dropped 1: 1 unprocessed");
  });
});

describe("US9-S3: the picture and the night hold", () => {
  const pictureStack = { ...STACK, preview: "/api/imaging/stack/preview.jpg?n=9" };
  it("names and sources the photograph, with no hold control by day", () => {
    block(pictureStack);
    const picture = screen.getByRole("img", { name: "Stack of 9 subs" });
    expect(picture.getAttribute("src")).toBe("/api/imaging/stack/preview.jpg?n=9");
    expect(picture.classList.contains("photo")).toBe(true);
    expect(screen.queryByRole("button", { name: "Hold to see colour" })).toBeNull();
  });

  it.each(["pointerUp", "pointerLeave", "pointerCancel", "blur"] as const)("restores red on %s", (release) => {
    block(pictureStack, true);
    const region = within(screen.getByRole("region", { name: "Stack" }));
    const hold = region.getByRole("button", { name: "Hold to see colour" });
    const picture = region.getByRole("img", { name: "Stack of 9 subs" });
    expect(region.getByText(NIGHT_HINT).textContent).toBe(NIGHT_HINT);
    expect(hold.getAttribute("aria-pressed")).toBe("false");
    fireEvent.pointerDown(hold);
    expect(picture.classList.contains("photo")).toBe(false);
    expect(hold.getAttribute("aria-pressed")).toBe("true");
    fireEvent[release](hold);
    expect(picture.classList.contains("photo")).toBe(true);
    expect(hold.getAttribute("aria-pressed")).toBe("false");
  });

  it("ends an unreleased hold after exactly 10 seconds", () => {
    vi.useFakeTimers();
    block(pictureStack, true);
    const hold = button("Hold to see colour");
    const picture = screen.getByRole("img");
    fireEvent.pointerDown(hold);
    act(() => vi.advanceTimersByTime(9_999));
    expect(picture.classList.contains("photo")).toBe(false);
    expect(hold.getAttribute("aria-pressed")).toBe("true");
    act(() => vi.advanceTimersByTime(1));
    expect(picture.classList.contains("photo")).toBe(true);
    expect(hold.getAttribute("aria-pressed")).toBe("false");
  });

  it.each([" ", "Enter"])("passes keyboard holds through the shared hook for %j", (key) => {
    block(pictureStack, true);
    const hold = button("Hold to see colour");
    fireEvent.keyDown(hold, { key });
    expect(hold.getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByRole("img").classList.contains("photo")).toBe(false);
    fireEvent.keyUp(hold, { key });
    expect(hold.getAttribute("aria-pressed")).toBe("false");
    expect(screen.getByRole("img").classList.contains("photo")).toBe(true);
  });
});

describe("US9-S4: an ended stack", () => {
  it("names its folder, counts and files, and offers a new start without running-stack lines", () => {
    block({ ...ENDED, missing: ["dark"], preview: "/api/imaging/stack/preview.jpg?n=9" });
    expect(screen.getByRole("status").textContent).toBe(
      `${END_STATUS} Saved runs/2026-01-16T02-00-05Z-M42/stack.fits and stack.png.`,
    );
    expect(button("Start stacking")).toBeTruthy();
    for (const name of ["Stop stacking", "Apply colour"]) expect(screen.queryByRole("button", { name })).toBeNull();
    expect(screen.queryByText(HINT)).toBeNull();
    expect(screen.queryByText(/^Dropped/)).toBeNull();
    expect(screen.queryByText(/^Stacked without/)).toBeNull();
    expect(screen.queryByRole("alert")).toBeNull();
    expect(screen.getByRole("img", { name: "Stack of 9 subs" }).classList.contains("photo")).toBe(true);
  });

  it("explains a camera ending", () => {
    block({ ...ENDED, ended_reason: "camera", error: "the scope node is not connected" });
    expect(screen.getByRole("alert").textContent).toBe("Stacking stopped: the scope node is not connected.");
  });

  it("reports a failed save without claiming saved files", () => {
    block({ ...ENDED, files: undefined, save_error: "could not save the stack: disk full" });
    expect(screen.getByRole("status").textContent).toBe(END_STATUS);
    expect(screen.getByRole("alert").textContent).toBe("The stack was not saved: could not save the stack: disk full.");
  });

  it.each([undefined, { json: "runs/x/stack.json" }, { fits: "runs/x/stack.fits", json: "runs/x/stack.json" }])(
    "does not claim saved pictures from incomplete files %j",
    (files) => {
      block({ ...ENDED, files });
      expect(screen.getByRole("status").textContent).toBe(END_STATUS);
    },
  );
});

describe("US9-S5: colour settings", () => {
  it("applies the changed gains and stretch then calls onChange", async () => {
    const fetch = serve({ ...READY, stack: STACK });
    const { onChange } = block(STACK);
    fireEvent.change(field("Red gain"), { target: { value: "1.5" } });
    fireEvent.change(field("Stretch"), { target: { value: "25" } });
    fireEvent.click(button("Apply colour"));
    await waitFor(() => expect(onChange).toHaveBeenCalledTimes(1));
    expect(fetch).toHaveBeenCalledExactlyOnceWith("/api/imaging/stack", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: '{"gains":[1.5,1,1],"stretch":25}',
    });
  });

  it("shows a refused colour change and calls no onChange", async () => {
    serve({ error: "gains must be three numbers from 0.1 to 10" }, 400);
    const { onChange } = block(STACK);
    fireEvent.click(button("Apply colour"));
    expect((await screen.findByRole("alert")).textContent).toBe("gains must be three numbers from 0.1 to 10");
    expect(onChange).not.toHaveBeenCalled();
  });

  it("seeds from options, preserves a draft across identical options and follows changed gains and stretch", () => {
    const stack = {
      ...STACK,
      options: { ...STACK.options, gains: [2, 1, 0.5] as [number, number, number], stretch: 20 },
    };
    const { onChange, rerender } = block(stack);
    expect(["Red gain", "Green gain", "Blue gain", "Stretch"].map((name) => field(name).value)).toEqual([
      "2",
      "1",
      "0.5",
      "20",
    ]);
    fireEvent.change(field("Red gain"), { target: { value: "1.5" } });
    rerender(
      <StackBlock
        view={{ ...READY, stack: { ...stack, taken: 15, options: { ...stack.options, gains: [2, 1, 0.5] } } }}
        night={false}
        onChange={onChange}
      />,
    );
    expect(field("Red gain").value).toBe("1.5");
    rerender(
      <StackBlock
        view={{ ...READY, stack: { ...stack, options: { ...stack.options, gains: [3, 1, 1] } } }}
        night={false}
        onChange={onChange}
      />,
    );
    expect(field("Red gain").value).toBe("3");
    rerender(
      <StackBlock
        view={{ ...READY, stack: { ...stack, options: { ...stack.options, gains: [3, 1, 1], stretch: 30 } } }}
        night={false}
        onChange={onChange}
      />,
    );
    expect(field("Stretch").value).toBe("30");
  });
});

describe("US9-S6: the Stack region on ImagingPage", () => {
  it("appears once after Calibration for a ready view", async () => {
    serve({ ...READY, calibration: { masters: [] } });
    render(<ImagingPage night={false} onBack={() => {}} />);
    const stack = await screen.findByRole("region", { name: "Stack" });
    expect(screen.getAllByRole("region", { name: "Stack" })).toHaveLength(1);
    expect(
      Boolean(
        screen.getByRole("region", { name: "Calibration" }).compareDocumentPosition(stack) &
        Node.DOCUMENT_POSITION_FOLLOWING,
      ),
    ).toBe(true);
  });

  it.each<ImagingView>([
    { state: "off" },
    { state: "no_link" },
    { state: "no_service" },
    { state: "no_camera" },
    { state: "error", error: "cannot reach the INDI server" },
  ])("is absent for $state even if the view carries a stack", async (view) => {
    serve({ ...view, stack: STACK });
    render(<ImagingPage night={true} onBack={() => {}} />);
    await act(async () => {});
    expect(screen.queryByRole("region", { name: "Stack" })).toBeNull();
  });

  it("reads /api/imaging immediately after a successful start and displays that read's stack", async () => {
    vi.useFakeTimers();
    let answer!: (value: Response) => void;
    const accepted = new Promise<Response>((resolve) => {
      answer = resolve;
    });
    let stacking = false;
    const fetch = vi.fn(async (input: RequestInfo | URL, _init?: RequestInit) => {
      if (String(input) === "/api/imaging/stack") return accepted;
      return response({ ...READY, ...(stacking ? { stack: STACK } : {}) });
    });
    vi.stubGlobal("fetch", fetch);
    render(<ImagingPage night={false} onBack={() => {}} />);
    await act(async () => {});
    fireEvent.click(button("Start stacking"));
    await act(async () => {});
    expect(fetch.mock.calls.map(([url]) => String(url))).toEqual(["/api/imaging", "/api/imaging/stack"]);
    stacking = true;
    await act(async () => {
      answer(response(READY));
    });
    expect(fetch.mock.calls.map(([url]) => String(url))).toEqual([
      "/api/imaging",
      "/api/imaging/stack",
      "/api/imaging",
    ]);
    const region = within(screen.getByRole("region", { name: "Stack" }));
    expect(region.getByRole("status").textContent).toBe("Stacking · 9 of 14 subs stacked · 1 min 30 s of exposure");
  });
});
