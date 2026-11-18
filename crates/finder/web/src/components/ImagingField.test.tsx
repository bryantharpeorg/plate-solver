// The imaging camera's field on the live chart (032 US6). Oracles: the worked
// corners of US6-S5, each number within 1e-9, and the path those corners write
// to one decimal with the stated conditions of US6-S6.
import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { ObservingSession, SolveReport } from "../api";
import { ORION2 } from "../test/fixtures";
import { Chart, DEFAULT_LAYERS } from "./Chart";
import { imagingFieldCorners } from "./ImagingField";

const AIM = { x: 700, y: 420 };
const FIELD = { width_arcsec: 2700, height_arcsec: 1800 };

describe("imagingFieldCorners", () => {
  it("gives the worked corners at an imaging angle of 0°, not mirrored", () => {
    const corners = imagingFieldCorners(AIM, 0, false, FIELD, 30)!;
    expect(corners).toHaveLength(4);
    const expected: [number, number][] = [
      [655, 390],
      [745, 390],
      [745, 450],
      [655, 450],
    ];
    expected.forEach(([x, y], i) => {
      expect(corners[i]![0]).toBeCloseTo(x, 9);
      expect(corners[i]![1]).toBeCloseTo(y, 9);
    });
  });

  it("gives the worked corners at an imaging angle of 90°", () => {
    const corners = imagingFieldCorners(AIM, 90, false, FIELD, 30)!;
    const expected: [number, number][] = [
      [670, 465],
      [670, 375],
      [730, 375],
      [730, 465],
    ];
    expected.forEach(([x, y], i) => {
      expect(corners[i]![0]).toBeCloseTo(x, 9);
      expect(corners[i]![1]).toBeCloseTo(y, 9);
    });
  });

  it("turns the imaging frame's other way round when mirrored", () => {
    const corners = imagingFieldCorners(AIM, 0, true, FIELD, 30)!;
    const expected: [number, number][] = [
      [745, 390],
      [655, 390],
      [655, 450],
      [745, 450],
    ];
    expected.forEach(([x, y], i) => {
      expect(corners[i]![0]).toBeCloseTo(x, 9);
      expect(corners[i]![1]).toBeCloseTo(y, 9);
    });
  });

  it("answers null for a finder scale that is not above 0", () => {
    expect(imagingFieldCorners(AIM, 0, false, FIELD, 0)).toBeNull();
  });
});

// The finder scale the worked report carries; the relation's field is 90 and 60
// times it, so the corners are the worked ones of US6-S5.
const SCALE = ORION2.pointing!.arcsec_per_px;

const measured = (relation: object = {}): ObservingSession => ({
  site: null,
  aim: { x: 700, y: 420, source: "measured" },
  imaging: {
    state: "ready",
    relation: {
      angle_deg: 0,
      mirrored: false,
      field: { width_arcsec: 90 * SCALE, height_arcsec: 60 * SCALE },
      ...relation,
    },
  },
});

const chart = (report: SolveReport | null, live: boolean, session?: ObservingSession) => (
  <Chart
    report={report}
    live={live}
    imageUrl="/frame.jpg"
    label="frame"
    busy={false}
    layers={DEFAULT_LAYERS}
    onLayers={() => {}}
    session={session}
  />
);

function unsolved(): SolveReport {
  return { ...ORION2, name: null, pointing: null, stars: [], objects: [], grid: [], status: "no_match" };
}

describe("the live chart's imaging field", () => {
  it("draws one titled field with the worked path while the aim point is measured", () => {
    const { container } = render(chart({ ...ORION2, name: null }, true, measured()));
    const drawn = container.querySelectorAll("[data-imaging-field]");
    expect(drawn).toHaveLength(1);
    const field = drawn[0]!;
    expect(field.querySelector("title")!.textContent).toBe("Imaging field");
    expect(
      (field.querySelector("path")!.getAttribute("d") ?? "").startsWith(
        "M655.0 390.0L745.0 390.0L745.0 450.0L655.0 450.0Z",
      ),
    ).toBe(true);
  });

  it.each([
    ["a tapped aim point", { ...ORION2, name: null }, true, { ...measured(), aim: { x: 700, y: 420 } }],
    ["a chart that is not live", { ...ORION2, name: null }, false, measured()],
    ["an unsolved report", unsolved(), true, measured()],
    ["a relation with no field", { ...ORION2, name: null }, true, measured({ field: undefined })],
  ] as const)("draws no imaging field for %s", (_label, report, live, session) => {
    const { container } = render(chart(report, live, session));
    expect(container.querySelector("[data-imaging-field]")).toBeNull();
  });
});
