// US9's oracles are the exact words, request bodies and worked times in 042's
// scenarios and plan, with the focus and centring messages from 041 US6/US5.
// All comparisons are exact; the only clock advanced is Vitest's in S9.
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { deleteImagingRun, postImagingRun, postImagingRunContinue, type ImagingRun, type ImagingView } from "../api";
import { readWebFile } from "../test/files";
import { ImagingPage } from "./ImagingPage";
import { ImagingRunBlock } from "./ImagingRunBlock";

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

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
    gain: null,
    offset: null,
    binnings: [1, 2],
    cooler: false,
  },
  settings: {
    exposure_ms: 1000,
    gain: null,
    offset: null,
    binning: 1,
    roi: { x: 0, y: 0, width: 1236, height: 822 },
    cooler_on: false,
    set_temp_c: null,
    forward_kib_per_s: null,
    save_at_node: false,
  },
};
const STACKING: ImagingRun = {
  id: "2026-01-16T02-00-00Z-M42",
  state: "stacking",
  by: "page",
  target: { id: "NGC 1976", name: "M42", ra_deg: 83.82, dec_deg: -5.39 },
  started_at: 1768528800,
  elapsed_s: 125.4,
  settings: { subs: null, duration_s: 7200, keep_subs: true },
  subs: { stacked: 12, dropped: { not_solved: 2, too_few_stars: 1 } },
  message: "Stacking M42: 12 stacked, 3 dropped.",
};
const FOCUS_MESSAGE =
  "Focus is clearly out: stars are 9.8″ across against a record of 3.1″. Turn the focuser while watching the focus aid below, then check focus again or go on anyway.";
const PAUSED: ImagingRun = { ...STACKING, state: "focus_pause", message: FOCUS_MESSAGE };
const ENDED: ImagingRun = {
  ...STACKING,
  state: "ended",
  folder: "runs/2026-01-16T02-00-10Z-M42",
  ended: {
    reason: "stopped",
    at: 1768529000,
    message: "Imaging run stopped. Track is still on.",
    saved: true,
    track_on: true,
  },
};
const TRACK_NOTE = "Track is still on: the top bar's Stop stops the mount.";
const NO_ANSWER = "The console did not answer. The run itself is not affected; try again.";
const REFUSED = "the imaging run is not paused";
const response = (body: unknown = READY, status = 200) => new Response(JSON.stringify(body), { status });

function serve(view: ImagingView = READY) {
  const fetch = vi.fn(async (_input: RequestInfo | URL, _init?: RequestInit) => response(view));
  vi.stubGlobal("fetch", fetch);
  return fetch;
}
function pending() {
  let answer!: (response: Response) => void;
  const fetch = vi.fn(
    (_input: RequestInfo | URL, _init?: RequestInit) =>
      new Promise<Response>((resolve) => {
        answer = resolve;
      }),
  );
  vi.stubGlobal("fetch", fetch);
  return { fetch, answer: (value: Response) => answer(value) };
}
function block(run?: ImagingRun, view: ImagingView = READY) {
  const refresh = vi.fn<() => Promise<void> | void>();
  return { refresh, ...render(<ImagingRunBlock view={{ ...view, run }} refresh={refresh} />) };
}
const button = (name: string) => screen.getByRole<HTMLButtonElement>("button", { name });
const field = (name: string) => screen.getByRole<HTMLInputElement>("spinbutton", { name });
const type = (name: string, value: string) => fireEvent.change(field(name), { target: { value } });
const click = (name: string) => fireEvent.click(button(name));
const text = (role: "status" | "alert", words: string) => expect(screen.getByRole(role).textContent).toBe(words);
const paragraph = (words: string) => expect(screen.getByText(words, { exact: true }).tagName).toBe("P");
const post = (body: string) => ({ method: "POST", headers: { "Content-Type": "application/json" }, body });

describe("US9-S1/S11: the start form", () => {
  it("offers one region, two empty limits and Keep every sub without a status or alert", () => {
    block();
    expect(screen.getAllByRole("region", { name: "Imaging run" })).toHaveLength(1);
    const region = within(screen.getByRole("region", { name: "Imaging run" }));
    expect(region.getByRole("button", { name: "Start imaging run" })).toBeTruthy();
    expect(field("Subs to stack").value).toBe("");
    expect(field("Minutes to stack").value).toBe("");
    expect(field("Minutes to stack").placeholder).toBe("120");
    expect(region.getByRole<HTMLInputElement>("checkbox", { name: "Keep every sub" }).checked).toBe(true);
    expect(region.queryByRole("status")).toBeNull();
    expect(region.queryByRole("alert")).toBeNull();
    paragraph(
      "Goes to the target, checks focus, centres it on the imaging camera's sensor and stacks subs until you stop it or a limit is reached (two hours unless you give another).",
    );
  });

  it("estimates full-sensor 16-bit subs, binning squared and only exposures that fit", () => {
    const { rerender, refresh } = block();
    paragraph("About 14.6 GB of subs in 120 minutes at the current exposure.");
    type("Minutes to stack", "30");
    paragraph("About 3.7 GB of subs in 30 minutes at the current exposure.");
    type("Minutes to stack", "");
    rerender(<ImagingRunBlock view={{ ...READY, settings: { ...READY.settings!, binning: 2 } }} refresh={refresh} />);
    paragraph("About 3.7 GB of subs in 120 minutes at the current exposure.");
    // 24 exposures fit: 25 would round up to 0.1 GB instead of 0.0 GB.
    rerender(
      <ImagingRunBlock view={{ ...READY, settings: { ...READY.settings!, exposure_ms: 293878 } }} refresh={refresh} />,
    );
    paragraph("About 0.0 GB of subs in 120 minutes at the current exposure.");
    // A region crops the picture, but not the whole-sensor estimate.
    rerender(
      <ImagingRunBlock
        view={{ ...READY, settings: { ...READY.settings!, roi: { x: 0, y: 0, width: 20, height: 20 } } }}
        refresh={refresh}
      />,
    );
    paragraph("About 14.6 GB of subs in 120 minutes at the current exposure.");
  });

  it("warns about an unfinished run while still offering Start", () => {
    block(undefined, { ...READY, unfinished_run: "runs/2026-01-15T03-00-00Z-M31" });
    text(
      "alert",
      "The last imaging run did not finish: the console stopped while it ran. Its subs are in runs/2026-01-15T03-00-00Z-M31/subs.",
    );
    expect(button("Start imaging run")).toBeTruthy();
  });
});

describe("US9-S2: a run that is on", () => {
  it("reads the exact message, counts and elapsed time, and stops on the second tap", async () => {
    const fetch = serve();
    const { refresh, rerender } = block(STACKING);
    text("status", "Stacking M42: 12 stacked, 3 dropped.");
    paragraph("12 stacked · 3 dropped (not solved 2, too few stars 1)");
    paragraph("Elapsed 2:05");
    expect(screen.queryByRole("button", { name: "Start imaging run" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Check focus again" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Go anyway" })).toBeNull();
    expect(screen.queryByRole("spinbutton")).toBeNull();
    expect(screen.queryByText("Started by an MCP client")).toBeNull();
    click("Stop imaging run");
    expect(button("Tap again to stop the run")).toBeTruthy();
    expect(fetch).not.toHaveBeenCalled();
    click("Tap again to stop the run");
    await waitFor(() => expect(refresh).toHaveBeenCalledOnce());
    expect(fetch).toHaveBeenCalledExactlyOnceWith("/api/imaging/run", { method: "DELETE" });
    expect(fetch.mock.invocationCallOrder[0]!).toBeLessThan(refresh.mock.invocationCallOrder[0]!);
    rerender(
      <ImagingRunBlock view={{ ...READY, run: { ...STACKING, by: "mcp", elapsed_s: 125.6 } }} refresh={refresh} />,
    );
    paragraph("Started by an MCP client");
    expect(screen.getByText("Started by an MCP client").className).toBe("min-w-0 break-words text-sm text-ink-2");
    paragraph("Elapsed 2:05");
    rerender(
      <ImagingRunBlock
        view={{ ...READY, run: { ...STACKING, subs: { stacked: 5, dropped: {} } } }}
        refresh={refresh}
      />,
    );
    paragraph("5 stacked · 0 dropped");
  });
});

describe("US9-S3/S4: focus pause and centring", () => {
  it.each([
    ["Check focus again", false],
    ["Go anyway", true],
  ] as const)("%s posts anyway %s then refreshes", async (name, anyway) => {
    const fetch = serve();
    const { refresh } = block(PAUSED);
    text("alert", FOCUS_MESSAGE);
    expect(screen.queryByRole("status")).toBeNull();
    const pauseRow = button("Check focus again").parentElement!;
    expect(pauseRow.contains(button("Go anyway"))).toBe(true);
    expect(pauseRow.contains(button("Stop imaging run"))).toBe(false);
    click(name);
    await waitFor(() => expect(refresh).toHaveBeenCalledOnce());
    expect(fetch).toHaveBeenCalledExactlyOnceWith("/api/imaging/run/continue", post(JSON.stringify({ anyway })));
    expect(fetch.mock.invocationCallOrder[0]!).toBeLessThan(refresh.mock.invocationCallOrder[0]!);
  });

  it("prints the run's centring message and warning as given", () => {
    block({
      ...STACKING,
      state: "centring",
      message: "Centring M42: 400″ from the sensor's centre, try 1 of 3.",
      warnings: ["Focus is slightly soft: stars are 3.6″ across against a record of 3.1″."],
    });
    text("status", "Centring M42: 400″ from the sensor's centre, try 1 of 3.");
    paragraph("Focus is slightly soft: stars are 3.6″ across against a record of 3.1″.");
  });
});

describe("US9-S5/S8: an ended run", () => {
  it.each([
    [
      "stopped",
      "Imaging run stopped. Track is still on.",
      12,
      true,
      "status",
      "Imaging run stopped. Track is still on. 12 subs stacked · saved on the console in runs/2026-01-16T02-00-10Z-M42.",
    ],
    [
      "stopped",
      "Imaging run stopped. Track is still on.",
      0,
      true,
      "status",
      "Imaging run stopped. Track is still on. No sub was stacked.",
    ],
    [
      "sun_up",
      "Ended: the Sun is up.",
      12,
      true,
      "alert",
      "Ended: the Sun is up. 12 subs stacked · saved on the console in runs/2026-01-16T02-00-10Z-M42.",
    ],
    [
      "count_limit",
      "Finished: the 12 subs asked for are stacked.",
      12,
      true,
      "status",
      "Finished: the 12 subs asked for are stacked. Saved on the console in runs/2026-01-16T02-00-10Z-M42.",
    ],
    [
      "duration_limit",
      "Finished: the 120-minute limit was reached.",
      12,
      true,
      "status",
      "Finished: the 120-minute limit was reached. 12 subs stacked · saved on the console in runs/2026-01-16T02-00-10Z-M42.",
    ],
    [
      "stopped",
      "Imaging run stopped. Track is still on.",
      12,
      false,
      "alert",
      "Imaging run stopped. Track is still on. 12 subs were stacked but the stack was not saved: the subs are in runs/2026-01-16T02-00-10Z-M42/subs.",
    ],
    [
      "save_failed",
      "Ended: the console could not save to its imaging folder: no space left on device. Check the disk.",
      12,
      false,
      "alert",
      "Ended: the console could not save to its imaging folder: no space left on device. Check the disk. 12 subs were stacked but the stack was not saved: the subs are in runs/2026-01-16T02-00-10Z-M42/subs.",
    ],
    [
      "stopped",
      "Imaging run stopped. Track is still on.",
      1,
      true,
      "status",
      "Imaging run stopped. Track is still on. 1 sub stacked · saved on the console in runs/2026-01-16T02-00-10Z-M42.",
    ],
    [
      "stopped",
      "Imaging run stopped. Track is still on.",
      1,
      false,
      "alert",
      "Imaging run stopped. Track is still on. 1 sub was stacked but the stack was not saved: the subs are in runs/2026-01-16T02-00-10Z-M42/subs.",
    ],
    [
      "count_limit",
      "Finished: the 12 subs asked for are stacked.",
      12,
      false,
      "alert",
      "Finished: the 12 subs asked for are stacked. 12 subs were stacked but the stack was not saved: the subs are in runs/2026-01-16T02-00-10Z-M42/subs.",
    ],
    [
      "stopped",
      "Imaging run stopped. Track is still on.",
      0,
      false,
      "status",
      "Imaging run stopped. Track is still on. No sub was stacked.",
    ],
  ] as const)("%s, %i subs, saved %s: %s", (reason, message, stacked, saved, role, words) => {
    block({
      ...ENDED,
      subs: { stacked, dropped: {} },
      ended: { ...ENDED.ended!, reason, message, saved, track_on: reason !== "sun_up" },
    });
    text(role, words);
    expect(screen.queryByRole(role === "status" ? "alert" : "status")).toBeNull();
    expect(button("Start imaging run")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Stop imaging run" })).toBeNull();
    if (reason === "sun_up") expect(screen.queryByText(TRACK_NOTE)).toBeNull();
    else paragraph(TRACK_NOTE);
  });

  it("prints each ended warning and removes the Track line once Track ends", () => {
    const warnings = [
      "Focus is slightly soft: stars are 3.6″ across against a record of 3.1″.",
      "Focus was not checked: going on anyway.",
    ];
    const { refresh, rerender } = block({ ...ENDED, warnings });
    for (const warning of warnings) paragraph(warning);
    paragraph(TRACK_NOTE);
    rerender(
      <ImagingRunBlock
        view={{ ...READY, run: { ...ENDED, ended: { ...ENDED.ended!, track_on: false } } }}
        refresh={refresh}
      />,
    );
    expect(screen.queryByText(TRACK_NOTE)).toBeNull();
  });
});

describe("US9-S6: the page through useImaging", () => {
  it("places the run before Camera, retains an ended run without a link, and hides an empty block without a link", async () => {
    serve({ ...READY, run: STACKING });
    const first = render(<ImagingPage night={false} onBack={() => {}} />);
    const region = await screen.findByRole("region", { name: "Imaging run" });
    const camera = screen.getByRole("region", { name: "Camera" });
    expect(Boolean(region.compareDocumentPosition(camera) & Node.DOCUMENT_POSITION_FOLLOWING)).toBe(true);
    expect(within(region).getByRole("status").textContent).toBe("Stacking M42: 12 stacked, 3 dropped.");
    first.unmount();
    serve({ state: "no_link", run: ENDED });
    const second = render(<ImagingPage night={true} onBack={() => {}} />);
    const ended = await screen.findByRole("region", { name: "Imaging run" });
    expect(within(ended).getByRole("status").textContent).toBe(
      "Imaging run stopped. Track is still on. 12 subs stacked · saved on the console in runs/2026-01-16T02-00-10Z-M42.",
    );
    second.unmount();
    serve({ state: "no_link" });
    render(<ImagingPage night={false} onBack={() => {}} />);
    await screen.findByText("The scope node is not connected.");
    expect(screen.queryByRole("region", { name: "Imaging run" })).toBeNull();
  });
});

it("US9-S7: records the built block once in the design system", () => {
  expect(readWebFile("../../../DESIGN.md").split("\n")).toContain("### Imaging run — built");
  const design = JSON.parse(readWebFile("../../../.impeccable/design.json")) as {
    components: { name: string; kind: string; refersTo: string }[];
  };
  const entries = design.components.filter((entry) => entry.name === "Imaging run");
  expect(entries).toHaveLength(1);
  expect(entries[0]).toMatchObject({ name: "Imaging run", kind: "custom", refersTo: "button" });
});

describe("US9-S9: Stop asks for a second tap for five seconds", () => {
  it("expires at 5 s, disables while stopping and while saving, and refreshes after the answer", async () => {
    vi.useFakeTimers();
    const request = pending();
    const { refresh, rerender, unmount } = block(STACKING);
    click("Stop imaging run");
    expect(request.fetch).not.toHaveBeenCalled();
    expect(button("Tap again to stop the run")).toBeTruthy();
    act(() => vi.advanceTimersByTime(4999));
    expect(button("Tap again to stop the run")).toBeTruthy();
    act(() => vi.advanceTimersByTime(1));
    click("Stop imaging run");
    expect(request.fetch).not.toHaveBeenCalled();
    act(() => vi.advanceTimersByTime(4999));
    click("Tap again to stop the run");
    expect(request.fetch).toHaveBeenCalledExactlyOnceWith("/api/imaging/run", { method: "DELETE" });
    expect(button("Stopping…").disabled).toBe(true);
    expect(refresh).not.toHaveBeenCalled();
    await act(async () => request.answer(response()));
    expect(refresh).toHaveBeenCalledOnce();
    rerender(
      <ImagingRunBlock
        view={{ ...READY, run: { ...STACKING, message: "Saving the stack: 12 stacked." } }}
        refresh={refresh}
      />,
    );
    expect(button("Stop imaging run").disabled).toBe(true);
    click("Stop imaging run");
    expect(request.fetch).toHaveBeenCalledOnce();
    unmount();
    expect(vi.getTimerCount()).toBe(0);
  });
});

describe("US9-S10: a pause request's own alert and pending buttons", () => {
  it("shows refusal as a second alert without refreshing and clears it when state changes", async () => {
    const fetch = serve();
    fetch.mockResolvedValue(response({ error: REFUSED }, 409));
    const { refresh, rerender } = block(PAUSED);
    click("Check focus again");
    await screen.findByText(REFUSED);
    expect(screen.getAllByRole("alert").map((node) => node.textContent)).toEqual([FOCUS_MESSAGE, REFUSED]);
    expect(refresh).not.toHaveBeenCalled();
    rerender(
      <ImagingRunBlock
        view={{ ...READY, run: { ...PAUSED, state: "solving", message: "Solving imaging frame for M42." } }}
        refresh={refresh}
      />,
    );
    expect(screen.queryByText(REFUSED)).toBeNull();
  });

  it("clears refusal when Go anyway starts, disables all buttons and reports no answer in its own words", async () => {
    const request = pending();
    const { refresh } = block(PAUSED);
    click("Check focus again");
    expect(button("Checking…").disabled).toBe(true);
    expect(button("Go anyway").disabled).toBe(true);
    expect(button("Stop imaging run").disabled).toBe(true);
    click("Go anyway");
    expect(request.fetch).toHaveBeenCalledOnce();
    await act(async () => request.answer(response({ error: REFUSED }, 409)));
    expect(screen.getAllByRole("alert").map((node) => node.textContent)).toEqual([FOCUS_MESSAGE, REFUSED]);
    click("Go anyway");
    expect(screen.queryByText(REFUSED)).toBeNull();
    expect(button("Go anyway").disabled).toBe(true);
    expect(button("Check focus again").disabled).toBe(true);
    expect(button("Stop imaging run").disabled).toBe(true);
    await act(async () => request.answer(response()));
    expect(refresh).toHaveBeenCalledOnce();
    request.fetch.mockRejectedValueOnce(new TypeError("browser's network failure"));
    click("Check focus again");
    await screen.findByText(NO_ANSWER);
    expect(screen.getAllByRole("alert").map((node) => node.textContent)).toEqual([FOCUS_MESSAGE, NO_ANSWER]);
    expect(refresh).toHaveBeenCalledOnce();
  });

  it("also clears a refusal when the run id changes", async () => {
    const fetch = serve();
    fetch.mockResolvedValue(response({ error: REFUSED }, 409));
    const { refresh, rerender } = block(PAUSED);
    click("Check focus again");
    await screen.findByText(REFUSED);
    rerender(<ImagingRunBlock view={{ ...READY, run: { ...PAUSED, id: "another-run" } }} refresh={refresh} />);
    expect(screen.queryByText(REFUSED)).toBeNull();
  });
});

it("US9-S12: uses the console's elapsed time for pause and stacking clocks", () => {
  const { refresh, rerender } = block({ ...PAUSED, elapsed_s: 25.4, pause_ends_at: 1768529700 });
  paragraph("Focus aid open below · 14:34 left");
  rerender(
    <ImagingRunBlock
      view={{ ...READY, run: { ...PAUSED, elapsed_s: 925.4, pause_ends_at: 1768529700 } }}
      refresh={refresh}
    />,
  );
  paragraph("Focus aid open below · 0:00 left");
  rerender(<ImagingRunBlock view={{ ...READY, run: { ...STACKING, stacking_since: 1768528860 } }} refresh={refresh} />);
  paragraph("Stacking for 1:05 of 120:00");
  paragraph("Elapsed 2:05");
  rerender(
    <ImagingRunBlock
      view={{
        ...READY,
        run: { ...STACKING, stacking_since: 1768528860, settings: { ...STACKING.settings, duration_s: 1800 } },
      }}
      refresh={refresh}
    />,
  );
  paragraph("Stacking for 1:05 of 30:00");
  paragraph("Elapsed 2:05");
});

describe("US9-S13: Start posts only valid optional limits", () => {
  it.each([
    ["", "", true, '{"keep_subs":true}'],
    ["50", "30", false, '{"subs":50,"duration_s":1800,"keep_subs":false}'],
    ["0", "0", true, '{"keep_subs":true}'],
    ["2.5", "", true, '{"keep_subs":true}'],
    ["-2", "-1", true, '{"keep_subs":true}'],
  ] as const)("posts subs %s, minutes %s, keep %s", async (subs, minutes, keep, body) => {
    const fetch = serve();
    const { refresh } = block();
    type("Subs to stack", subs);
    type("Minutes to stack", minutes);
    if (!keep) clickCheckbox();
    click("Start imaging run");
    await waitFor(() => expect(refresh).toHaveBeenCalledOnce());
    expect(fetch).toHaveBeenCalledExactlyOnceWith("/api/imaging/run", post(body));
    expect(fetch.mock.invocationCallOrder[0]!).toBeLessThan(refresh.mock.invocationCallOrder[0]!);
  });

  it("retains drafts across a new view and disables Starting until its answer", async () => {
    const request = pending();
    const { refresh, rerender } = block();
    type("Subs to stack", "50");
    rerender(<ImagingRunBlock view={{ ...READY }} refresh={refresh} />);
    click("Start imaging run");
    expect(request.fetch).toHaveBeenCalledExactlyOnceWith("/api/imaging/run", post('{"subs":50,"keep_subs":true}'));
    expect(button("Starting…").disabled).toBe(true);
    expect(refresh).not.toHaveBeenCalled();
    await act(async () => request.answer(response()));
    expect(refresh).toHaveBeenCalledOnce();
  });

  it("preserves the minute and checkbox drafts as well", async () => {
    const fetch = serve();
    const { refresh, rerender } = block();
    type("Minutes to stack", "30");
    clickCheckbox();
    rerender(<ImagingRunBlock view={{ ...READY }} refresh={refresh} />);
    click("Start imaging run");
    await waitFor(() => expect(refresh).toHaveBeenCalledOnce());
    expect(fetch).toHaveBeenCalledExactlyOnceWith("/api/imaging/run", post('{"duration_s":1800,"keep_subs":false}'));
  });

  it("shows a refused start without refreshing and removes its alert when a run appears", async () => {
    const fetch = serve();
    fetch.mockResolvedValue(response({ error: "no target is selected" }, 409));
    const { refresh, rerender } = block();
    click("Start imaging run");
    await screen.findByRole("alert");
    text("alert", "no target is selected");
    expect(refresh).not.toHaveBeenCalled();
    rerender(<ImagingRunBlock view={{ ...READY, run: STACKING }} refresh={refresh} />);
    expect(screen.queryByRole("alert")).toBeNull();
  });
});

function clickCheckbox() {
  fireEvent.click(screen.getByRole("checkbox", { name: "Keep every sub" }));
}

describe("the three API calls", () => {
  it.each([
    ["start", () => postImagingRun({}), "/api/imaging/run", post("{}")],
    ["stop", () => deleteImagingRun(), "/api/imaging/run", { method: "DELETE" }],
    ["continue", () => postImagingRunContinue(false), "/api/imaging/run/continue", post('{"anyway":false}')],
  ] as const)(
    "%s returns the view, preserves refusal words and replaces network errors",
    async (_name, call, url, options) => {
      const fetch = serve();
      expect(await call()).toEqual(READY);
      expect(fetch).toHaveBeenCalledExactlyOnceWith(url, options);
      fetch.mockResolvedValueOnce(response({ error: "this console runs no imaging runs" }, 409));
      await expect(call()).rejects.toThrow("this console runs no imaging runs");
      fetch.mockRejectedValueOnce(new TypeError("browser network error"));
      await expect(call()).rejects.toThrow(NO_ANSWER);
    },
  );

  it("orders the start keys independently of the caller's object order", async () => {
    const fetch = serve();
    await postImagingRun({ keep_subs: false, duration_s: 1800, subs: 50 });
    expect(fetch).toHaveBeenCalledExactlyOnceWith(
      "/api/imaging/run",
      post('{"subs":50,"duration_s":1800,"keep_subs":false}'),
    );
  });
});
