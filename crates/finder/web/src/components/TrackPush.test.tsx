import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ObservingSession, SessionTrackPush, Target } from "../api";
import recorded from "../test/fixtures/session-track";
import { TrackControl } from "./TrackControl";

// Oracle: the push and its values are spec 038's scenario session (Track following
// M42, push {"state":"following","ra_deg":84.0,"dec_deg":-3.4,"at":1768529000.0});
// the unpushed status and readout are the unedited session-track capture, and M43
// is OpenNGC's NGC 1982 row (crates/finder/data/openngc.csv). Nothing here comes
// from the component.
afterEach(() => vi.unstubAllGlobals());
const session = (name: keyof typeof recorded) => recorded[name] as ObservingSession;

const following: SessionTrackPush = { state: "following", ra_deg: 84.0, dec_deg: -3.4, at: 1768529000.0 };

const pushed = (push: SessionTrackPush): ObservingSession => {
  const withPush = session("tracking");
  return { ...withPush, track: { ...withPush.track!, push } };
};

// The server's canonical object for NGC 1982 (M43), from OpenNGC's row.
const m43: Target = {
  id: "NGC 1982",
  messier: "M43",
  common_names: ["Mairan's Nebula"],
  code: "HII",
  kind: "Emission nebula",
  family: "nebula",
  ra_deg: 83.88075,
  dec_deg: -5.26747,
  constellation: "Ori",
  major_arcmin: 20.0,
  minor_arcmin: 15.0,
  pa_deg: null,
  mag: 9.0,
};

describe("Recenter after a push by hand", () => {
  it("offers Recenter M42 while following a push, with the status and readout unchanged", () => {
    const { unmount } = render(<TrackControl session={pushed(following)} />);
    expect(screen.getByRole("button", { name: "Recenter M42" })).toBeTruthy();
    const withPush = screen.getByRole("region", { name: "Track" });
    const pushedStatus = within(withPush).getByRole("status").textContent;
    const pushedReadout = withPush.querySelector("dl")?.textContent;
    unmount();

    render(<TrackControl session={session("tracking")} />);
    const track = screen.getByRole("region", { name: "Track" });
    expect(within(track).getByRole("status").textContent).toBe(pushedStatus);
    expect(track.querySelector("dl")?.textContent).toBe(pushedReadout);
  });

  it("offers the same Recenter M42 while the push is still moving", () => {
    render(<TrackControl session={pushed({ state: "moving" })} />);
    expect(screen.getByRole("button", { name: "Recenter M42" })).toBeTruthy();
  });

  it("posts Track on once from Recenter after a 200 and refreshes the session", async () => {
    const fetch = vi.fn(async () => new Response(JSON.stringify(recorded.tracking)));
    vi.stubGlobal("fetch", fetch);
    const changed = vi.fn();
    render(<TrackControl session={pushed(following)} onChange={changed} />);
    fireEvent.click(screen.getByRole("button", { name: "Recenter M42" }));
    await waitFor(() => expect(changed).toHaveBeenCalledOnce());
    expect(fetch).toHaveBeenCalledTimes(1);
    expect(fetch).toHaveBeenCalledWith("/api/track", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ on: true }),
    });
    expect(screen.getByRole("switch", { name: "Track" }).getAttribute("aria-checked")).toBe("true");
  });

  it("shows a refused Recenter as an alert, still Track on, with one request", async () => {
    const fetch = vi.fn(async () => new Response(JSON.stringify({ error: "Sun is up" }), { status: 409 }));
    vi.stubGlobal("fetch", fetch);
    render(<TrackControl session={pushed(following)} />);
    fireEvent.click(screen.getByRole("button", { name: "Recenter M42" }));
    expect((await screen.findByRole("alert")).textContent).toContain("Sun is up");
    expect(fetch).toHaveBeenCalledTimes(1);
    expect(fetch).toHaveBeenCalledWith("/api/track", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ on: true }),
    });
    expect(screen.getByRole("switch", { name: "Track" }).getAttribute("aria-checked")).toBe("true");
  });

  it("offers no Recenter without a push: the switch is the only button", () => {
    render(<TrackControl session={session("tracking")} />);
    const track = screen.getByRole("region", { name: "Track" });
    expect(screen.queryAllByRole("button")).toHaveLength(0);
    expect(track.querySelectorAll("button")).toHaveLength(1);
    expect(track.querySelector("button")?.getAttribute("role")).toBe("switch");
  });

  it("offers no Recenter while the session's target is not Track's", () => {
    render(<TrackControl session={{ ...pushed(following), target: m43 }} />);
    expect(screen.queryByRole("button", { name: /Recenter/ })).toBeNull();
  });
});
