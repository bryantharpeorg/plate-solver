// The Nudge block (055 US5, FR-017 to FR-019). Oracles: the wire answers and the words the
// scenarios write out exactly, and the recorded session `tracking` (record-tracking.rs),
// which no page code produces. The Imaging page's ready view is the simulated imaging
// camera as the console relays it, as in ImagingPage.test.tsx.
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ImagingView, ObservingSession } from "../api";
import recorded from "../test/fixtures/session-track";
import { GuidanceBlock } from "./GuidanceBlock";
import { ImagingPage } from "./ImagingPage";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

const session = (name: keyof typeof recorded) => recorded[name] as ObservingSession;
const nudge = () => screen.getByRole("region", { name: "Nudge" });
const status = () => within(nudge()).getByRole("status").textContent;
const arrow = (way: string) => within(nudge()).getByRole("button", { name: `Nudge ${way}` }) as HTMLButtonElement;
const follow = () => within(nudge()).getByRole("combobox", { name: "Arrows follow" }) as HTMLSelectElement;
const options = () =>
  within(follow())
    .getAllByRole("option")
    .map((option) => option.textContent);
const NUDGE = "/api/mount/nudge";

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

describe("the Nudge block in the guidance block", () => {
  it("posts the chosen view and step, and reads the status through the move (US5-S1)", async () => {
    const fetch = vi.fn(async () => new Response(JSON.stringify({ up_arcsec: 0, right_arcsec: 240, moved: "axes" })));
    vi.stubGlobal("fetch", fetch);
    const changed = vi.fn();
    const tracking = session("tracking");
    const { rerender } = render(<GuidanceBlock session={tracking} onChange={changed} />);
    fireEvent.change(follow(), { target: { value: "finder_camera" } });
    fireEvent.click(within(nudge()).getByRole("button", { name: "Coarse 4′" }));
    expect(within(nudge()).getByRole("button", { name: "Coarse 4′" }).getAttribute("aria-pressed")).toBe("true");
    fireEvent.click(arrow("right"));
    await waitFor(() => expect(status()).toBe("Nudging right 4′…"));
    expect(changed).toHaveBeenCalledOnce();
    expect(fetch).toHaveBeenCalledWith(NUDGE, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ direction: "right", step: "coarse", view: "finder_camera" }),
    });
    rerender(
      <GuidanceBlock session={{ ...tracking, axis_move: { state: "moving", tool: "nudge" } }} onChange={changed} />,
    );
    expect(status()).toBe("Nudging right 4′…");
    rerender(
      <GuidanceBlock session={{ ...tracking, axis_move: { state: "done", tool: "nudge" } }} onChange={changed} />,
    );
    expect(status()).toBe("Nudged right 4′");
  });

  it("holds every arrow until the answer, then shows the refusal in the block's alert (US5-S2)", async () => {
    let release!: () => void;
    const held = new Promise<void>((resolve) => {
      release = resolve;
    });
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => {
        await held;
        return new Response(JSON.stringify({ error: "the night has no sync yet" }), { status: 409 });
      }),
    );
    render(<GuidanceBlock session={session("tracking")} />);
    fireEvent.click(arrow("up"));
    for (const way of ["up", "right", "down", "left"]) expect(arrow(way).disabled).toBe(true);
    release();
    await waitFor(() => expect(within(nudge()).getByRole("alert").textContent).toBe("the night has no sync yet"));
    expect(arrow("up").disabled).toBe(false);
    for (const way of ["up", "right", "down", "left"]) expect(arrow(way).disabled).toBe(false);
  });

  it("offers the views the session has, after Track, with the arrows as a cross (US5-S3)", () => {
    const tracking = session("tracking");
    const { imaging_camera: _camera, ...withoutCamera } = tracking;
    const { rerender } = render(<GuidanceBlock session={withoutCamera} />);
    expect(options()).toEqual(["the mount", "the finder camera"]);
    rerender(<GuidanceBlock session={tracking} />);
    expect(options()).toEqual(["the mount", "the finder camera", "the imaging camera"]);
    expect(follow().value).toBe("mount");
    const track = screen.getByRole("region", { name: "Track" });
    expect(track.compareDocumentPosition(nudge()) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(["up", "right", "down", "left"].map((way) => arrow(way).textContent)).toEqual(["↑", "→", "↓", "←"]);
    const { mount: _mount, ...withoutMount } = tracking;
    rerender(<GuidanceBlock session={withoutMount} />);
    expect(screen.queryByRole("region", { name: "Nudge" })).toBeNull();
  });
});

describe("the Nudge block on the Imaging page", () => {
  it("follows the frame with the imaging camera chosen, ready or not (US5-S4)", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response(JSON.stringify(READY))),
    );
    const { target: _target, ...untargeted } = session("tracking");
    const { unmount } = render(<ImagingPage night={false} onBack={() => {}} session={untargeted} />);
    const frame = await screen.findByRole("region", { name: "Imaging frame" });
    expect(frame.compareDocumentPosition(nudge()) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(follow().value).toBe("imaging_camera");
    unmount();
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response(JSON.stringify({ state: "off" }))),
    );
    render(<ImagingPage night={false} onBack={() => {}} session={untargeted} />);
    expect(await screen.findByText("The scope node is not connected.")).toBeTruthy();
    expect(screen.getByRole("region", { name: "Nudge" })).toBeTruthy();
  });
});
