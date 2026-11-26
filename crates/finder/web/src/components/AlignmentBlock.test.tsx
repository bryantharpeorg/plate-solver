// US5-S1–S4. Oracles: the scenario's literal words and the line from (728, 544)
// to (-300, -2000), intersected with the top edge of a 1456 × 1088 frame.
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import type { ImagingAlign, ImagingAlignCentre, ImagingView } from "../api";
import { AlignmentBlock, aimLine, centreLine } from "./AlignmentBlock";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

const READY: ImagingView = { state: "ready" };
const RUNNING: ImagingAlign = { state: "running", since: 1768528800, exposure_ms: 500, frames: 0 };
const CENTRE: ImagingAlignCentre = {
  x: 700,
  y: 420,
  finder_width: 1456,
  finder_height: 1088,
  right_deg: -0.23,
  up_deg: 1.03,
  separation_deg: 1.05,
  on_frame: true,
  middle: true,
};
const measured = (centre = CENTRE): ImagingAlign => ({
  ...RUNNING,
  frames: 2,
  last: { frame_id: 2, at: 1768528801, outcome: "measured", centre, aim: "replaced", done: true },
});
const view = (align: ImagingAlign): ImagingView => ({ ...READY, align });
const response = (body: unknown = READY, status = 200) => new Response(JSON.stringify(body), { status });

it("US5-S1: words the four places and the aim point's word exactly", () => {
  expect(
    centreLine(
      { ...CENTRE, right_deg: -6.49, up_deg: 1.03, separation_deg: 6.57, on_frame: false, middle: false },
      false,
    ),
  ).toBe(
    "The imaging camera points 6.5° left and 1.0° up of the finder camera's centre, as the finder camera sees it: 6.6° in all, off the finder camera's frame.",
  );
  expect(centreLine({ ...CENTRE, right_deg: 4.72, up_deg: -3.77, separation_deg: 6.03, middle: false }, false)).toBe(
    "The imaging camera points 4.7° right and 3.8° down of the finder camera's centre, as the finder camera sees it: 6.0° in all, on the finder camera's frame.",
  );
  expect(centreLine({ ...CENTRE, right_deg: 1.42, up_deg: -1.29, separation_deg: 1.92 }, false)).toBe(
    "The imaging camera points 1.4° right and 1.3° down of the finder camera's centre, as the finder camera sees it: 1.9° in all, in the middle of the finder camera's frame. Done once the next frame holds it there.",
  );
  expect(centreLine(CENTRE, true)).toBe(
    "Done: the imaging camera's centre has held in the middle of the finder camera's frame. The imaging camera points 0.2° left and 1.0° up of the finder camera's centre, as the finder camera sees it: 1.1° in all.",
  );
  expect(aimLine("replaced")).toBe("The aim point is measured from this frame (replaced).");
  expect(aimLine("no_live_pointing")).toBe(
    "The aim point is not measured from this frame (no_live_pointing); a later imaging solve measures it.",
  );
});

it("US5-S2: shows the first-frame status, unsolved causes, no live solve and measured aim", () => {
  const refresh = vi.fn();
  const { rerender } = render(<AlignmentBlock view={view(RUNNING)} refresh={refresh} />);
  expect(screen.getByRole("status").textContent).toBe("Taking the first imaging frame.");
  const unsolved: ImagingAlign = {
    ...RUNNING,
    frames: 1,
    last: {
      frame_id: 7,
      at: 1768528801,
      outcome: "not_solved",
      error: "the imaging solver found no solution",
      unsolved: 1,
      done: false,
    },
  };
  rerender(<AlignmentBlock view={view(unsolved)} refresh={refresh} />);
  expect(screen.getByRole("status").textContent).toBe(
    "Imaging frame 7 did not solve: the imaging solver found no solution. The next frame looks again.",
  );
  rerender(
    <AlignmentBlock
      view={view({ ...unsolved, frames: 3, last: { ...unsolved.last!, frame_id: 9, unsolved: 3 } })}
      refresh={refresh}
    />,
  );
  expect(screen.getByRole("status").textContent).toBe(
    "Imaging frame 9 did not solve: the imaging solver found no solution. 3 frames in a row have not solved: the imaging camera may be more than 10° from the finder camera's centre, the sky may hold too few stars, or the field height may be wrong. The next frame looks again.",
  );
  rerender(
    <AlignmentBlock
      view={view({
        ...RUNNING,
        frames: 4,
        last: { frame_id: 10, at: 1768528802, outcome: "no_live_solve", done: false },
      })}
      refresh={refresh}
    />,
  );
  expect(screen.getByRole("status").textContent).toBe("The finder camera had no live solve for imaging frame 10.");
  rerender(<AlignmentBlock view={view(measured())} refresh={refresh} />);
  expect(screen.getByText("The aim point is measured from this frame (replaced).")).toBeTruthy();
  expect(screen.getByRole("status").textContent).toBe(centreLine(CENTRE, true));
});

it("US5-S3: waits for the view, starts and stops with a refresh, and holds the off-frame point along its ray", async () => {
  const fetch = vi.fn(async (_input: RequestInfo | URL, _init?: RequestInit) => response());
  vi.stubGlobal("fetch", fetch);
  const refresh = vi.fn();
  const { rerender } = render(<AlignmentBlock view={null} refresh={refresh} />);
  expect(screen.getByText("Reading the imaging camera.")).toBeTruthy();
  expect(screen.queryByRole("button")).toBeNull();
  rerender(<AlignmentBlock view={READY} refresh={refresh} />);
  fireEvent.click(screen.getByRole("button", { name: "Start aligning" }));
  await waitFor(() => expect(refresh).toHaveBeenCalledTimes(1));
  expect(fetch).toHaveBeenNthCalledWith(1, "/api/imaging/align", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: "{}",
  });
  rerender(
    <AlignmentBlock
      view={view(measured({ ...CENTRE, x: -300, y: -2000, on_frame: false, middle: false }))}
      refresh={refresh}
    />,
  );
  const drawing = screen.getByRole("img", { name: "The imaging camera's centre off the finder camera's frame" });
  const circle = drawing.querySelector("circle")!;
  expect(Math.abs(Number(circle.getAttribute("cy")) - 12)).toBeLessThan(1e-6);
  expect(Math.abs(Number(circle.getAttribute("cx")) - (12 + (160 * (728 - (1028 * 544) / 2544)) / 1456))).toBeLessThan(
    1e-6,
  );
  const middle = drawing.querySelector("rect[stroke-dasharray]")!;
  expect(Number(middle.getAttribute("width"))).toBe(80);
  expect(Math.abs(Number(middle.getAttribute("height")) - (80 * 1088) / 1456)).toBeLessThan(1e-6);
  fireEvent.click(screen.getByRole("button", { name: "Stop aligning" }));
  await waitFor(() => expect(refresh).toHaveBeenCalledTimes(2));
  expect(fetch).toHaveBeenLastCalledWith("/api/imaging/align", { method: "DELETE" });
});

it("US5-S4: reports a failed alignment and offers a new start", () => {
  render(
    <AlignmentBlock
      view={view({ ...RUNNING, state: "ended", ended: "failed", error: "the frame failed" })}
      refresh={vi.fn()}
    />,
  );
  expect(screen.getByRole("alert").textContent).toBe("The finder alignment stopped: the frame failed.");
  expect(screen.getByRole("button", { name: "Start aligning" })).toBeTruthy();
});

it("shows the route's words when either start or stop is refused", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => response({ error: "the imaging camera is held by the focus aid" }, 409)),
  );
  const refresh = vi.fn();
  const { rerender } = render(<AlignmentBlock view={READY} refresh={refresh} />);
  fireEvent.click(screen.getByRole("button", { name: "Start aligning" }));
  expect((await screen.findByRole("alert")).textContent).toBe("the imaging camera is held by the focus aid");
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => response({ error: "this console takes no imaging frames" }, 409)),
  );
  rerender(<AlignmentBlock view={view(RUNNING)} refresh={refresh} />);
  fireEvent.click(screen.getByRole("button", { name: "Stop aligning" }));
  await waitFor(() => expect(screen.getByRole("alert").textContent).toBe("this console takes no imaging frames"));
});
