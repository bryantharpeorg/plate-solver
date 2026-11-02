import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App } from "../App";
import type { ObservingSession } from "../api";
import recorded from "../test/fixtures/session-track";

// Oracle: recorded Track states, with telemetry held at rest to exercise the
// interval before autonomous motion. Exact visibility and bodyless Stop request.
afterEach(() => {
  // Unmount while EventSource is still stubbed: cleanup may flush pending effects.
  cleanup();
  window.history.replaceState(null, "", "/");
  vi.unstubAllGlobals();
});
function serve(session: ObservingSession) {
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
              : { version: "test", database: { catalog: "fixture", stars: 1 } },
        ),
      ),
  );
  vi.stubGlobal("fetch", fetch);
  return fetch;
}
const cases = ["armed", "tracking", "keyhole_wait"] as const;
describe("Stop while Track may move the mount", () => {
  it.each(cases.flatMap((state) => ["/", "/setup"].map((path) => ({ state, path }))))(
    "shows in the top bar on $path while $state and POSTs Stop",
    async ({ state, path }) => {
      const source = recorded[state] as ObservingSession;
      const fetch = serve({ ...source, goto: undefined, mount: { ...source.mount!, moving: false } });
      window.history.replaceState(null, "", path);
      render(<App />);
      await screen.findByRole("button", { name: "Stop" });
      if (path === "/setup") expect(screen.getByRole("heading", { name: "Setup" })).toBeTruthy();
      const top = screen.getByRole("link", { name: "Finder home" }).closest("header")!;
      const stop = within(top).getByRole("button", { name: "Stop" }) as HTMLButtonElement;
      expect(stop.disabled).toBe(false);
      expect(stop.closest('dialog, [inert], [aria-hidden="true"]')).toBeNull();
      fireEvent.click(stop);
      await waitFor(() => expect(fetch).toHaveBeenCalledWith("/api/stop", { method: "POST" }));
    },
  );

  it.each(["/", "/setup"].flatMap((path) => [false, true].map((ended) => ({ path, ended }))))(
    "hides with idle mount on $path, ended=$ended",
    async ({ path, ended }) => {
      const source = recorded.too_low as ObservingSession;
      serve({
        ...source,
        goto: undefined,
        track: ended ? source.track : undefined,
        mount: { ...source.mount!, moving: false },
      });
      window.history.replaceState(null, "", path);
      render(<App />);
      // Positive control: session has loaded, including its mount/model telemetry.
      if (path === "/setup") await screen.findByText(/scope.local:1/);
      else await screen.findByRole("button", { name: "GoTo" });
      expect(screen.queryByRole("button", { name: "Stop" })).toBeNull();
    },
  );
});
