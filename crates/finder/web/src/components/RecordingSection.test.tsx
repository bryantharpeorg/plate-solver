import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ObservingSession, SessionRecording } from "../api";
import { useSession } from "../useSession";
import { RecordingSection } from "./RecordingSection";
import { SetupPage } from "./SetupPage";

// Oracle: US3-S1–S6's session wire values, request bodies and display copy, exact (no tolerance).
const SESSION: ObservingSession = {
  site: null,
  node: {
    address: "scope:50051",
    token_set: true,
    connected: true,
    last_frame_age_s: 0,
    offset_s: 1000,
    camera: null,
    stars: 30,
    out_of_range: [],
  },
};
const RECORDING: SessionRecording = {
  on: true,
  folder: "2026-01-16T02-00-00Z",
  since: 1768528800,
  frames: 12,
  solves: 9,
  previews: 2,
};
const RUNNING = "Recording to 2026-01-16T02-00-00Z · 12 frames · 9 solves";
const SAVED = "Saved 2026-01-16T02-00-00Z: 12 frames, 9 solves.";
const onChange = vi.fn();
const region = () => screen.getByRole("region", { name: "Recording" });
const section = (recording = RECORDING) =>
  render(<RecordingSection session={{ ...SESSION, recording }} onChange={onChange} />);
const setup = (session = SESSION) =>
  render(<SetupPage session={session} put={vi.fn()} onBack={() => {}} onChange={onChange} />);
const answer = (session: ObservingSession) => {
  const fetch = vi.fn(async () => new Response(JSON.stringify(session), { status: 200 }));
  vi.stubGlobal("fetch", fetch);
  return fetch;
};

beforeEach(() => {
  onChange.mockReset();
  vi.stubGlobal(
    "EventSource",
    class {
      addEventListener() {}
      removeEventListener() {}
      close() {}
    },
  );
});
afterEach(() => vi.unstubAllGlobals());

describe("Setup recording (025 US3)", () => {
  it("places one quiet Recording region immediately after Aim point and starts a recording", async () => {
    const fetch = answer({ ...SESSION, recording: RECORDING });
    setup();
    expect(screen.getAllByRole("region", { name: "Recording" })).toHaveLength(1);
    const aim = screen.getByRole("region", { name: "Aim point" });
    expect(aim.compareDocumentPosition(region()) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(aim.nextElementSibling).toBe(region());
    const recording = within(region());
    expect(recording.queryByRole("status")).toBeNull();
    expect(recording.queryByRole("alert")).toBeNull();
    expect(
      recording.getByText(
        "Saves each star frame's stars, every solve and the previews to the console's recordings folder, to test with later.",
      ),
    ).toBeTruthy();
    expect(onChange).not.toHaveBeenCalled();
    fireEvent.click(recording.getByRole("button", { name: "Start recording" }));
    await waitFor(() => expect(onChange).toHaveBeenCalledTimes(1));
    expect(fetch).toHaveBeenCalledExactlyOnceWith("/api/record", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: '{"on":true}',
    });
    expect(fetch.mock.invocationCallOrder[0]).toBeLessThan(onChange.mock.invocationCallOrder[0]!);
  });

  it("shows the exact start refusal without calling onChange", async () => {
    const fetch = vi.fn(
      async () => new Response(JSON.stringify({ error: "this console has no recordings folder" }), { status: 409 }),
    );
    vi.stubGlobal("fetch", fetch);
    setup();
    fireEvent.click(within(region()).getByRole("button", { name: "Start recording" }));
    expect((await within(region()).findByRole("alert")).textContent).toBe("this console has no recordings folder");
    expect(onChange).not.toHaveBeenCalled();
  });

  it("omits Recording without a scope node", () => {
    setup({ site: null });
    expect(screen.queryByRole("region", { name: "Recording" })).toBeNull();
  });

  it("names the running folder and counts, then posts stop and three raw frames before refreshing", async () => {
    const fetch = answer({ ...SESSION, recording: RECORDING });
    section();
    expect(within(region()).getByRole("status").textContent).toBe(RUNNING);
    expect(within(region()).queryByRole("alert")).toBeNull();
    fireEvent.click(within(region()).getByRole("button", { name: "Stop recording" }));
    await waitFor(() => expect(onChange).toHaveBeenCalledTimes(1));
    expect(fetch).toHaveBeenNthCalledWith(1, "/api/record", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: '{"on":false}',
    });
    expect(fetch.mock.invocationCallOrder[0]).toBeLessThan(onChange.mock.invocationCallOrder[0]!);
    fireEvent.click(within(region()).getByRole("button", { name: "Save 3 raw frames" }));
    await waitFor(() => expect(onChange).toHaveBeenCalledTimes(2));
    expect(fetch).toHaveBeenCalledTimes(2);
    expect(fetch).toHaveBeenNthCalledWith(2, "/api/record/raw", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: '{"frames":3}',
    });
    expect(fetch.mock.invocationCallOrder[1]).toBeLessThan(onChange.mock.invocationCallOrder[1]!);
  });

  it("keeps Stop recording available while the scope node is disconnected", () => {
    render(
      <RecordingSection session={{ ...SESSION, node: { ...SESSION.node!, connected: false }, recording: RECORDING }} />,
    );
    expect((within(region()).getByRole("button", { name: "Stop recording" }) as HTMLButtonElement).disabled).toBe(
      false,
    );
  });

  it("keeps the running line and raw progress in one status, disabling another raw request", () => {
    section({ ...RECORDING, raw: { wanted: 3, saved: 1 } });
    const statuses = within(region()).getAllByRole("status");
    expect(statuses).toHaveLength(1);
    expect(statuses[0]!.textContent).toContain(RUNNING);
    expect(statuses[0]!.textContent).toContain("Raw frames: 1 of 3 saved");
    expect((within(region()).getByRole("button", { name: "Save 3 raw frames" }) as HTMLButtonElement).disabled).toBe(
      true,
    );
    expect(within(region()).queryByRole("alert")).toBeNull();
  });

  it.each([
    [0, "this scope node cannot send raw frames", "Raw frames stopped: this scope node cannot send raw frames."],
    [1, "the mount started moving", "Raw frames stopped: the mount started moving."],
  ])("shows a raw error unchanged and enables another request (%s saved)", (saved, error, message) => {
    section({ ...RECORDING, raw: { wanted: 3, saved, error } });
    expect(within(region()).getByRole("status").textContent).toBe(RUNNING);
    expect(within(region()).getByRole("alert").textContent).toBe(message);
    expect((within(region()).getByRole("button", { name: "Save 3 raw frames" }) as HTMLButtonElement).disabled).toBe(
      false,
    );
  });

  it("shows completed raw progress and enables the next request", () => {
    section({ ...RECORDING, raw: { wanted: 3, saved: 3 } });
    expect(within(region()).getByRole("status").textContent).toContain("Raw frames: 3 of 3 saved");
    expect((within(region()).getByRole("button", { name: "Save 3 raw frames" }) as HTMLButtonElement).disabled).toBe(
      false,
    );
  });

  it.each([
    ["stopped", SAVED],
    ["limit", `${SAVED} It stopped at its frame limit.`],
    ["node_restarted", `${SAVED} It stopped when the scope node restarted. Start a new recording.`],
  ] as const)("reports what was saved after %s, offering a new recording without an alert", (ended_reason, message) => {
    section({ ...RECORDING, on: false, ended_reason });
    expect(within(region()).getByRole("status").textContent).toBe(message);
    expect(within(region()).getByRole("button", { name: "Start recording" })).toBeTruthy();
    expect(within(region()).queryByRole("alert")).toBeNull();
    expect(within(region()).queryByRole("button", { name: "Stop recording" })).toBeNull();
    expect(within(region()).queryByRole("button", { name: "Save 3 raw frames" })).toBeNull();
  });

  it("reports a failed write alongside what was saved", () => {
    section({ ...RECORDING, on: false, ended_reason: "write_failed" });
    expect(within(region()).getByRole("status").textContent).toBe(SAVED);
    expect(within(region()).getByRole("alert").textContent).toBe(
      "Recording stopped: the console could not write to its folder.",
    );
    expect(within(region()).getByRole("button", { name: "Start recording" })).toBeTruthy();
  });

  it("reads recording from GET /api/session through useSession on the real Setup path", async () => {
    const fetch = answer({ ...SESSION, recording: RECORDING });
    function OtherDevice() {
      const { session, put, refresh } = useSession();
      return <SetupPage session={session} put={put} onBack={() => {}} onChange={refresh} />;
    }
    render(<OtherDevice />);
    const recording = await screen.findByRole("region", { name: "Recording" });
    expect(within(recording).getByRole("status").textContent).toBe(RUNNING);
    expect(fetch).toHaveBeenCalledWith("/api/session");
  });
});
