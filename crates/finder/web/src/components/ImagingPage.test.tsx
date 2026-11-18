// The Imaging page (031 US7). Oracles: the imaging view of US1-S1, the worked
// time in hand of US7-S4 (30.3 s less the 2 s exposure), the 10 s hold limit of
// owner decision 11, and the PUT key order of plan.md § The page.
import { act, cleanup, fireEvent, render, renderHook, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ImagingFrame, ImagingView, ObservingSession } from "../api";
import { App } from "../App";
import { COLOUR_HOLD_MS, useColourHold } from "../useColourHold";
import { ImagingPage } from "./ImagingPage";

afterEach(() => {
  cleanup();
  window.history.replaceState(null, "", "/");
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

const NODE: NonNullable<ObservingSession["node"]> = {
  address: "scope.local:1",
  token_set: true,
  connected: true,
  last_frame_age_s: 0,
  offset_s: null,
  camera: null,
  stars: 30,
  out_of_range: [],
};

// The view of US1-S1: the simulated imaging camera as the console relays it.
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
};

const frame = (over: Partial<ImagingFrame>): ImagingFrame => ({
  id: 1,
  state: "saved",
  purpose: "frame",
  exposure_ms: 2000,
  ...over,
});

/** Answer every route with `view`, and `refusals[url]()` for the routes it names. */
function serve(view: unknown, refusals: Record<string, () => Response> = {}) {
  const fetch = vi.fn(async (input: RequestInfo | URL, _init?: RequestInit) => {
    const answer = refusals[String(input)];
    return answer ? answer() : new Response(JSON.stringify(view));
  });
  vi.stubGlobal("fetch", fetch);
  return fetch;
}

/** Answer the App's routes: the imaging view, a session, and samples. */
function serveApp(session: object, nodePaired: boolean) {
  vi.stubGlobal("fetch", async (input: RequestInfo | URL) => {
    const url = String(input);
    const body =
      url === "/api/imaging"
        ? READY
        : url === "/api/session"
          ? session
          : url === "/api/samples"
            ? []
            : { status: "ok", version: "test", database: { catalog: "hip", stars: 1 }, samples: 0 };
    return new Response(JSON.stringify(body));
  });
  if (nodePaired) {
    vi.stubGlobal(
      "EventSource",
      class {
        addEventListener() {}
        removeEventListener() {}
        close() {}
      },
    );
  }
}

describe("the /imaging route", () => {
  it("opens with the Imaging link current, after Setup, and goes back", async () => {
    serveApp({ site: { lat_deg: 40, lon_deg: -75 }, node: NODE }, true);
    window.history.pushState(null, "", "/imaging");
    render(<App />);
    await screen.findByRole("link", { name: "Imaging" });
    const banner = screen.getAllByRole("banner")[0]!;
    const imaging = within(banner).getByRole("link", { name: "Imaging" });
    const setup = within(banner).getByRole("link", { name: "Setup" });
    expect(imaging.getAttribute("aria-current")).toBe("page");
    expect(Boolean(setup.compareDocumentPosition(imaging) & Node.DOCUMENT_POSITION_FOLLOWING)).toBe(true);
    expect(screen.getByRole("heading", { name: "Imaging" })).toBeTruthy();
    fireEvent.click(screen.getByRole("link", { name: "Back to console" }));
    expect(window.location.pathname).toBe("/");
    expect(screen.queryByRole("heading", { name: "Imaging" })).toBeNull();
  });

  it("shows the Imaging link only while a scope node is paired", async () => {
    serveApp({ site: { lat_deg: 40, lon_deg: -75 } }, false);
    render(<App />);
    await act(async () => {});
    // The session's first read has answered: no node, no Imaging link.
    expect(screen.getByRole("link", { name: "Setup" })).toBeTruthy();
    expect(screen.queryByRole("link", { name: "Imaging" })).toBeNull();
  });

  it("links to /imaging when a node is paired", async () => {
    serveApp({ site: { lat_deg: 40, lon_deg: -75 }, node: NODE }, true);
    render(<App />);
    await screen.findByRole("link", { name: "Imaging" });
    fireEvent.click(screen.getByRole("link", { name: "Imaging" }));
    expect(window.location.pathname).toBe("/imaging");
    expect(screen.getByRole("heading", { name: "Imaging" })).toBeTruthy();
  });
});

describe("ImagingPage's states that are not ready", () => {
  it.each([
    ['{"state":"off"}', { state: "off" }, "The scope node is not connected."],
    ['{"state":"no_link"}', { state: "no_link" }, "The scope node is not connected."],
    [
      '{"state":"no_service"}',
      { state: "no_service" },
      "This scope node has no imaging service. Rebuild it to match this console.",
    ],
    ['{"state":"no_camera"}', { state: "no_camera" }, "This scope node has no imaging camera."],
  ])("%s shows one status line", async (_label, view, words) => {
    serve(view);
    render(<ImagingPage night={false} onBack={() => {}} />);
    expect((await screen.findByRole("status")).textContent).toBe(words);
    expect(screen.queryByRole("region", { name: "Settings" })).toBeNull();
    expect(screen.queryByRole("region", { name: "Imaging frame" })).toBeNull();
  });

  it("shows the state error as an alert", async () => {
    serve({ state: "error", error: "cannot reach the INDI server" });
    render(<ImagingPage night={false} onBack={() => {}} />);
    expect((await screen.findByRole("alert")).textContent).toBe(
      "The imaging camera is not answering: cannot reach the INDI server.",
    );
    expect(screen.queryByRole("region", { name: "Settings" })).toBeNull();
    expect(screen.queryByRole("region", { name: "Imaging frame" })).toBeNull();
  });
});

describe("ImagingPage's camera and settings", () => {
  it("shows the camera, its sensor and the settings the camera has", async () => {
    serve(READY);
    render(<ImagingPage night={false} onBack={() => {}} />);
    const camera = await screen.findByRole("region", { name: "Camera" });
    expect(camera.textContent).toContain("simulated imaging camera · 1236 × 822 px · 19.12 µm · 16-bit · RGGB");
    expect(within(camera).getByRole("status").textContent).toBe("Sensor 20.0 °C · cooler off");
    const settings = screen.getByRole("region", { name: "Settings" });
    for (const name of ["Exposure", "Gain", "Offset", "Set temperature", "Forwarding limit"]) {
      expect(within(settings).getByRole("textbox", { name })).toBeTruthy();
    }
    const binning = within(settings).getByRole("combobox", { name: "Binning" });
    expect(within(binning).getByRole("option", { name: "1 × 1" })).toBeTruthy();
    expect(within(binning).getByRole("option", { name: "2 × 2" })).toBeTruthy();
    expect(screen.getByText("2 × 2 is half resolution: a quarter of the bytes for each imaging frame.")).toBeTruthy();
    expect(screen.getByText("Empty for no limit. From 64 to 1048576 KiB a second.")).toBeTruthy();
    expect(within(settings).getByRole("switch", { name: "Cooler" })).toBeTruthy();
    expect(
      within(settings).getByRole("checkbox", { name: "Also keep each imaging frame at the scope node" }),
    ).toBeTruthy();
    expect(within(settings).getByRole("button", { name: "Save imaging settings" })).toBeTruthy();
  });

  it("saves the settings with one PUT in the stated key order", async () => {
    const fetch = serve(READY);
    render(<ImagingPage night={false} onBack={() => {}} />);
    await screen.findByRole("region", { name: "Settings" });
    fireEvent.change(screen.getByRole("textbox", { name: "Exposure" }), { target: { value: "2000" } });
    fireEvent.change(screen.getByRole("combobox", { name: "Binning" }), { target: { value: "2" } });
    fireEvent.click(screen.getByRole("button", { name: "Save imaging settings" }));
    expect(fetch).toHaveBeenCalledWith("/api/imaging/settings", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        exposure_ms: 2000,
        gain: 120,
        offset: 10,
        binning: 2,
        cooler_on: false,
        forward_kib_per_s: null,
        save_at_node: false,
      }),
    });
    fireEvent.change(screen.getByRole("textbox", { name: "Forwarding limit" }), { target: { value: "4096" } });
    fireEvent.click(screen.getByRole("button", { name: "Save imaging settings" }));
    const bodies = fetch.mock.calls
      .filter(([url]) => String(url) === "/api/imaging/settings")
      .map(([, init]) => JSON.parse(String(init!.body)) as { forward_kib_per_s: number | null });
    expect(bodies.at(-1)!.forward_kib_per_s).toBe(4096);
  });

  it("shows a forwarding limit the settings hold", async () => {
    serve({ ...READY, settings: { ...READY.settings!, forward_kib_per_s: 4096 } });
    render(<ImagingPage night={false} onBack={() => {}} />);
    await screen.findByRole("textbox", { name: "Forwarding limit" });
    // The draft follows the settings once they arrive (they seed it, not the test).
    await waitFor(() =>
      expect((screen.getByRole("textbox", { name: "Forwarding limit" }) as HTMLInputElement).value).toBe("4096"),
    );
  });

  it("shows a refused settings save as an alert", async () => {
    serve(READY, {
      "/api/imaging/settings": () =>
        new Response(JSON.stringify({ error: "gain 301 is outside the camera's range 0–300" }), { status: 400 }),
    });
    render(<ImagingPage night={false} onBack={() => {}} />);
    await screen.findByRole("region", { name: "Settings" });
    fireEvent.click(screen.getByRole("button", { name: "Save imaging settings" }));
    expect((await screen.findByRole("alert")).textContent).toBe("gain 301 is outside the camera's range 0–300");
  });

  it("reads the sensor line with the cooler on and a set temperature", async () => {
    serve({
      ...READY,
      status: { temp_c: -10.0, cooler_power: 75.0, set_temp_c: -10.0, exposing: false },
      settings: { ...READY.settings!, cooler_on: true, set_temp_c: -10.0 },
    });
    render(<ImagingPage night={false} onBack={() => {}} />);
    expect((await screen.findByRole("status")).textContent).toBe("Sensor -10.0 °C · cooler on at 75% · set -10.0 °C");
  });

  it("reads the sensor line after the scope node restarts with the cooler running", async () => {
    serve({
      ...READY,
      status: { temp_c: 5.0, cooler_power: 40.0, set_temp_c: null, exposing: false },
      settings: { ...READY.settings!, cooler_on: true },
    });
    render(<ImagingPage night={false} onBack={() => {}} />);
    expect((await screen.findByRole("status")).textContent).toBe("Sensor 5.0 °C · cooler on at 40%");
  });

  it("shows only the controls a camera has", async () => {
    serve({ ...READY, camera: { ...READY.camera!, gain: null, offset: null, cooler: false, bayer: null } });
    render(<ImagingPage night={false} onBack={() => {}} />);
    const camera = await screen.findByRole("region", { name: "Camera" });
    expect(camera.textContent).toContain("· 16-bit · mono");
    expect(camera.textContent).not.toContain("RGGB");
    const settings = screen.getByRole("region", { name: "Settings" });
    for (const name of ["Gain", "Offset", "Set temperature"]) {
      expect(within(settings).queryByRole("textbox", { name })).toBeNull();
    }
    expect(within(settings).queryByRole("switch", { name: "Cooler" })).toBeNull();
  });
});

describe("ImagingPage's imaging frame", () => {
  it("offers Take imaging frame and posts when clicked", async () => {
    const fetch = serve(READY);
    render(<ImagingPage night={false} onBack={() => {}} />);
    const region = await screen.findByRole("region", { name: "Imaging frame" });
    fireEvent.click(within(region).getByRole("button", { name: "Take imaging frame" }));
    expect(fetch).toHaveBeenCalledWith("/api/imaging/frame", { method: "POST" });
  });

  it("shows a refused frame as an alert", async () => {
    serve(READY, {
      "/api/imaging/frame": () =>
        new Response(JSON.stringify({ error: "an exposure is already in progress" }), { status: 409 }),
    });
    render(<ImagingPage night={false} onBack={() => {}} />);
    const region = await screen.findByRole("region", { name: "Imaging frame" });
    fireEvent.click(within(region).getByRole("button", { name: "Take imaging frame" }));
    expect((await screen.findByRole("alert")).textContent).toBe("an exposure is already in progress");
  });

  it("shows an exposure in progress, with Take disabled, and aborts it", async () => {
    const fetch = serve({ ...READY, frame: frame({ state: "exposing" }) });
    render(<ImagingPage night={false} onBack={() => {}} />);
    const region = await screen.findByRole("region", { name: "Imaging frame" });
    expect(within(region).getByRole("status").textContent).toBe("Exposing for 2000 ms");
    expect((within(region).getByRole("button", { name: "Take imaging frame" }) as HTMLButtonElement).disabled).toBe(
      true,
    );
    fireEvent.click(within(region).getByRole("button", { name: "Abort exposure" }));
    expect(fetch).toHaveBeenCalledWith("/api/imaging/abort", { method: "POST" });
  });

  it("shows the bytes received, with Take disabled and Abort beside it", async () => {
    serve({ ...READY, frame: frame({ state: "receiving", received: 262144, bytes: 2031984 }) });
    render(<ImagingPage night={false} onBack={() => {}} />);
    const region = await screen.findByRole("region", { name: "Imaging frame" });
    expect(within(region).getByRole("status").textContent).toBe("Receiving 262144 of 2031984 bytes");
    expect((within(region).getByRole("button", { name: "Take imaging frame" }) as HTMLButtonElement).disabled).toBe(
      true,
    );
    expect(within(region).getByRole("button", { name: "Abort exposure" })).toBeTruthy();
  });

  it("names the saved file and how long after the exposure it was in hand", async () => {
    serve({
      ...READY,
      frame: frame({ file: "frames/2026-01-16/000001.fits", start: 1768528800.0, received_at: 1768528830.3 }),
    });
    render(<ImagingPage night={false} onBack={() => {}} />);
    const region = await screen.findByRole("region", { name: "Imaging frame" });
    expect(within(region).getByRole("status").textContent).toBe(
      "Saved frames/2026-01-16/000001.fits · in hand 28.3 s after the exposure",
    );
  });

  it("names the scope node's copy beside the saved file", async () => {
    serve({
      ...READY,
      frame: frame({
        file: "frames/2026-01-16/000001.fits",
        start: 1768528800.0,
        received_at: 1768528830.3,
        node_file: "frame-000001.fits",
      }),
    });
    render(<ImagingPage night={false} onBack={() => {}} />);
    const region = await screen.findByRole("region", { name: "Imaging frame" });
    expect(within(region).getByRole("status").textContent).toBe(
      "Saved frames/2026-01-16/000001.fits · in hand 28.3 s after the exposure · also at the scope node as frame-000001.fits",
    );
  });

  it("reports the scope node's failure to keep a copy", async () => {
    serve({
      ...READY,
      frame: frame({
        file: "frames/2026-01-16/000001.fits",
        start: 1768528800.0,
        received_at: 1768528830.3,
        node_save_error: "this scope node has no save_dir",
      }),
    });
    render(<ImagingPage night={false} onBack={() => {}} />);
    expect((await screen.findByRole("alert")).textContent).toBe(
      "The scope node kept no copy: this scope node has no save_dir.",
    );
  });

  it("reports a failed frame", async () => {
    serve({ ...READY, frame: frame({ state: "failed", error: "the imaging frame stalled" }) });
    render(<ImagingPage night={false} onBack={() => {}} />);
    expect((await screen.findByRole("alert")).textContent).toBe("Imaging frame failed: the imaging frame stalled.");
  });

  it("reports an aborted exposure", async () => {
    serve({ ...READY, frame: frame({ state: "aborted" }) });
    render(<ImagingPage night={false} onBack={() => {}} />);
    const region = await screen.findByRole("region", { name: "Imaging frame" });
    expect(within(region).getByRole("status").textContent).toBe("Exposure aborted.");
  });
});

const SAVED: ImagingView = {
  ...READY,
  frame: frame({
    file: "frames/2026-01-16/000001.fits",
    preview: "/api/imaging/frames/1/preview.jpg",
    start: 1768528800.0,
    received_at: 1768528830.3,
  }),
};

describe("ImagingPage's picture at night", () => {
  it("shows the preview as a photo, and colour only while the control is held", async () => {
    serve(SAVED);
    render(<ImagingPage night={true} onBack={() => {}} />);
    const picture = await screen.findByRole("img", { name: "Imaging frame 1" });
    expect(picture.getAttribute("src")).toBe("/api/imaging/frames/1/preview.jpg");
    expect(picture.classList.contains("photo")).toBe(true);
    const hold = screen.getByRole("button", { name: "Hold to see colour" });
    expect(hold.getAttribute("aria-pressed")).toBe("false");
    expect(
      screen.getByText("The picture is red at night. Hold the control to see its colours for up to 10 seconds."),
    ).toBeTruthy();
    fireEvent.pointerDown(hold);
    expect(picture.classList.contains("photo")).toBe(false);
    expect(hold.getAttribute("aria-pressed")).toBe("true");
    fireEvent.pointerUp(hold);
    expect(picture.classList.contains("photo")).toBe(true);
    expect(hold.getAttribute("aria-pressed")).toBe("false");
    for (const release of [fireEvent.pointerLeave, fireEvent.pointerCancel, fireEvent.blur]) {
      fireEvent.pointerDown(hold);
      expect(picture.classList.contains("photo")).toBe(false);
      release(hold);
      expect(picture.classList.contains("photo")).toBe(true);
      expect(hold.getAttribute("aria-pressed")).toBe("false");
    }
    fireEvent.keyDown(hold, { key: " " });
    expect(picture.classList.contains("photo")).toBe(false);
    fireEvent.keyUp(hold, { key: " " });
    expect(picture.classList.contains("photo")).toBe(true);
    expect(hold.getAttribute("aria-pressed")).toBe("false");
  });

  it("restores the colour 10 s after the hold began, and a repeated key does not restart the limit", async () => {
    vi.useFakeTimers();
    serve(SAVED);
    render(<ImagingPage night={true} onBack={() => {}} />);
    await act(async () => {});
    const picture = screen.getByRole("img", { name: "Imaging frame 1" });
    const hold = screen.getByRole("button", { name: "Hold to see colour" });
    fireEvent.pointerDown(hold);
    expect(picture.classList.contains("photo")).toBe(false);
    await act(() => vi.advanceTimersByTimeAsync(COLOUR_HOLD_MS));
    expect(picture.classList.contains("photo")).toBe(true);
    expect(hold.getAttribute("aria-pressed")).toBe("false");
    fireEvent.keyDown(hold, { key: " " });
    await act(() => vi.advanceTimersByTimeAsync(5000));
    fireEvent.keyDown(hold, { key: " ", repeat: true });
    expect(picture.classList.contains("photo")).toBe(false);
    await act(() => vi.advanceTimersByTimeAsync(5000));
    expect(picture.classList.contains("photo")).toBe(true);
    expect(hold.getAttribute("aria-pressed")).toBe("false");
  });

  it("offers no hold control by day", async () => {
    serve(SAVED);
    render(<ImagingPage night={false} onBack={() => {}} />);
    await screen.findByRole("img", { name: "Imaging frame 1" });
    expect(screen.queryByRole("button", { name: "Hold to see colour" })).toBeNull();
  });
});

describe("useColourHold", () => {
  it("holds only while the control is held, at night", () => {
    const { result } = renderHook(() => useColourHold(true));
    expect(result.current.held).toBe(false);
    act(() => result.current.bind.onPointerDown());
    expect(result.current.held).toBe(true);
    act(() => result.current.bind.onPointerUp());
    expect(result.current.held).toBe(false);
  });

  it("holds nothing by day", () => {
    const { result } = renderHook(() => useColourHold(false));
    act(() => result.current.bind.onPointerDown());
    expect(result.current.held).toBe(false);
  });

  it("ends a hold when night turns off", () => {
    const { result, rerender } = renderHook(({ night }: { night: boolean }) => useColourHold(night), {
      initialProps: { night: true },
    });
    act(() => result.current.bind.onPointerDown());
    expect(result.current.held).toBe(true);
    rerender({ night: false });
    expect(result.current.held).toBe(false);
  });
});

describe("ImagingPage's reads", () => {
  it("reads /api/imaging when it opens, every second and after each request, and stops when unmounted", async () => {
    vi.useFakeTimers();
    const fetch = serve(READY);
    render(<ImagingPage night={false} onBack={() => {}} />);
    const reads = () => fetch.mock.calls.filter(([url]) => String(url) === "/api/imaging").length;
    await act(async () => {});
    expect(reads()).toBe(1);
    await act(() => vi.advanceTimersByTimeAsync(1000));
    expect(reads()).toBe(2);
    await act(() => vi.advanceTimersByTimeAsync(1000));
    expect(reads()).toBe(3);
    fireEvent.click(screen.getByRole("button", { name: "Save imaging settings" }));
    await act(async () => {});
    expect(reads()).toBe(4);
    fireEvent.click(screen.getByRole("button", { name: "Take imaging frame" }));
    await act(async () => {});
    expect(reads()).toBe(5);
    cleanup();
    await act(() => vi.advanceTimersByTimeAsync(5000));
    expect(reads()).toBe(5);
  });

  it("changes nothing the page shows when a read fails", async () => {
    vi.useFakeTimers();
    let fail = false;
    vi.stubGlobal(
      "fetch",
      async (input: RequestInfo | URL) =>
        new Response(fail && String(input) === "/api/imaging" ? "{}" : JSON.stringify(READY), {
          status: fail && String(input) === "/api/imaging" ? 500 : 200,
        }),
    );
    render(<ImagingPage night={false} onBack={() => {}} />);
    await act(async () => {});
    expect(screen.getByRole("region", { name: "Camera" })).toBeTruthy();
    fail = true;
    await act(() => vi.advanceTimersByTimeAsync(1000));
    expect(screen.getByRole("region", { name: "Camera" })).toBeTruthy();
    expect(screen.getByRole("textbox", { name: "Exposure" })).toBeTruthy();
  });

  it("keeps a settings draft across the one-second reads", async () => {
    vi.useFakeTimers();
    serve(READY);
    render(<ImagingPage night={false} onBack={() => {}} />);
    await act(async () => {});
    fireEvent.change(screen.getByRole("textbox", { name: "Exposure" }), { target: { value: "2000" } });
    await act(() => vi.advanceTimersByTimeAsync(1000));
    await act(() => vi.advanceTimersByTimeAsync(1000));
    expect((screen.getByRole("textbox", { name: "Exposure" }) as HTMLInputElement).value).toBe("2000");
  });
});
