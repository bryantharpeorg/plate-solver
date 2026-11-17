import { describe, expect, it } from "vitest";
import type { ObjectMark, SolveReport } from "./api";
import {
  boxesOverlap,
  layoutLabels,
  OBJECT_TEXT,
  ringRadius,
  STAR_TEXT,
  type Box,
  type LabelInput,
  type LabelSpot,
} from "./labels";
import { fromView, toView } from "./rotation";
import { ORION2 } from "./test/fixtures";

// Oracle: plan.md § Interfaces' turned-view geometry. Every star label is laid out where
// it is seen and checked against toView and fromView, never against the layout's own
// arithmetic. WIDE is the plan's non-square frame check: a square fixture like ORION2
// would hide a swapped box.

/** ORION2's marks, spread across the plan's wide 1296 × 972 frame. */
const WIDE: SolveReport = {
  ...ORION2,
  width: 1296,
  height: 972,
  stars: ORION2.stars.map((s) => ({ ...s, x: (s.x * 1296) / 507, y: (s.y * 972) / 507 })),
  objects: ORION2.objects.map((o): ObjectMark => ({
    ...o,
    x: (o.x * 1296) / 507,
    y: (o.y * 972) / 507,
    radius_px: (o.radius_px * 972) / 507,
    minor_px: (o.minor_px * 972) / 507,
  })),
  detected: ORION2.detected.map(([x, y]): [number, number] => [(x * 1296) / 507, (y * 972) / 507]),
};

const measure = (text: string, size: number) => text.length * size * 0.62;

function input(rotation?: number): LabelInput {
  return {
    W: WIDE.width,
    H: WIDE.height,
    u: WIDE.width / (480 - 60),
    stars: WIDE.stars,
    objects: WIDE.objects,
    detected: WIDE.detected,
    furniture: [],
    measure,
    ...(rotation !== undefined && { rotation }),
  };
}

const around = (x: number, y: number, r: number): Box => ({ x0: x - r, y0: y - r, x1: x + r, y1: y + r });

const boxOf = (spot: LabelSpot): Box => {
  expect(spot.box).toBeDefined();
  return spot.box as Box;
};

/** The box's own anchor point: where its text's anchor sits on its baseline. */
function anchorPoint(b: Box, anchor: LabelSpot["anchor"], size: number): [number, number] {
  return [anchor === "start" ? b.x0 : anchor === "end" ? b.x1 : (b.x0 + b.x1) / 2, b.y1 - 0.22 * size];
}

const toSeen = (deg: number, x: number, y: number) => toView(WIDE.width, WIDE.height, deg, x, y);

for (const deg of [90, 79]) {
  describe(`labels laid out in the view turned by ${deg}°`, () => {
    const u = WIDE.width / (480 - 60);
    const spots = layoutLabels(input(deg));
    const placed = [...spots.entries()];
    const starLabels = placed.filter(([key]) => key.startsWith("s:"));

    it("still places the belt stars' names", () => {
      for (const name of ["Alnitak", "Alnilam", "Mintaka", "Saiph"]) {
        const star = WIDE.stars.find((s) => s.name === name);
        expect(star, name).toBeDefined();
        expect(spots.has(`s:${star!.id}`), name).toBe(true);
      }
    });

    it("keeps every star label off every other label, every ring and every detection, where they are drawn", () => {
      expect(starLabels.length).toBeGreaterThan(0);
      for (const [key, spot] of starLabels) {
        const b = boxOf(spot);
        for (const [other, otherSpot] of placed) {
          if (other !== key) expect(boxesOverlap(b, boxOf(otherSpot)), `${key} over ${other}`).toBe(false);
        }
        for (const s of WIDE.stars) {
          const [sx, sy] = toSeen(deg, s.x, s.y);
          expect(boxesOverlap(b, around(sx, sy, (ringRadius(s) + 1) * u)), `${key} over ${s.id}'s ring`).toBe(false);
        }
        for (const [x, y] of WIDE.detected) {
          const [dx, dy] = toSeen(deg, x, y);
          expect(boxesOverlap(b, around(dx, dy, 4 * u)), `${key} over a detection`).toBe(false);
        }
      }
    });

    it("keeps every star label's box, its corners taken back through fromView, 3u inside the frame", () => {
      for (const [key, spot] of starLabels) {
        const b = boxOf(spot);
        for (const [cx, cy] of [
          [b.x0, b.y0],
          [b.x1, b.y0],
          [b.x0, b.y1],
          [b.x1, b.y1],
        ] as [number, number][]) {
          const [fx, fy] = fromView(WIDE.width, WIDE.height, deg, cx, cy);
          expect(
            fx >= 3 * u && fx <= WIDE.width - 3 * u && fy >= 3 * u && fy <= WIDE.height - 3 * u,
            `${key}'s corner (${cx}, ${cy})`,
          ).toBe(true);
        }
      }
    });

    it("centres every label's drawn text on its laid-out box, the sum taken before toView", () => {
      for (const [key, spot] of placed) {
        const b = boxOf(spot);
        const size = key.startsWith("s:") ? STAR_TEXT * u : OBJECT_TEXT * u;
        const [ax, ay] = anchorPoint(b, spot.anchor, size);
        const cx = (b.x0 + b.x1) / 2;
        const cy = (b.y0 + b.y1) / 2;
        const [vx, vy] = toView(WIDE.width, WIDE.height, deg, spot.x + (cx - ax), spot.y + (cy - ay));
        expect(vx, key).toBeCloseTo(cx, 6);
        expect(vy, key).toBeCloseTo(cy, 6);
      }
    });
  });
}

describe("labels laid out with no turn", () => {
  it("lays out at 0° exactly as it does with no rotation at all", () => {
    expect(layoutLabels(input(0))).toEqual(layoutLabels(input()));
  });
});
