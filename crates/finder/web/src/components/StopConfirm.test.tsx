import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { SessionMount } from "../api";
import { StopButton } from "./StopButton";

// Oracle: 017 FR-014–FR-016's exact text, roles, request and session-read order.
// Replies and reads are supplied explicitly; assertions have no timing tolerance.
const unconfirmed =
  "The mount did not confirm Stop and may still be moving. Press Stop again. If it keeps moving, switch off the mount's power.";
const waiting = "Stop sent. Waiting for the mount to report that it has stopped.";
const moving: SessionMount = { connected: true, moving: true, board: null, firmware: null };
const disconnected: SessionMount = { ...moving, connected: false, moving: false };

afterEach(() => vi.unstubAllGlobals());

function stopButton() {
  const button = screen.getByRole("button", { name: "Stop" }) as HTMLButtonElement;
  expect(button.disabled).toBe(false);
  return button;
}

function reply(body: object) {
  const fetch = vi.fn(async () => new Response(JSON.stringify(body)));
  vi.stubGlobal("fetch", fetch);
  return fetch;
}

function pendingReply() {
  let resolve!: (response: Response) => void;
  const response = new Promise<Response>((done) => {
    resolve = done;
  });
  const fetch = vi.fn(() => response);
  vi.stubGlobal("fetch", fetch);
  return {
    fetch,
    finish: async (body: object) => {
      await act(async () => resolve(new Response(JSON.stringify(body))));
    },
  };
}

describe("Stop confirmation", () => {
  it("keeps the exact unconfirmed alert and enabled Stop when the mount reading is lost", async () => {
    const fetch = reply({ stopped: true, acknowledged: false });
    const onChange = vi.fn();
    const { rerender } = render(<StopButton mount={moving} onChange={onChange} />);
    fireEvent.click(stopButton());
    await waitFor(() => expect(onChange).toHaveBeenCalledOnce());

    rerender(<StopButton mount={disconnected} onChange={onChange} />);
    expect(screen.getByRole("alert").textContent).toBe(unconfirmed);
    stopButton();
    expect(screen.queryByText(waiting)).toBeNull();
    expect(fetch).toHaveBeenCalledExactlyOnceWith("/api/stop", { method: "POST" });
  });

  it("keeps a failed request's exact error and Stop when the mount reading is lost", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("The scope link is unavailable. Try again.")));
    const { rerender } = render(<StopButton mount={moving} />);
    fireEvent.click(stopButton());
    expect((await screen.findByRole("alert")).textContent).toBe("The scope link is unavailable. Try again.");

    rerender(<StopButton mount={disconnected} />);
    expect(screen.getByRole("alert").textContent).toBe("The scope link is unavailable. Try again.");
    stopButton();
    expect(screen.queryByText(waiting)).toBeNull();

    rerender(<StopButton mount={{ ...moving, moving: false }} />);
    expect(screen.queryByRole("alert")).toBeNull();
    expect(screen.queryByRole("button", { name: "Stop" })).toBeNull();
  });

  it.each([disconnected, undefined])(
    "shows a plain waiting note after a confirmed reply without a live reading: %s",
    async (mount) => {
      reply({ stopped: true });
      const onChange = vi.fn();
      const { rerender } = render(<StopButton mount={moving} onChange={onChange} />);
      fireEvent.click(stopButton());
      await waitFor(() => expect(onChange).toHaveBeenCalledOnce());

      rerender(<StopButton mount={mount} onChange={onChange} />);
      const note = screen.getByText(waiting);
      expect(note.tagName).toBe("P");
      expect(note.getAttribute("role")).toBeNull();
      expect(screen.queryByRole("alert")).toBeNull();
      expect(screen.queryByRole("status")).toBeNull();
      stopButton();

      rerender(<StopButton mount={{ ...moving, moving: false }} onChange={onChange} />);
      expect(screen.queryByText(waiting)).toBeNull();
      expect(screen.queryByRole("button", { name: "Stop" })).toBeNull();
    },
  );

  it("shows neither message for a confirmed reply while the live mount still reports motion", async () => {
    reply({ stopped: true });
    const onChange = vi.fn();
    const { rerender } = render(<StopButton mount={moving} onChange={onChange} />);
    fireEvent.click(stopButton());
    await waitFor(() => expect(onChange).toHaveBeenCalledOnce());
    rerender(<StopButton mount={{ ...moving }} onChange={onChange} />);

    expect(screen.queryByText(unconfirmed)).toBeNull();
    expect(screen.queryByText(waiting)).toBeNull();
    expect(screen.queryByRole("alert")).toBeNull();
    stopButton();
  });

  it("clears only for a new live reading at rest received after the reply", async () => {
    const request = pendingReply();
    const { rerender } = render(<StopButton mount={moving} />);
    fireEvent.click(stopButton());

    // This rest reading arrived during the request. The reply must capture this
    // latest object, rather than the moving object from the click's closure.
    const beforeReply: SessionMount = { ...moving, moving: false };
    rerender(<StopButton mount={beforeReply} />);
    await request.finish({ stopped: true, acknowledged: false });
    expect(screen.getByRole("alert").textContent).toBe(unconfirmed);
    stopButton();

    rerender(<StopButton mount={beforeReply} />);
    expect(screen.getByRole("alert").textContent).toBe(unconfirmed);
    stopButton();

    rerender(<StopButton mount={{ ...disconnected }} />);
    expect(screen.getByRole("alert").textContent).toBe(unconfirmed);
    stopButton();

    rerender(<StopButton mount={{ ...moving }} />);
    expect(screen.getByRole("alert").textContent).toBe(unconfirmed);
    stopButton();

    rerender(<StopButton mount={{ ...beforeReply }} />);
    expect(screen.queryByRole("alert")).toBeNull();
    expect(screen.queryByRole("button", { name: "Stop" })).toBeNull();
  });

  it("accepts a second press while the first request is pending", async () => {
    const request = pendingReply();
    const { rerender } = render(<StopButton mount={moving} />);
    fireEvent.click(stopButton());
    fireEvent.click(stopButton());
    expect(request.fetch).toHaveBeenCalledTimes(2);
    expect(request.fetch).toHaveBeenNthCalledWith(1, "/api/stop", { method: "POST" });
    expect(request.fetch).toHaveBeenNthCalledWith(2, "/api/stop", { method: "POST" });
    stopButton();

    await request.finish({ stopped: true, acknowledged: false });
    rerender(<StopButton mount={disconnected} />);
    expect(screen.getByRole("alert").textContent).toBe(unconfirmed);
    stopButton();
  });

  it("keeps Stop visible while retrying an unconfirmed Stop with no live reading", async () => {
    const request = pendingReply();
    request.fetch.mockResolvedValueOnce(new Response(JSON.stringify({ stopped: true, acknowledged: false })));
    const { rerender } = render(<StopButton mount={moving} />);
    fireEvent.click(stopButton());
    expect((await screen.findByRole("alert")).textContent).toBe(unconfirmed);
    rerender(<StopButton mount={disconnected} />);

    fireEvent.click(stopButton());
    expect(screen.queryByRole("alert")).toBeNull();
    stopButton();
    fireEvent.click(stopButton());
    expect(request.fetch).toHaveBeenCalledTimes(3);
    expect(request.fetch).toHaveBeenNthCalledWith(2, "/api/stop", { method: "POST" });
    expect(request.fetch).toHaveBeenNthCalledWith(3, "/api/stop", { method: "POST" });

    await request.finish({ stopped: true });
    expect(screen.getByText(waiting)).toBeTruthy();
    expect(screen.queryByRole("alert")).toBeNull();
    stopButton();
  });
});
