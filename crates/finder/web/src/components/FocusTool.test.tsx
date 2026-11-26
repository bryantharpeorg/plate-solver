// US4-S1–S4. Oracles: the story's exact words, requests and worked readings;
// the history leaves a gap for null and keeps only the last 60 frames.
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import type { FinderFocus, ImagingView } from "../api";
import { finderStatus, FocusTool } from "./FocusTool";

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

const aid = (over: Partial<FinderFocus> = {}): FinderFocus => ({
  state: "running",
  since: 0,
  readings: 0,
  found: false,
  history: [],
  ...over,
});

it("US4-S1: describes the last finder camera frame with the exact status and detected-count hint", () => {
  expect(finderStatus(aid())).toBe("Waiting for the first finder camera frame");
  expect(
    finderStatus(
      aid({ last: { frame_id: 1, state: "reading", stars: 15, detected: 31 }, median: { value: 2.83, readings: 3 } }),
    ),
  ).toBe("Median of the last 3 finder camera frames · 15 stars");
  expect(
    finderStatus(
      aid({ last: { frame_id: 2, state: "reading", stars: 9, detected: 31 }, median: { value: 2.8, readings: 2 } }),
    ),
  ).toBe("2 of 3 finder camera frames since the last change · 9 stars");
  expect(finderStatus(aid({ last: { frame_id: 3, state: "settling", stars: 15, detected: 31 } }))).toBe(
    "Settling after a change",
  );
  expect(finderStatus(aid({ last: { frame_id: 4, state: "too_few_stars", stars: 2, detected: 3 } }))).toBe(
    "No reading: 2 stars measured, and 5 are needed. Far out of focus the stars vanish: turn the focuser slowly until they show.",
  );
  expect(finderStatus(aid({ last: { frame_id: 5, state: "too_few_stars", stars: 2, detected: 31 } }))).toBe(
    "No reading: 2 of the 31 stars detected could be measured, and 5 are needed: the rest are saturated, crowded or at the frame's edge.",
  );
  expect(finderStatus(aid({ last: { frame_id: 6, state: "far_out", value: 21.2, stars: 15, detected: 31 } }))).toBe(
    "Far out of focus: stars about 21.20 px across",
  );
  expect(finderStatus(aid({ last: { frame_id: 7, state: "too_few_stars", stars: 2, detected: 5 } }))).toBe(
    "No reading: 2 of the 5 stars detected could be measured, and 5 are needed: the rest are saturated, crowded or at the frame's edge.",
  );
});

it("US4-S2: starts and stops the finder camera's aid, showing its reading, detected stars and bounded history", async () => {
  const running = aid({
    readings: 5,
    median: { value: 2.83, readings: 3 },
    last: { frame_id: 5, state: "reading", value: 2.83, stars: 15, detected: 31 },
    best: 2.7,
    trend: "better",
    found: true,
    history: [5.9, 6.1, null, 2.8, 2.83],
  });
  let current: FinderFocus | null = null;
  let firstRead!: (response: Response) => void;
  const first = new Promise<Response>((resolve) => {
    firstRead = resolve;
  });
  const fetch = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    expect(String(input)).toBe("/api/finder-camera/focus");
    if (init?.method === "POST") current = running;
    else if (init?.method === "DELETE") current = { ...running, state: "ended", ended: "stopped" };
    else if (current == null) return first;
    return new Response(JSON.stringify({ focus: current }));
  });
  vi.stubGlobal("fetch", fetch);
  const mounted = render(<FocusTool view={null} refresh={vi.fn()} />);
  const region = screen.getByRole("region", { name: "Focus a camera" });
  const block = within(region);
  expect(block.getByText("Reading the finder camera's focus aid.")).toBeTruthy();
  firstRead(new Response(JSON.stringify({ focus: null })));
  fireEvent.click(await block.findByRole("button", { name: "Start the finder camera's focus aid" }));
  await block.findByText("2.83 px");
  expect(fetch).toHaveBeenCalledWith("/api/finder-camera/focus", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: "{}",
  });
  expect(block.getByText("Focus reading")).toBeTruthy();
  expect(block.getByRole("status").textContent).toBe("Median of the last 3 finder camera frames · 15 stars");
  expect(block.getByText("Stars detected 31")).toBeTruthy();
  expect(block.getByText("Best so far 2.70 px")).toBeTruthy();
  expect(block.getByText("Sharper than before the last change")).toBeTruthy();
  expect(block.getByText("Passed through focus and came back to the best so far.")).toBeTruthy();
  const bars = block
    .getByRole("img", { name: "Focus readings of the last 5 finder camera frames" })
    .querySelectorAll("rect");
  expect(bars.length).toBe(4);
  expect(Array.from(bars).map((bar) => bar.getAttribute("x"))).toEqual(["0", "4", "12", "16"]);
  expect(Array.from(bars).every((bar) => bar.getAttribute("fill") === "currentColor")).toBe(true);
  fireEvent.click(block.getByRole("button", { name: "Stop the finder camera's focus aid" }));
  await waitFor(() => expect(block.getByRole("status").textContent).toBe("The finder camera's focus aid stopped."));
  expect(fetch).toHaveBeenCalledWith("/api/finder-camera/focus", { method: "DELETE" });
  mounted.unmount();
  current = aid({ readings: 61, history: Array.from({ length: 61 }, (_, i) => i + 1) });
  render(<FocusTool view={null} refresh={vi.fn()} />);
  const history = await screen.findByRole("img", { name: "Focus readings of the last 60 finder camera frames" });
  expect(history.querySelectorAll("rect").length).toBe(60);
});

it("US4-S3: shows a failed run's words as an alert and offers a new start, without a status", async () => {
  vi.stubGlobal(
    "fetch",
    async () =>
      new Response(
        JSON.stringify({
          focus: aid({ state: "ended", ended: "failed", error: "no raw frame arrived within 15 s" }),
        }),
      ),
  );
  render(<FocusTool view={null} refresh={vi.fn()} />);
  const block = within(screen.getByRole("region", { name: "Focus a camera" }));
  expect((await block.findByRole("alert")).textContent).toBe(
    "The finder camera's focus aid stopped: no raw frame arrived within 15 s.",
  );
  expect(block.getByRole("button", { name: "Start the finder camera's focus aid" })).toBeTruthy();
  expect(block.queryByRole("status")).toBeNull();
});

it("US4-S4: picking the imaging camera shows the existing Focus block once its view is ready", () => {
  vi.stubGlobal("fetch", async () => new Response(JSON.stringify({ focus: null })));
  const refresh = vi.fn();
  const mounted = render(<FocusTool view={null} refresh={refresh} />);
  const block = within(screen.getByRole("region", { name: "Focus a camera" }));
  fireEvent.change(block.getByRole("combobox", { name: "Camera" }), { target: { value: "imaging" } });
  expect(block.getByText("The imaging camera's focus aid shows here once the imaging camera is ready.")).toBeTruthy();
  const view: ImagingView = { state: "ready", focus: { unit: "px" } };
  mounted.rerender(<FocusTool view={view} refresh={refresh} />);
  expect(block.getByRole("heading", { name: "Focus", level: 2 })).toBeTruthy();
  expect(block.getByRole("button", { name: "Start focus aid", exact: true })).toBeTruthy();
  expect(
    block.getByText(/^A focus reading is the size of the stars in the focus region: smaller is sharper\./),
  ).toBeTruthy();
  expect(block.queryByRole("button", { name: "Start the finder camera's focus aid" })).toBeNull();
});

it("FR-014: reads at once and every second while the finder camera is selected, clearing its timer on leaving", async () => {
  vi.useFakeTimers();
  const fetch = vi.fn(async () => new Response(JSON.stringify({ focus: aid() })));
  vi.stubGlobal("fetch", fetch);
  const mounted = await act(async () => render(<FocusTool view={null} refresh={vi.fn()} />));
  expect(fetch).toHaveBeenCalledTimes(1);
  expect(fetch).toHaveBeenLastCalledWith("/api/finder-camera/focus");
  expect(screen.getByText("—")).toBeTruthy();
  expect(screen.getByRole("status").textContent).toBe("Waiting for the first finder camera frame");
  await act(async () => {
    await vi.advanceTimersByTimeAsync(999);
  });
  expect(fetch).toHaveBeenCalledTimes(1);
  await act(async () => {
    await vi.advanceTimersByTimeAsync(1);
  });
  expect(fetch).toHaveBeenCalledTimes(2);
  fireEvent.change(screen.getByRole("combobox", { name: "Camera" }), { target: { value: "imaging" } });
  await act(async () => {
    await vi.advanceTimersByTimeAsync(2000);
  });
  expect(fetch).toHaveBeenCalledTimes(2);
  await act(async () => {
    fireEvent.change(screen.getByRole("combobox", { name: "Camera" }), { target: { value: "finder" } });
  });
  expect(fetch).toHaveBeenCalledTimes(3);
  mounted.unmount();
  await act(async () => {
    await vi.advanceTimersByTimeAsync(2000);
  });
  expect(fetch).toHaveBeenCalledTimes(3);
});

it("FR-014: shows a refused start or stop's own words as an alert", async () => {
  let current: FinderFocus | null = null;
  vi.stubGlobal("fetch", async (_input: RequestInfo | URL, init?: RequestInit) => {
    if (init?.method === "POST")
      return new Response(
        JSON.stringify({ error: "the finder camera's exposure or gain is automatic: fix both on Setup" }),
        { status: 409 },
      );
    if (init?.method === "DELETE")
      return new Response(JSON.stringify({ error: "writes require the console's own origin and an allowed host" }), {
        status: 403,
      });
    return new Response(JSON.stringify({ focus: current }));
  });
  const mounted = render(<FocusTool view={null} refresh={vi.fn()} />);
  fireEvent.click(await screen.findByRole("button", { name: "Start the finder camera's focus aid" }));
  expect((await screen.findByRole("alert")).textContent).toBe(
    "the finder camera's exposure or gain is automatic: fix both on Setup",
  );
  expect(screen.getByRole("button", { name: "Start the finder camera's focus aid" })).toBeTruthy();
  mounted.unmount();
  current = aid();
  render(<FocusTool view={null} refresh={vi.fn()} />);
  fireEvent.click(await screen.findByRole("button", { name: "Stop the finder camera's focus aid" }));
  expect((await screen.findByRole("alert")).textContent).toBe(
    "writes require the console's own origin and an allowed host",
  );
  expect(screen.getByRole("button", { name: "Stop the finder camera's focus aid" })).toBeTruthy();
});
