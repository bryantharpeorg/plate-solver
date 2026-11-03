import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { App } from "../App";
import { McpSection } from "./McpSection";

// Oracle: GET /api/mcp's documented wire examples. URL order is exact;
// these read-only views add neither requests nor live-message roles.
const lan = {
  enabled: true,
  local_only: false,
  urls: ["http://localhost:8080/mcp", "http://192.168.1.20:8080/mcp", "http://172.24.1.2:8080/mcp"],
  allowed_hosts: ["surface.local"],
};
afterEach(() => {
  cleanup();
  window.history.replaceState(null, "", "/");
  vi.unstubAllGlobals();
});

it.each([
  { name: "LAN", status: lan },
  { name: "loopback", status: { ...lan, local_only: true, urls: [lan.urls[0]!], allowed_hosts: [] } },
  { name: "off", status: { ...lan, enabled: false, urls: [] } },
])("shows the $name answer without fetching or live roles", ({ name, status }) => {
  const fetch = vi.fn();
  vi.stubGlobal("fetch", fetch);
  render(<McpSection status={status} />);
  const section = screen.getByRole("region", { name: "MCP" });
  expect(within(section).queryByRole("status")).toBeNull();
  expect(within(section).queryByRole("alert")).toBeNull();
  expect(fetch).not.toHaveBeenCalled();
  if (name === "off") expect(within(section).getByText("MCP is off.")).toBeTruthy();
  else {
    expect(within(section).getByRole("list", { name: "MCP URLs" }).textContent).toBe(status.urls.join(""));
    expect(
      within(section)
        .getAllByRole("listitem")
        .slice(0, status.urls.length)
        .map((li) => li.textContent),
    ).toEqual(status.urls);
    if (name === "LAN") {
      expect(within(section).getByText("surface.local")).toBeTruthy();
      expect(section.textContent).toMatch(/WSL.*NAT.*phone.*README.*portproxy or mirrored.mode/i);
    } else {
      expect(section.textContent).toMatch(/only from this machine/i);
      expect(section.textContent).toMatch(/other devices.*can't reach.*listens on the LAN/i);
    }
  }
});

it.each([{ status: "ok" }, { enabled: "yes", urls: [] }, { enabled: true }, null])(
  "renders nothing for an invalid or health-shaped answer: %j",
  (status) => {
    const { container } = render(<McpSection status={status} />);
    expect(container.innerHTML).toBe("");
  },
);

it.each(["ok", "rejected", "non-OK", "not JSON"])(
  "App reads MCP only on entering Setup and catches %s reads",
  async (answer) => {
    let reads = 0;
    const fetch = vi.fn(async (input: RequestInfo | URL) => {
      if (String(input) === "/api/mcp") {
        reads++;
        if (answer === "rejected") throw new Error("offline");
        if (answer === "non-OK") return new Response("unavailable", { status: 503 });
        if (answer === "not JSON") return new Response("unavailable");
        return new Response(JSON.stringify(lan));
      }
      return new Response(
        JSON.stringify(
          String(input) === "/api/samples"
            ? []
            : String(input) === "/api/health"
              ? { version: "test", database: { catalog: "fixture", stars: 1 } }
              : { site: null, target: null },
        ),
      );
    });
    vi.stubGlobal("fetch", fetch);
    render(<App />);
    expect(reads).toBe(0);
    for (const count of [1, 2]) {
      fireEvent.click(screen.getByRole("link", { name: "Setup" }));
      await waitFor(() => expect(reads).toBe(count));
      if (answer === "ok") {
        const section = await screen.findByRole("region", { name: "MCP" });
        expect(section.parentElement!.lastElementChild).toBe(section);
      } else expect(screen.queryByRole("region", { name: "MCP" })).toBeNull();
      fireEvent.click(screen.getByRole("link", { name: "Back to console" }));
    }
  },
);
