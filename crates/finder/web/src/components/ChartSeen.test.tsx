import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it } from "vitest";
import type { SolveReport, StarMark } from "../api";
import { ORION2 } from "../test/fixtures";
import { Chart, DEFAULT_LAYERS } from "./Chart";

function Harness({ stars }: { stars: StarMark[] }) {
  const [layers, setLayers] = useState(DEFAULT_LAYERS);
  const report: SolveReport = { ...ORION2, stars, objects: [] };
  return (
    <Chart report={report} imageUrl="/frame.jpg" label="frame" busy={false} layers={layers} onLayers={setLayers} />
  );
}
const a: StarMark = { ...ORION2.stars[0]!, id: "A", name: "Star A", matched: true };
const b: StarMark = { ...ORION2.stars[1]!, id: "B", name: "Star B", matched: false, seen: true };
function star(container: HTMLElement, id: string) {
  const node = container.querySelector(`[data-star="${id}"]`);
  expect(node).not.toBeNull();
  return node!;
}

// Oracle: spec 020's exact classes/captions, and equality with a matched ring's
// radius in the same render (zero tolerance), using saved fixture positions.
describe("seen star marks", () => {
  it("gives a held star the matched class, radius, delay and label, and shrinks it when unseen (S9)", () => {
    const { container, rerender } = render(<Harness stars={[a, b]} />);
    const ring = (id: string) => Number(star(container, id).querySelector(".ring")!.getAttribute("r"));
    expect(star(container, "B").classList.contains("mark-matched")).toBe(true);
    expect(ring("B")).toBe(ring("A"));
    expect(star(container, "B").getAttribute("style")).toBe(star(container, "A").getAttribute("style"));
    expect(star(container, "B").querySelector(".mark-label")?.textContent).toBe("Star B");
    rerender(<Harness stars={[a, { ...b, seen: false }]} />);
    expect(star(container, "B").classList.contains("mark-expected")).toBe(true);
    expect(ring("B")).toBeLessThan(ring("A"));
  });

  it("explains detected, held, unseen and matched stars when tapped (S10)", () => {
    const stars = [
      a,
      { ...b, id: "C", detected: true, seen: true },
      { ...b, id: "D", detected: false, seen: true },
      { ...b, id: "E", detected: false, seen: false },
    ];
    const { container } = render(<Harness stars={stars} />);
    for (const [id, text] of [
      ["C", "Detected; not used in the solve"],
      ["D", "Seen in recent frames, not in this one"],
      ["E", "In the catalog but not detected"],
    ]) {
      fireEvent.click(star(container, id!));
      expect(container.querySelector("figcaption")!.textContent).toContain(text);
    }
    fireEvent.click(star(container, "A"));
    expect(container.querySelector("figcaption")!.textContent).toContain("Matched; the solution puts it");
  });

  it("counts and filters the legend by seen, falling back to matched or detected", () => {
    const { container } = render(
      <Harness stars={[a, b, { ...b, id: "C", seen: undefined, detected: true }, { ...b, id: "D", seen: false }]} />,
    );
    expect(screen.getByRole("button", { name: "Matched3" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Unseen1" })).toBeTruthy();
    expect(star(container, "C").classList.contains("mark-matched")).toBe(true);
    fireEvent.click(screen.getByRole("button", { name: /Matched/ }));
    for (const id of ["A", "B", "C"]) expect(container.querySelector(`[data-star="${id}"]`)).toBeNull();
    star(container, "D");
    fireEvent.click(screen.getByRole("button", { name: /Unseen/ }));
    expect(container.querySelectorAll("[data-star]")).toHaveLength(0);
  });
});
