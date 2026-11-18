// The aim point's source on Setup (032 US6-S7). Oracle: the stated words,
// compared exactly, with the coordinates line and the section's roles unchanged.
import { render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ObservingSession } from "../api";
import { useSession } from "../useSession";
import { AimTap } from "./AimTap";
import { SetupPage } from "./SetupPage";

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

const MEASURED: ObservingSession = {
  site: null,
  aim: { x: 700, y: 420, source: "measured" },
  imaging: { state: "ready", relation: { angle_deg: 17.04 } },
};

describe("the aim point's source", () => {
  it("says measured with the imaging angle, keeping the coordinates and no status or alert", () => {
    render(<AimTap session={MEASURED} live={null} />);
    const region = screen.getByRole("region", { name: "Aim point" });
    expect(within(region).getByText("Measured from the imaging camera. Imaging angle 17.0°.")).toBeTruthy();
    expect(within(region).getByText("Aim point: (700, 420) px")).toBeTruthy();
    expect(within(region).queryByRole("status")).toBeNull();
    expect(within(region).queryByRole("alert")).toBeNull();
  });

  it("says measured without the imaging angle when the console has none", () => {
    render(<AimTap session={{ site: null, aim: { x: 700, y: 420, source: "measured" } }} live={null} />);
    expect(screen.getByText("Measured from the imaging camera.")).toBeTruthy();
  });

  it("says tapped for an aim point without a source", () => {
    render(<AimTap session={{ site: null, aim: { x: 700, y: 420 } }} live={null} />);
    expect(screen.getByText("Set by tapping a star.")).toBeTruthy();
  });

  it("says nothing with no aim point", () => {
    render(<AimTap session={{ site: null }} live={null} />);
    expect(screen.queryByText(/^Measured from the imaging camera\./)).toBeNull();
    expect(screen.queryByText("Set by tapping a star.")).toBeNull();
  });
});

describe("Setup fed by useSession", () => {
  it("keeps the aim's source and the imaging relation", async () => {
    const session = {
      site: null,
      node: {
        address: "scope:50051",
        token_set: true,
        connected: true,
        last_frame_age_s: 0,
        offset_s: null,
        camera: null,
        stars: 30,
        out_of_range: [],
      },
      aim: { x: 700, y: 420, source: "measured" },
      imaging: { state: "ready", relation: { angle_deg: 17.04 } },
    };
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response(JSON.stringify(session))),
    );
    vi.stubGlobal(
      "EventSource",
      class {
        addEventListener() {}
        removeEventListener() {}
        close() {}
      },
    );
    function Page() {
      const { session: read, put } = useSession();
      return <SetupPage session={read} put={put} onBack={() => {}} />;
    }
    render(<Page />);
    await waitFor(() =>
      expect(screen.getByText("Measured from the imaging camera. Imaging angle 17.0°.")).toBeTruthy(),
    );
  });
});
