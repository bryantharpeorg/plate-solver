// The Preview frames block (058 US4). Oracles: the words, names, roles, requests and
// alerts of US4-S1 to US4-S5 and US4-S7, compared exactly. Time is real: each wait is
// bounded (3 s for a frame or a status, 1.2 s for "no request"), and each test has 15 s.
import { cleanup, fireEvent, render, screen, waitFor, within, type RenderResult } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ImagingFrame, ImagingView } from "../api";
import { ImagingPage } from "./ImagingPage";
import { PreviewBlock } from "./PreviewBlock";

const TIMEOUT = 15_000;
const FRAME_WAIT = { timeout: 3_000 };
const WORDS =
  "Imaging frames one after another at the settings above, shown and never saved, to focus or frame by eye. " +
  "The whole frame shows under Imaging frame, and its brightest part here, one pixel a cell.";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  Object.defineProperty(document, "visibilityState", { configurable: true, get: () => "visible" });
});

const crop = (id: number) => `Crop of preview frame ${id} around its brightest part`;
const wait = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

const view = (frame: ImagingFrame): ImagingView => ({ state: "ready", frame });

/** A preview frame in flight, as the console answers it. */
const exposing = (id: number, purpose = "preview"): ImagingFrame => ({
  id,
  state: "exposing",
  purpose,
  exposure_ms: 200,
});

/** A preview frame the console has shown, with its crop. */
const shown = (id: number): ImagingFrame => ({
  id,
  state: "shown",
  purpose: "preview",
  exposure_ms: 200,
  preview: `/api/imaging/frames/${id}/preview.jpg`,
  crop: `/api/imaging/frames/${id}/crop.jpg`,
});

type Answer = Response | "unreachable";

/**
 * The stub console. Each POST takes the next frame id (1, 2, …) and answers it exposing;
 * each read answers the frame last taken as shown, unless `read` says otherwise.
 * `post` answers the n-th POST (1-based) itself, or undefined to take the frame, and
 * `read` answers the n-th read. "unreachable" makes the call reject, as a request that
 * never reaches the console does.
 */
function stubConsole(
  options: {
    read?: (last: number, reads: number) => ImagingFrame | "unreachable";
    post?: (n: number) => Answer | undefined;
  } = {},
) {
  let posts = 0;
  let reads = 0;
  let last = 0;
  const fetch = vi.fn(async (input: RequestInfo | URL, _init?: RequestInit) => {
    const url = String(input);
    if (url === "/api/imaging/preview") {
      posts += 1;
      const answer = options.post?.(posts);
      if (answer === "unreachable") throw new TypeError("Failed to fetch");
      if (answer != null) return answer;
      last = posts;
      return new Response(JSON.stringify(view(exposing(last))));
    }
    if (url === "/api/imaging") {
      reads += 1;
      const frame = options.read?.(last, reads) ?? shown(last);
      if (frame === "unreachable") throw new TypeError("Failed to fetch");
      return new Response(JSON.stringify(view(frame)));
    }
    throw new Error(`the block asked for ${url}, which the stub console does not hold`);
  });
  vi.stubGlobal("fetch", fetch);
  const postCount = () => fetch.mock.calls.filter(([input]) => String(input) === "/api/imaging/preview").length;
  return { fetch, postCount };
}

const region = () => screen.getByRole("region", { name: "Preview frames" });
const button = (name: string) => within(region()).getByRole("button", { name }) as HTMLButtonElement;
const status = () => within(region()).queryByRole("status");

describe("the Preview frames block: frames, stopping and starting", () => {
  it(
    "shows frames one after another, stops, and starts again with no picture",
    async () => {
      const stub = stubConsole();
      render(<PreviewBlock />);
      expect(within(region()).getByText(WORDS)).toBeTruthy();

      fireEvent.click(button("Start preview frames"));
      expect(status()?.textContent).toBe("Waiting for the first preview frame");

      const first = await within(region()).findByRole("img", { name: crop(1) }, FRAME_WAIT);
      expect(first.getAttribute("src")).toBe("/api/imaging/frames/1/crop.jpg");
      expect(status()?.textContent).toBe("Preview frame 1 · not saved");

      await within(region()).findByRole("img", { name: crop(2) }, FRAME_WAIT);
      expect(status()?.textContent).toMatch(/^Preview frame 2 · every \d\.\d s · not saved$/);
      const posts = stub.fetch.mock.calls.filter(([, init]) => init?.method === "POST");
      expect(posts.length).toBeGreaterThanOrEqual(2);
      expect(posts.every(([input]) => String(input) === "/api/imaging/preview")).toBe(true);

      fireEvent.click(button("Stop preview frames"));
      expect(status()?.textContent).toBe("Stopping: the last preview frame is still in flight");
      await waitFor(() => expect(status()).toBeNull(), FRAME_WAIT);

      const stopped = stub.postCount();
      await wait(1_200);
      expect(stub.postCount()).toBe(stopped);
      expect(button("Start preview frames")).toBeTruthy();
      expect(within(region()).queryByRole("alert")).toBeNull();

      fireEvent.click(button("Start preview frames"));
      expect(status()?.textContent).toBe("Waiting for the first preview frame");
      expect(within(region()).queryByRole("img")).toBeNull();

      const fourth = await within(region()).findByRole("img", { name: crop(4) }, FRAME_WAIT);
      expect(fourth.getAttribute("src")).toBe("/api/imaging/frames/4/crop.jpg");
      expect(status()?.textContent).toBe("Preview frame 4 · not saved");
    },
    TIMEOUT,
  );

  it(
    "stops at once with the route's words when the POST is refused",
    async () => {
      const fetch = vi.fn(
        async () => new Response(JSON.stringify({ error: "the imaging camera is held by the stack" }), { status: 409 }),
      );
      vi.stubGlobal("fetch", fetch);
      render(<PreviewBlock />);
      fireEvent.click(button("Start preview frames"));
      expect((await within(region()).findByRole("alert")).textContent).toBe(
        "Preview frames stopped: the imaging camera is held by the stack.",
      );
      expect(button("Start preview frames")).toBeTruthy();
      expect(fetch).toHaveBeenCalledTimes(1);
    },
    TIMEOUT,
  );

  it(
    "stops with the words of a failed frame, an aborted frame and an overtaking frame",
    async () => {
      const rigs: Array<{ options: Parameters<typeof stubConsole>[0]; alert: string; posts: number }> = [
        {
          options: {
            read: (last) => ({ ...exposing(last), state: "failed", error: "the imaging frame stalled" }),
          },
          alert: "Preview frames stopped: the imaging frame stalled.",
          posts: 1,
        },
        {
          options: { read: (last) => ({ ...exposing(last), state: "aborted" }) },
          alert: "Preview frames stopped: the exposure was aborted.",
          posts: 1,
        },
        {
          options: {
            post: (n) =>
              n === 2
                ? new Response(JSON.stringify({ error: "the imaging camera is held by the focus aid" }), {
                    status: 409,
                  })
                : undefined,
            read: (last) => exposing(last + 1, "focus"),
          },
          alert: "Preview frames stopped: the imaging camera is held by the focus aid.",
          posts: 2,
        },
      ];
      for (const rig of rigs) {
        const stub = stubConsole(rig.options);
        const block: RenderResult = render(<PreviewBlock />);
        fireEvent.click(button("Start preview frames"));
        expect((await within(region()).findByRole("alert", undefined, FRAME_WAIT)).textContent).toBe(rig.alert);
        expect(stub.postCount()).toBe(rig.posts);
        expect(within(region()).queryByRole("img")).toBeNull();
        block.unmount();
      }
    },
    TIMEOUT,
  );

  it(
    "pauses while the page is hidden, and asks for the next frame once it is shown",
    async () => {
      const stub = stubConsole();
      const block = render(<PreviewBlock />);
      fireEvent.click(button("Start preview frames"));
      await within(region()).findByRole("img", { name: crop(1) }, FRAME_WAIT);

      setVisibility("hidden");
      expect(button("Stop preview frames")).toBeTruthy();
      const hiddenAt = stub.fetch.mock.calls.length;
      const postsAtHidden = stub.postCount();
      await wait(1_200);
      expect(stub.fetch.mock.calls.length).toBe(hiddenAt);

      setVisibility("visible");
      const second = await within(region()).findByRole("img", { name: crop(2) }, FRAME_WAIT);
      expect(second.getAttribute("src")).toBe("/api/imaging/frames/2/crop.jpg");
      await waitFor(() => expect(stub.postCount()).toBeGreaterThan(postsAtHidden), FRAME_WAIT);

      block.unmount();
      const unmountedAt = stub.fetch.mock.calls.length;
      await wait(1_200);
      expect(stub.fetch.mock.calls.length).toBe(unmountedAt);
    },
    TIMEOUT,
  );

  it(
    "shows the Preview frames block after Imaging frame on the Imaging page",
    async () => {
      vi.stubGlobal(
        "fetch",
        vi.fn(async () => new Response(JSON.stringify(IMAGING_VIEW))),
      );
      render(<ImagingPage night={false} onBack={() => {}} />);
      const imagingFrame = await screen.findByRole("region", { name: "Imaging frame" });
      const preview = screen.getByRole("region", { name: "Preview frames" });
      expect(imagingFrame.compareDocumentPosition(preview) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
      const picture = within(imagingFrame).getByRole("img", { name: "Imaging frame 7" });
      expect(picture.getAttribute("src")).toBe("/api/imaging/frames/7/preview.jpg");
      expect(within(preview).getByRole("button", { name: "Start preview frames" })).toBeTruthy();
    },
    TIMEOUT,
  );

  it(
    "shows the console's words, not a picture, when the console does not answer a POST",
    async () => {
      const stub = stubConsole({
        read: (last, reads) => (reads === 1 ? exposing(last) : reads <= 5 ? "unreachable" : shown(last)),
        post: (n) => (n === 2 ? "unreachable" : undefined),
      });
      render(<PreviewBlock />);
      fireEvent.click(button("Start preview frames"));
      await waitFor(() => expect(status()?.textContent).toBe("Waiting for the console"), FRAME_WAIT);

      expect((await within(region()).findByRole("alert", undefined, FRAME_WAIT)).textContent).toBe(
        "Preview frames stopped: the console did not answer.",
      );
      const picture = within(region()).getByRole("img", { name: crop(1) });
      expect(picture.getAttribute("src")).toBe("/api/imaging/frames/1/crop.jpg");
      expect(button("Start preview frames")).toBeTruthy();
      expect(stub.postCount()).toBe(2);
    },
    TIMEOUT,
  );
});

/** The stub console's view of a ready imaging camera whose frame 7 is shown (US4-S5). */
const IMAGING_VIEW: ImagingView = {
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
    exposure_ms: 200,
    gain: 120,
    offset: 10,
    binning: 1,
    roi: { x: 0, y: 0, width: 1236, height: 822 },
    cooler_on: false,
    set_temp_c: null,
    forward_kib_per_s: null,
    save_at_node: false,
  },
  frame: { ...shown(7) },
};

/** Set `document.visibilityState` and tell the page, as the browser does. */
function setVisibility(state: "hidden" | "visible") {
  Object.defineProperty(document, "visibilityState", { configurable: true, get: () => state });
  document.dispatchEvent(new Event("visibilitychange"));
}
