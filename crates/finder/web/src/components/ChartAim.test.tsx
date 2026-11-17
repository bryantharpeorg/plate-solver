import { render, screen, within } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it } from "vitest";
import type { ObservingSession, SolveReport } from "../api";
import { ORION2 } from "../test/fixtures";
import { SESSION } from "../test/targets";
import { Chart, DEFAULT_LAYERS } from "./Chart";

// Oracle: US2's aim pixel and SVG contract, and the saved 507 × 507 ORION2 report.
// Coordinates, label attributes, legend counts and furniture are exact; radii use
// the chart's documented 320 CSS-pixel fallback in jsdom (which has no layout).
const SESSION_WITH_AIM: ObservingSession = { ...SESSION, aim: { x: 320.5, y: 190 } };

function Harness({
  live = true,
  session = SESSION_WITH_AIM,
  report = ORION2,
}: {
  live?: boolean;
  session?: ObservingSession;
  report?: SolveReport;
}) {
  const [layers, setLayers] = useState(DEFAULT_LAYERS);
  return (
    <Chart
      report={report}
      imageUrl="/frame.jpg"
      label="orion2"
      busy={false}
      layers={layers}
      onLayers={setLayers}
      live={live}
      session={session}
    />
  );
}

describe("the live chart's aim mark", () => {
  it("marks the aim pixel after the furniture with two circles and an upright label toward the middle", () => {
    const { container, rerender } = render(<Harness />);
    const svg = container.querySelector('svg[role="img"]')!;
    expect(svg.querySelectorAll("[data-aim]")).toHaveLength(1);
    const aim = svg.querySelector("[data-aim]")!;
    expect(aim.tagName).toBe("g");
    expect(aim.getAttribute("transform")).toBe("translate(320.5 190)");
    expect(aim.querySelector("title")?.textContent).toBe("Aim point: where the telescope looks");
    expect(aim.querySelectorAll("circle")).toHaveLength(2);
    expect(aim.querySelector(".aim-ring")?.getAttribute("r")).toBe(String((9 * 507) / 320));
    expect(aim.querySelector(".aim-dot")?.getAttribute("r")).toBe(String((1.6 * 507) / 320));
    expect(aim.classList.contains("pointer-events-none")).toBe(true);
    const label = aim.querySelector("text")!;
    expect(label.textContent).toBe("aim");
    expect(label.hasAttribute("transform")).toBe(false);
    expect(label.getAttribute("text-anchor")).toBe("end");
    expect(Number(label.getAttribute("x"))).toBeLessThan(0);
    const furniture = svg.querySelector(".furniture")!;
    expect(furniture.compareDocumentPosition(aim) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(aim.parentElement).toBe(furniture.parentElement);
    expect(aim.parentElement?.hasAttribute("clip-path")).toBe(true);

    rerender(<Harness session={{ ...SESSION_WITH_AIM, aim: { x: 190, y: 190 } }} />);
    const leftLabel = svg.querySelector("[data-aim] text")!;
    expect(leftLabel.getAttribute("text-anchor")).toBe("start");
    expect(Number(leftLabel.getAttribute("x"))).toBeGreaterThan(0);
    expect(leftLabel.hasAttribute("transform")).toBe(false);
  });

  it("shows the mark only when the chart is live and an aim point is set", () => {
    const { container, rerender } = render(<Harness />);
    expect(container.querySelectorAll("[data-aim]")).toHaveLength(1);

    rerender(<Harness live={false} />);
    expect(container.querySelector("[data-aim]")).toBeNull();

    rerender(<Harness session={SESSION} />);
    expect(container.querySelector("[data-aim]")).toBeNull();

    rerender(<Harness />);
    expect(container.querySelectorAll("[data-aim]")).toHaveLength(1);
  });

  it("leaves all five legend chips, the furniture and the target mark unchanged", () => {
    const { container, rerender } = render(<Harness session={SESSION} />);
    const furniture = container.querySelector(".furniture")!.outerHTML;
    rerender(<Harness />);
    expect(container.querySelectorAll("[data-aim]")).toHaveLength(1);
    expect(container.querySelectorAll(".furniture")).toHaveLength(1);
    expect(container.querySelector(".furniture")!.outerHTML).toBe(furniture);
    expect(
      within(screen.getByRole("group", { name: "Show on the chart" }))
        .getAllByRole("button")
        .map((button) => [button.textContent, button.getAttribute("aria-pressed")]),
    ).toEqual([
      ["Grid12", "true"],
      ["Matched30", "true"],
      ["Unseen25", "true"],
      ["Deep sky3", "true"],
      ["Centroids67", "false"],
    ]);

    const report = { ...ORION2, pointing_seq: SESSION.pointing!.seq };
    rerender(<Harness report={report} session={SESSION} />);
    const target = container.querySelector('[data-target="M78"]')!.outerHTML;
    rerender(<Harness report={report} />);
    expect(container.querySelector('[data-target="M78"]')!.outerHTML).toBe(target);
  });

  it("still marks the aim pixel when the live frame has no solution", () => {
    const { container } = render(<Harness report={{ ...ORION2, pointing: null }} />);
    expect(container.querySelectorAll("[data-aim]")).toHaveLength(1);
    expect(container.querySelector("[data-aim]")?.getAttribute("transform")).toBe("translate(320.5 190)");
  });
});
