import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { M8, M75, M78, SESSION, TIME } from "../test/targets";
import type { ObservingSession } from "../useSession";
import { GuidanceBlock } from "./GuidanceBlock";

// US1-S1–S6: exact display oracles from the spec and the explicit SESSION fixture.
// Tenths round magnitudes half up, including 0.35 and readings that round to zero.
afterEach(cleanup);
const block = () => screen.getByRole("region", { name: /Target/ });

describe("the observer's push directions", () => {
  it("leads with right and up to a tenth, in the Push style and ink", () => {
    render(<GuidanceBlock session={SESSION} />);
    const push = block().querySelector("[data-push]");
    expect(push?.textContent).toBe("right 33.6° · up 12.4°");
    expect(push?.classList.contains("text-2xl")).toBe(true);
    expect(push?.classList.contains("leading-tight")).toBe(true);
    expect(push?.classList.contains("font-semibold")).toBe(true);
    expect(block().classList.contains("text-ink")).toBe(true);
  });

  it("prints negative magnitudes as left and down", () => {
    render(
      <GuidanceBlock session={{ ...SESSION, guidance: { class: "pointable", turn_deg: -2.74, tilt_deg: -1.37 } }} />,
    );
    expect(block().querySelector("[data-push]")?.textContent).toBe("left 2.7° · down 1.4°");
  });

  it("keeps right and up when either signed reading rounds to zero", () => {
    render(
      <GuidanceBlock session={{ ...SESSION, guidance: { class: "pointable", turn_deg: -0.04, tilt_deg: 0.04 } }} />,
    );
    expect(block().querySelector("[data-push]")?.textContent).toBe("right 0.0° · up 0.0°");
  });

  it("prints a quiet on-sky distance in tenths only when supplied", () => {
    const { rerender } = render(
      <GuidanceBlock session={{ ...SESSION, guidance: { ...SESSION.guidance!, separation_deg: 2.83 } }} />,
    );
    expect(block().querySelector("[data-away]")?.textContent).toBe("2.8° away");
    expect(block().querySelector("[data-away]")?.classList.contains("quiet")).toBe(true);
    rerender(<GuidanceBlock session={{ ...SESSION, guidance: { ...SESSION.guidance!, separation_deg: 0.35 } }} />);
    expect(block().querySelector("[data-away]")?.textContent).toBe("0.4° away");
    rerender(<GuidanceBlock session={SESSION} />);
    expect(block().querySelector("[data-away]")).toBeNull();
  });

  it("keeps the old words quiet, after push and away and before pointing age", () => {
    const { rerender } = render(<GuidanceBlock session={SESSION} />);
    const old = screen.getByText("turn clockwise 34° · tilt up 12°");
    expect(old.classList.contains("quiet")).toBe(true);
    const push = block().querySelector("[data-push]")!;
    expect(push).not.toBeNull();
    expect(push.compareDocumentPosition(old) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();

    rerender(<GuidanceBlock session={{ ...SESSION, guidance: { ...SESSION.guidance!, separation_deg: 2.83 } }} />);
    const lines = [...block().querySelectorAll("[data-push], [data-away], p.quiet")].map((p) => p.textContent);
    expect(lines).toEqual([
      "right 33.6° · up 12.4°",
      "2.8° away",
      "turn clockwise 34° · tilt up 12°",
      "Pointing 2 min ago.",
    ]);
  });

  it("suppresses both new lines without directions or while stale, and restores both with directions", () => {
    // Rebuild the five no-directions sessions from GuidanceBlock.test.tsx inline.
    const withoutDirections: ObservingSession[] = [
      { ...SESSION, pointing: null, guidance: { class: "pointable" } },
      {
        ...SESSION,
        target: M8,
        guidance: {
          class: "below_horizon",
          rise: Date.parse("2026-01-16T11:36:42Z") / 1000,
          transit: Date.parse("2026-01-16T15:13:18Z") / 1000,
          set: Date.parse("2026-01-16T18:50:54Z") / 1000,
        },
      },
      { ...SESSION, target: M8, guidance: { class: "too_low", never: true, transit: TIME + 3600 } },
      { ...SESSION, target: M75, guidance: { class: "sun_guard", turn_deg: 3, tilt_deg: 4, rise: TIME } },
      {
        ...SESSION,
        site: null,
        target: { ...M78, id: "NGC 1976", messier: "M42" },
        guidance: { class: "no_site" },
      },
    ];
    const { rerender } = render(<GuidanceBlock session={SESSION} />);
    for (const session of withoutDirections) {
      rerender(<GuidanceBlock session={{ ...session, guidance: { ...session.guidance!, separation_deg: 2.83 } }} />);
      expect(block().querySelector("[data-push]")).toBeNull();
      expect(block().querySelector("[data-away]")).toBeNull();
    }
    const withSeparation = { ...SESSION, guidance: { ...SESSION.guidance!, separation_deg: 2.83 } };
    rerender(<GuidanceBlock session={withSeparation} stale />);
    expect(block().querySelector("[data-push]")).toBeNull();
    expect(block().querySelector("[data-away]")).toBeNull();
    rerender(<GuidanceBlock session={{ ...withSeparation, guidance: { ...withSeparation.guidance, stale: true } }} />);
    expect(block().querySelector("[data-push]")).toBeNull();
    expect(block().querySelector("[data-away]")).toBeNull();
    rerender(<GuidanceBlock session={withSeparation} />);
    expect(block().querySelector("[data-push]")?.textContent).toBe("right 33.6° · up 12.4°");
    expect(block().querySelector("[data-away]")?.textContent).toBe("2.8° away");
  });
});
