// US5-S5–S6 and FR-017: the App's own navigation, with the page rig's API stubs.
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { App } from "../App";
import { ORION2, SIMILAR_M42 } from "../test/fixtures";

afterEach(() => {
  cleanup();
  window.history.replaceState(null, "", "/");
  vi.unstubAllGlobals();
});

function serve(nodePaired: boolean) {
  vi.stubGlobal(
    "EventSource",
    class {
      addEventListener() {}
      removeEventListener() {}
      close() {}
    },
  );
  vi.stubGlobal("fetch", async (input: RequestInfo | URL) => {
    const url = String(input);
    const body =
      url === "/api/session"
        ? {
            site: { lat_deg: 40, lon_deg: -75 },
            ...(nodePaired
              ? {
                  node: {
                    address: "scope.local:1",
                    token_set: true,
                    connected: true,
                    last_frame_age_s: 0,
                    offset_s: null,
                    camera: null,
                    stars: 30,
                    out_of_range: [],
                  },
                }
              : {}),
          }
        : url === "/api/imaging"
          ? { state: "ready" }
          : url === "/api/samples"
            ? [{ name: "orion2.jpg", label: "Orion2" }]
            : url === "/api/solve"
              ? ORION2
              : url.startsWith("/api/similar")
                ? SIMILAR_M42
                : { status: "ok", version: "test", database: { catalog: "hip", stars: 1 }, samples: 1 };
    return new Response(JSON.stringify(body));
  });
  window.history.pushState(null, "", "/setup");
}

it("US5-S5: opens Tools from Setup with a paired scope node and returns to the console", async () => {
  serve(true);
  render(<App />);
  const tools = await screen.findByRole("link", { name: "Tools" });
  expect(tools.getAttribute("href")).toBe("/tools");
  fireEvent.click(tools);
  expect(window.location.pathname).toBe("/tools");
  expect(screen.getByRole("heading", { name: "Tools", level: 1 })).toBeTruthy();
  expect(await screen.findByRole("button", { name: "Start aligning" })).toBeTruthy();
  fireEvent.click(screen.getByRole("link", { name: "Back to console" }));
  expect(window.location.pathname).toBe("/");
});

it("US5-S6: Setup has no Tools link without a paired scope node", async () => {
  serve(false);
  render(<App />);
  await screen.findByRole("heading", { name: "Setup", level: 1 });
  await waitFor(() => expect((screen.getByRole("textbox", { name: "Latitude" }) as HTMLInputElement).value).toBe("40"));
  expect(screen.queryByRole("link", { name: "Tools" })).toBeNull();
});

it("FR-017: picking a frame on Tools opens the console", async () => {
  serve(true);
  render(<App />);
  fireEvent.click(await screen.findByRole("link", { name: "Tools" }));
  fireEvent.click(screen.getByRole("button", { name: "Frames" }));
  const picker = screen.getByRole("dialog", { name: "Choose a frame" });
  fireEvent.click(await within(picker).findByRole("button", { name: "Orion2" }));
  expect(window.location.pathname).toBe("/");
  expect(await screen.findByRole("heading", { name: "Orion" })).toBeTruthy();
});
