import { render } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it } from "vitest";
import type { ObservingSession, SolveReport } from "../api";
import { ORION2 } from "../test/fixtures";
import { SESSION } from "../test/targets";
import { Chart, DEFAULT_LAYERS } from "./Chart";

// Oracles: US3's explicit SVG contract and the saved 507 × 507 ORION2 report.
// Transforms and presence are exact; label bearings allow 0.01°, distances 1e-9
// chart units. jsdom's 320px fallback determines the scale, without SVG layout.
const WITH_UP = {
  ...SESSION,
  aim: { x: 320.5, y: 190 },
  pointing: { ...SESSION.pointing!, up_deg: 78 },
};
const REPORT = { ...ORION2, pointing_seq: 7 };

function Harness({
  live = true,
  session = WITH_UP,
  report = REPORT,
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

describe("the chart's up mark", () => {
  it("draws up and right at the live aim point with upright labels at their bearings", () => {
    const { container } = render(<Harness />);
    const svg = container.querySelector('svg[role="img"]')!;
    expect(svg.querySelectorAll("[data-up]")).toHaveLength(1);
    const mark = svg.querySelector("[data-up]")!;
    expect(mark.tagName).toBe("g");
    expect(mark.getAttribute("transform")).toBe("translate(320.5 190) rotate(78)");
    expect(mark.classList.contains("pointer-events-none")).toBe(true);
    const u = ORION2.width / 320;
    expect(mark.getAttribute("stroke-width")).toBe(String(1.3 * u));
    const path = mark.querySelector("path")!;
    expect(path.getAttribute("fill")).toBe("none");
    expect(path.getAttribute("d")).toBe(
      `M0 ${-13 * u}V${-34 * u}M${-4 * u} ${-28 * u}L0 ${-34 * u}L${4 * u} ${-28 * u}M${13 * u} 0H${22 * u}`,
    );
    const aim = svg.querySelector("[data-aim]")!;
    expect(aim.compareDocumentPosition(mark) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(mark.parentElement).toBe(aim.parentElement);
    expect(mark.parentElement?.hasAttribute("clip-path")).toBe(true);
    const labels = [...svg.querySelectorAll("text[data-up-label]")];
    expect(labels.map((label) => label.textContent)).toEqual(["up", "right"]);
    for (const [i, label] of labels.entries()) {
      expect(mark.contains(label)).toBe(false);
      expect(label.parentElement).toBe(mark.parentElement);
      expect(label.hasAttribute("transform")).toBe(false);
      expect(label.classList.contains("pointer-events-none")).toBe(true);
      const dx = Number(label.getAttribute("x")) - 320.5;
      const dy = 190 - Number(label.getAttribute("y"));
      const bearing = ((Math.atan2(dx, dy) * 180) / Math.PI + 360) % 360;
      expect(Math.abs(bearing - (i === 0 ? 78 : 168))).toBeLessThan(0.01);
      expect(Math.abs(Math.hypot(dx, dy) - (i === 0 ? 43 : 31) * u)).toBeLessThan(1e-9);
    }
  });

  it("uses the frame centre without an aim point or when the chart is not live", () => {
    const { aim, ...withoutAim } = WITH_UP;
    const { container, rerender } = render(<Harness session={withoutAim} />);
    const centre = `translate(${ORION2.width / 2} ${ORION2.height / 2})`;
    expect(container.querySelector("[data-up]")?.getAttribute("transform")).toBe(`${centre} rotate(78)`);
    rerender(<Harness live={false} />);
    expect(container.querySelector("[data-up]")?.getAttribute("transform")).toBe(`${centre} rotate(78)`);
  });

  it("requires up_deg and a report for the session's latest pointing", () => {
    const { container, rerender } = render(<Harness />);
    expect(container.querySelectorAll("[data-up]")).toHaveLength(1);
    rerender(<Harness session={{ ...WITH_UP, pointing: SESSION.pointing }} />);
    expect(container.querySelector("[data-up], [data-up-label]")).toBeNull();
    rerender(<Harness report={{ ...REPORT, pointing_seq: 8 }} />);
    expect(container.querySelector("[data-up], [data-up-label]")).toBeNull();
    rerender(<Harness report={ORION2} />);
    expect(container.querySelector("[data-up], [data-up-label]")).toBeNull();
    rerender(<Harness />);
    expect(container.querySelectorAll("[data-up]")).toHaveLength(1);
    expect(container.querySelectorAll("[data-up-label]")).toHaveLength(2);
  });

  it("draws a zero bearing without a target and leaves no stray zero text", () => {
    const { container } = render(
      <Harness session={{ ...WITH_UP, target: null, pointing: { ...WITH_UP.pointing, up_deg: 0 } }} />,
    );
    const svg = container.querySelector('svg[role="img"]')!;
    expect(svg.querySelectorAll("[data-up]")).toHaveLength(1);
    expect(svg.querySelector("[data-up]")?.getAttribute("transform")).toBe("translate(320.5 190) rotate(0)");
    const text = document.createTreeWalker(svg, NodeFilter.SHOW_TEXT);
    while (text.nextNode()) {
      expect(text.currentNode.textContent?.trim()).not.toBe("0");
    }
    expect(svg.querySelectorAll("[data-up-label]")).toHaveLength(2);
  });
});
