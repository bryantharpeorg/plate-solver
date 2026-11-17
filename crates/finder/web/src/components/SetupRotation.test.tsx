import { act, fireEvent, render, renderHook, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ObservingSession } from "../api";
import { useSession } from "../useSession";
import { SetupPage } from "./SetupPage";

// Oracles: the section's place in Setup (US1-S5), its own refusal and clearing
// words (US1-S6), and the session's wire shape (US1-S7).
const SESSION: ObservingSession = {
  site: { lat_deg: 40, lon_deg: -75 },
  min_alt_deg: 15,
  target: null,
  chart_rotation_deg: 79,
};

function open(session: ObservingSession = SESSION) {
  const put = vi.fn(async () => {});
  render(<SetupPage session={session} put={put} onBack={() => {}} />);
  return put;
}

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe("Setup's chart rotation", () => {
  it("sits between minimum altitude and the scope node and saves the typed angle", async () => {
    const put = open();
    const rotation = screen.getByRole("region", { name: "Chart rotation" });
    const altitude = screen.getByRole("region", { name: "Minimum altitude" });
    const scope = screen.getByRole("region", { name: "Scope node and camera" });
    expect(altitude.compareDocumentPosition(rotation) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(rotation.compareDocumentPosition(scope) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect((screen.getByRole("textbox", { name: "Chart rotation" }) as HTMLInputElement).value).toBe("79");
    expect(
      screen.getByText(
        "Turns the live chart and photos counter-clockwise, in degrees. Set it once to match how the camera sits in its holder.",
      ),
    ).toBeTruthy();
    fireEvent.change(screen.getByRole("textbox", { name: "Chart rotation" }), { target: { value: "85" } });
    fireEvent.click(screen.getByRole("button", { name: "Save chart rotation" }));
    await waitFor(() => expect(put).toHaveBeenCalledWith({ chart_rotation_deg: 85 }));
    expect(await screen.findByText("Chart rotation saved.")).toBeTruthy();
  });

  it("clears the setting from an empty field and refuses anything but a number", async () => {
    const put = open();
    fireEvent.change(screen.getByRole("textbox", { name: "Chart rotation" }), { target: { value: "" } });
    fireEvent.click(screen.getByRole("button", { name: "Save chart rotation" }));
    await waitFor(() => expect(put).toHaveBeenCalledWith({ chart_rotation_deg: null }));
    expect(await screen.findByText("Chart rotation cleared.")).toBeTruthy();
    fireEvent.change(screen.getByRole("textbox", { name: "Chart rotation" }), { target: { value: "abc" } });
    fireEvent.click(screen.getByRole("button", { name: "Save chart rotation" }));
    expect(put).toHaveBeenCalledTimes(1);
    expect(await screen.findByText("Chart rotation must be a number of degrees.")).toBeTruthy();
  });

  it("keeps chart_rotation_deg from every read of the session", async () => {
    vi.spyOn(document, "visibilityState", "get").mockReturnValue("visible");
    vi.stubGlobal("fetch", async () => new Response(JSON.stringify({ site: null, chart_rotation_deg: 90 })));
    const { result, unmount } = renderHook(() => useSession());
    await act(async () => {});
    expect(result.current.session.chart_rotation_deg).toBe(90);
    unmount();
    vi.stubGlobal("fetch", async () => new Response(JSON.stringify({ site: null })));
    const empty = renderHook(() => useSession());
    await act(async () => {});
    expect(empty.result.current.session.chart_rotation_deg).toBeUndefined();
    expect(Object.hasOwn(empty.result.current.session, "chart_rotation_deg")).toBe(false);
  });
});
