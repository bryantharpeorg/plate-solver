import { render } from "@testing-library/react";
import { StrictMode } from "react";
import { describe, expect, it } from "vitest";
import type { SolveReport } from "../api";
import { ORION2, VIRGO } from "../test/fixtures";
import { Chart, DEFAULT_LAYERS } from "./Chart";

function chart(report: SolveReport | null, live?: boolean, layers = DEFAULT_LAYERS) {
  return (
    <Chart
      report={report}
      live={live}
      imageUrl="/frame.jpg"
      label="frame"
      busy={false}
      layers={layers}
      onLayers={() => {}}
    />
  );
}

function frame(base: SolveReport = ORION2, dra = 0): SolveReport {
  return { ...base, name: null, pointing: { ...base.pointing!, ra_deg: base.pointing!.ra_deg + dra } };
}

function unsolved(): SolveReport {
  return { ...ORION2, name: null, pointing: null, stars: [], objects: [], grid: [], status: "no_match" };
}

function star(container: HTMLElement, id: string) {
  const node = container.querySelector(`[data-star="${id}"]`);
  expect(node).not.toBeNull();
  return node!;
}

function sameGrid(container: HTMLElement, before: Element[]) {
  const after = [...container.querySelectorAll(".grid-line")];
  expect(before.length).toBeGreaterThan(0);
  expect(after).toHaveLength(before.length);
  after.forEach((node, i) => expect(node).toBe(before[i]));
}

// Oracle: saved reports supply the marks; DOM identity and intro classes are exact
// expectations (zero tolerance). Every live update below uses a new report object.
describe("the live chart", () => {
  it("keeps the SVG, grid paths in order and every star across a 0.004 degree move (S1)", () => {
    const { container, rerender } = render(chart(frame(), true));
    const svg = container.querySelector('svg[role="img"]');
    const grid = [...container.querySelectorAll(".grid-line")];
    const stars = ORION2.stars.map((s) => star(container, s.id));

    rerender(chart(frame(ORION2, 0.004), true));

    expect(container.querySelector('svg[role="img"]')).toBe(svg);
    sameGrid(container, grid);
    ORION2.stars.forEach((s, i) => expect(star(container, s.id)).toBe(stars[i]));
  });

  it("mounts a different SVG for a different upload (S2, negative control)", () => {
    const { container, rerender } = render(chart(ORION2));
    const svg = container.querySelector('svg[role="img"]');
    expect(svg).not.toBeNull();

    rerender(chart(VIRGO));

    const next = container.querySelector('svg[role="img"]');
    expect(next).not.toBeNull();
    expect(next).not.toBe(svg);
  });

  it("inks in an entering star while keeping every persisting star's node (S3)", () => {
    const first = { ...frame(), stars: ORION2.stars.slice(1) };
    const { container, rerender } = render(chart(first, true));
    const stars = first.stars.map((s) => star(container, s.id));
    expect(container.querySelector(`[data-star="${ORION2.stars[0].id}"]`)).toBeNull();

    rerender(chart(frame(ORION2, 0.004), true));

    first.stars.forEach((s, i) => expect(star(container, s.id)).toBe(stars[i]));
    expect(star(container, ORION2.stars[0].id).classList.contains("ink-in")).toBe(true);
  });

  it("holds two unsolved frames, drops on the third and recovers without another intro (S4–S6)", () => {
    const { container, rerender } = render(chart(frame(), true));
    const svg = container.querySelector('svg[role="img"]');
    const grid = [...container.querySelectorAll(".grid-line")];

    for (let i = 0; i < 2; i++) {
      rerender(chart(unsolved(), true));
      expect(container.querySelector('svg[role="img"]')).toBe(svg);
      sameGrid(container, grid);
      expect(container.querySelectorAll("[data-star]")).toHaveLength(ORION2.stars.length);
    }

    rerender(chart(unsolved(), true));
    expect(container.querySelector('svg[role="img"]')).toBe(svg);
    expect(container.querySelector(".grid-line")).toBeNull();
    expect(container.querySelectorAll(".mark-detected")).toHaveLength(ORION2.detected.length);

    rerender(chart(frame(ORION2, 0.012), true));
    expect(container.querySelector('svg[role="img"]')).toBe(svg);
    expect(container.querySelectorAll(".grid-line").length).toBeGreaterThan(0);
    expect(container.querySelector(".grid-line.draw")).toBeNull();
    expect(container.querySelector(".furniture")).not.toBeNull();
    expect(container.querySelector(".furniture.ink-in")).toBeNull();
    expect(container.querySelector(".edge-label")?.parentElement?.parentElement?.classList.contains("ink-in"))
      .toBe(false);
  });

  it("mounts a fresh SVG on every switch to or from live and replays the live intro (S7, negative control)", () => {
    const { container, rerender } = render(chart(ORION2));
    let svg = container.querySelector('svg[role="img"]');
    expect(svg).not.toBeNull();

    for (const element of [chart(frame(), true), chart(VIRGO), chart(frame(), true)]) {
      rerender(element);
      const next = container.querySelector('svg[role="img"]');
      expect(next).not.toBeNull();
      expect(next).not.toBe(svg);
      expect(container.querySelector(".grid-line.draw")).not.toBeNull();
      svg = next;
    }
  });

  it("resets the unsolved count when an intervening live frame solves (S8)", () => {
    const { container, rerender } = render(chart(frame(), true));
    const grid = [...container.querySelectorAll(".grid-line")];

    rerender(chart(unsolved(), true));
    rerender(chart(frame(ORION2, 0.004), true));
    rerender(chart(unsolved(), true));
    rerender(chart(unsolved(), true));

    sameGrid(container, grid);
  });

  it("counts a report object once despite layer changes and StrictMode renders", () => {
    const { container, rerender } = render(<StrictMode>{chart(frame(), true)}</StrictMode>);
    const grid = [...container.querySelectorAll(".grid-line")];
    const miss = unsolved();

    rerender(<StrictMode>{chart(miss, true)}</StrictMode>);
    rerender(<StrictMode>{chart(miss, true, { ...DEFAULT_LAYERS, detected: true })}</StrictMode>);
    rerender(<StrictMode>{chart(miss, true)}</StrictMode>);
    rerender(<StrictMode>{chart(unsolved(), true)}</StrictMode>);
    sameGrid(container, grid);

    rerender(<StrictMode>{chart(unsolved(), true)}</StrictMode>);
    expect(container.querySelector(".grid-line")).toBeNull();
  });

  it("shows unsolved frames until the first solve, which still plays its intro", () => {
    const { container, rerender } = render(chart(unsolved(), true));
    const svg = container.querySelector('svg[role="img"]');
    expect(container.querySelector(".grid-line")).toBeNull();
    expect(container.querySelectorAll(".mark-detected")).toHaveLength(ORION2.detected.length);
    rerender(chart(unsolved(), true));
    rerender(chart(unsolved(), true));

    rerender(chart(frame(), true));
    expect(container.querySelector('svg[role="img"]')).toBe(svg);
    expect(container.querySelector(".grid-line.draw")).not.toBeNull();
    expect(container.querySelector(".furniture.ink-in")).not.toBeNull();
  });

  it("keeps grid paths with their sky lines when the lines change order", () => {
    const first = frame();
    const { container, rerender } = render(chart(first, true));
    const paths = [...container.querySelectorAll(".grid-line")];
    let offset = 0;
    const lines = first.grid.map((line) => {
      const nodes = paths.slice(offset, offset + line.paths.length);
      offset += line.paths.length;
      return nodes;
    });

    rerender(chart({ ...frame(ORION2, 0.004), grid: [...first.grid].reverse() }, true));

    sameGrid(container, lines.reverse().flat());
  });

  it("moves persisting marks in place and retains their mounted intro classes", () => {
    const { container, rerender } = render(chart(frame(), true));
    const original = star(container, ORION2.stars[0].id);
    const objects = ORION2.objects.map((o) => container.querySelector(`[data-object="${o.id}"]`));
    objects.forEach((node) => expect(node).not.toBeNull());
    const next = frame(ORION2, 0.004);
    next.stars = ORION2.stars.map((s) => ({ ...s, x: s.x + 1, y: s.y + 2, matched: !s.matched }));
    next.objects = ORION2.objects.map((o) => ({ ...o, x: o.x + 1, y: o.y + 2 }));

    rerender(chart(next, true));

    expect(star(container, next.stars[0].id)).toBe(original);
    expect(original.querySelector(".ring")?.getAttribute("cx")).toBe(String(next.stars[0].x));
    expect(original.querySelector(".ring")?.getAttribute("cy")).toBe(String(next.stars[0].y));
    expect(original.classList.contains(next.stars[0].matched ? "mark-matched" : "mark-expected")).toBe(true);
    next.objects.forEach((o, i) => {
      const node = container.querySelector(`[data-object="${o.id}"]`);
      expect(node).toBe(objects[i]);
      expect(node?.getAttribute("transform")).toBe(`translate(${o.x} ${o.y})`);
    });
    expect(container.querySelector(".grid-line.draw")).not.toBeNull();
    expect(container.querySelector(".furniture.ink-in")).not.toBeNull();
  });

  it("remounts a toggled grid without intro after live reports have settled", () => {
    const { container, rerender } = render(chart(frame(), true));
    const next = frame(ORION2, 0.004);
    rerender(chart(next, true));
    rerender(chart(next, true, { ...DEFAULT_LAYERS, grid: false }));
    expect(container.querySelector(".grid-line")).toBeNull();

    rerender(chart(next, true));
    expect(container.querySelectorAll(".grid-line").length).toBeGreaterThan(0);
    expect(container.querySelector(".grid-line.draw")).toBeNull();
    expect(container.querySelector(".grid-line")?.getAttribute("style")).toBeNull();
  });
});
