import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "../App";
import type { ObservingSession } from "../api";
import second from "../test/fixtures/session-second-sync-slewing.json";
import { StopButton } from "./StopButton";

// Oracle: simulator second-sync capture and FR-024's exact DOM/accessibility
// contract. jsdom has no layout: 44px is explicit on the control; native modal
// calls are checked separately because jsdom does not implement modal inertness.
beforeEach(() => window.history.replaceState(null, "", "/"));
afterEach(() => {
  window.history.replaceState(null, "", "/");
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});
function serve(session: ObservingSession = second as ObservingSession) {
  vi.stubGlobal(
    "EventSource",
    class {
      addEventListener() {}
      removeEventListener() {}
      close() {}
    },
  );
  const fetch = vi.fn(
    async (input: RequestInfo | URL) =>
      new Response(
        JSON.stringify(
          String(input) === "/api/session"
            ? session
            : String(input) === "/api/samples"
              ? []
              : String(input) === "/api/health"
                ? { version: "test", database: { catalog: "fixture", stars: 1 } }
                : { stopped: true },
        ),
      ),
  );
  vi.stubGlobal("fetch", fetch);
  return fetch;
}
function assertAccessible() {
  const top = screen.getByRole("link", { name: "Finder home" }).closest("header")!;
  const stop = within(top).getByRole("button", { name: "Stop" }) as HTMLButtonElement;
  expect(stop.disabled).toBe(false);
  expect(stop.closest('dialog, [inert], [aria-hidden="true"]')).toBeNull();
  expect(stop.style.minHeight).toBe("44px");
  return stop;
}

describe("Stop", () => {
  it("sits in Setup's top bar during the second sync and POSTs without a token or body", async () => {
    const fetch = serve();
    window.history.replaceState(null, "", "/setup");
    render(<App />);
    await screen.findByRole("button", { name: "Stop" });
    expect(screen.getByRole("heading", { name: "Setup" })).toBeTruthy();
    fireEvent.click(assertAccessible());
    await waitFor(() => expect(fetch).toHaveBeenCalledWith("/api/stop", { method: "POST" }));
  });

  it.each(["/", "/setup"])("remains accessible on %s with the target sheet open and closed", async (path) => {
    serve();
    window.history.replaceState(null, "", path);
    const modal = vi.spyOn(HTMLDialogElement.prototype, "showModal");
    render(<App />);
    await screen.findByRole("button", { name: "Stop" });
    assertAccessible();
    fireEvent.click(screen.getByRole("button", { name: "Choose target" }));
    expect(screen.getByRole("dialog", { name: "Choose a target" })).toBeTruthy();
    assertAccessible();
    expect(modal).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Close" }));
    expect(screen.queryByRole("dialog", { name: "Choose a target" })).toBeNull();
    assertAccessible();
  });

  it("also keeps the frame sheet from making Stop inert", async () => {
    serve();
    const modal = vi.spyOn(HTMLDialogElement.prototype, "showModal");
    render(<App />);
    await screen.findByRole("button", { name: "Stop" });
    fireEvent.click(screen.getByRole("button", { name: "Frames" }));
    expect(screen.getByRole("dialog")).toBeTruthy();
    assertAccessible();
    expect(modal).not.toHaveBeenCalled();
  });

  it("stays enabled through pending, failed and repeated Stop requests", async () => {
    let reject!: (error: Error) => void;
    const fetch = vi.fn(
      () =>
        new Promise<Response>((_resolve, fail) => {
          reject = fail;
        }),
    );
    vi.stubGlobal("fetch", fetch);
    render(<StopButton mount={{ connected: false, board: null, firmware: null, moving: true }} />);
    const stop = screen.getByRole("button", { name: "Stop" }) as HTMLButtonElement;
    fireEvent.click(stop);
    expect(stop.disabled).toBe(false);
    reject(new Error("Stop did not reach the mount. Try again."));
    expect((await screen.findByRole("alert")).textContent).toContain("Try again");
    fireEvent.click(stop);
    expect(fetch).toHaveBeenCalledTimes(2);
    expect(stop.disabled).toBe(false);
  });

  it.each(["slewing", "solving", "correcting"] as const)("shows for %s before telemetry reports motion", (state) => {
    render(
      <StopButton
        mount={{ connected: true, moving: false, board: null, firmware: null }}
        goto={{ state, correction: 1, error_arcsec: 70, reason: null }}
      />,
    );
    expect((screen.getByRole("button", { name: "Stop" }) as HTMLButtonElement).disabled).toBe(false);
  });

  it("renders nothing without a mount or for an idle mount", () => {
    const { rerender } = render(<StopButton />);
    expect(screen.queryByRole("button", { name: "Stop" })).toBeNull();
    rerender(<StopButton mount={{ connected: true, moving: false, board: null, firmware: null }} />);
    expect(screen.queryByRole("button", { name: "Stop" })).toBeNull();
  });
});
