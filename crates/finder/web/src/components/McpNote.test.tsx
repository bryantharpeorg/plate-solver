import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { App } from "../App";
import type { ObservingSession } from "../api";
import ready from "../test/fixtures/session-ready.json";
import { mountMayMove } from "./StopButton";
import { McpNote } from "./McpNote";
import { MountSection } from "./MountSection";

// Oracle: FR-015's exact text and single-Stop contract, with recorded idle
// telemetry. App exercises useSession's field copy rather than bypassing it.
afterEach(() => {
  cleanup();
  window.history.replaceState(null, "", "/");
  vi.unstubAllGlobals();
});

it.each(["/", "/setup"])("keeps one enabled Stop on %s before moving telemetry, for either source", async (path) => {
  window.history.replaceState(null, "", path);
  vi.stubGlobal(
    "EventSource",
    class {
      addEventListener() {}
      removeEventListener() {}
      close() {}
    },
  );
  let session = { ...ready, goto: undefined, motion: undefined } as ObservingSession;
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
  const view = render(<App />);
  await screen.findByRole("button", { name: "Choose target" });
  await waitFor(() => expect(fetch).toHaveBeenCalledWith("/api/session"));
  expect(screen.queryByText(/Moved by an MCP client/)).toBeNull();
  expect(screen.queryByRole("button", { name: "Stop" })).toBeNull();

  for (const by of ["mcp", "page", undefined] as const) {
    session = { ...session, motion: by ? { by, tool: "goto_target", since: 123 } : undefined };
    // Remount to request the next independent session snapshot immediately.
    view.unmount();
    const next = render(<App />);
    if (by) {
      await screen.findByRole("button", { name: "Stop" });
      expect(screen.getAllByRole("button", { name: "Stop" })).toHaveLength(1);
      const top = screen.getByRole("link", { name: "Finder home" }).closest("header")!;
      const stop = within(top).getByRole("button", { name: "Stop" }) as HTMLButtonElement;
      expect(stop.disabled).toBe(false);
      if (by === "mcp") {
        expect(within(top).getByText("Moved by an MCP client · goto_target")).toBeTruthy();
        if (path === "/") {
          const guidance = screen.getByRole("region", { name: /^Target/ });
          expect(within(guidance).getByText("Moved by an MCP client · goto_target")).toBeTruthy();
          expect(within(guidance).queryByRole("button", { name: "Stop" })).toBeNull();
        }
      } else expect(screen.queryByText(/Moved by an MCP client/)).toBeNull();
      fireEvent.click(stop);
      await waitFor(() => expect(fetch).toHaveBeenCalledWith("/api/stop", { method: "POST" }));
    } else {
      await waitFor(() => expect(screen.queryByRole("button", { name: "Stop" })).toBeNull());
      expect(screen.queryByText(/Moved by an MCP client/)).toBeNull();
    }
    next.unmount();
  }
});

it("counts any motion before mount telemetry exists", () => {
  const motion = { by: "mcp", tool: "move_axis", since: 123 } as const;
  expect(mountMayMove(undefined, undefined, undefined, motion)).toBe(true);
  expect(mountMayMove(undefined)).toBe(false);
});

it.each(["move_axis", "set_tracking"])(
  "names %s as plain text and blocks model actions while motion is present",
  (tool) => {
    const motion = { by: "mcp", tool, since: 123 } as const;
    const { container } = render(<McpNote motion={motion} />);
    expect(container.textContent).toBe(`Moved by an MCP client · ${tool}`);
    expect(screen.queryByRole("status")).toBeNull();
    expect(screen.queryByRole("alert")).toBeNull();
    render(<MountSection session={{ ...ready, motion } as ObservingSession} />);
    for (const name of ["Second sync", "Reset night's model", "Reset fixed terms"]) {
      expect((screen.getByRole("button", { name }) as HTMLButtonElement).disabled).toBe(true);
    }
  },
);
