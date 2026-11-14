// Where the chart's names go. A label sits beside its mark where it covers nothing else
// drawn on the chart: no other label, star ring, detection, object outline, or the chart's
// furniture. Close in first; further out with a leader line when the close spots are
// taken. Objects always get a label (an atlas never leaves an object unnamed); a star whose
// name finds no room goes without.

import type { ObjectMark, StarMark } from "./api";
import { seenNow } from "./calm";

/** A rectangle in image pixels. */
export interface Box {
  x0: number;
  y0: number;
  x1: number;
  y1: number;
}

/** A placed label: its text anchor point, and a leader from the mark when set off. */
export interface LabelSpot {
  x: number;
  y: number;
  anchor: "start" | "middle" | "end";
  leader?: { x1: number; y1: number; x2: number; y2: number };
  /** The text's extent, as laid out. */
  box?: Box;
}

/** Text width in CSS pixels for a label set at `size` CSS pixels. */
export type Measure = (text: string, size: number, italic: boolean) => number;

export interface LabelInput {
  /** Frame size in image pixels. */
  W: number;
  H: number;
  /** Image pixels per CSS pixel. */
  u: number;
  /** Stars drawn on the chart. */
  stars: StarMark[];
  /** Objects drawn on the chart. */
  objects: ObjectMark[];
  /** Detections drawn on the chart. */
  detected: [number, number][];
  /** The chart's furniture: compass, scale bar, centre reticle. */
  furniture: Box[];
  measure: Measure;
}

/** Labels' sizes in CSS pixels. */
export const OBJECT_TEXT = 13;
export const STAR_TEXT = 12;

/** A star ring's radius in CSS pixels. */
export const ringRadius = (s: StarMark) => ((s.seen ?? seenNow(s)) ? 7.5 : 4.5);

/** An object symbol's radius in image pixels: its true size, but never smaller than a mark. */
export const objectRadius = (o: ObjectMark, u: number) => Math.max(o.radius_px, 9 * u);

/** A galaxy's minor semi-axis in image pixels, as drawn. */
export const galaxyMinor = (o: ObjectMark, u: number) => Math.max(Math.min(o.minor_px, objectRadius(o, u)), 4 * u);

/** How far an object's symbol reaches from its centre in direction `(dx, dy)` (a unit vector). */
export function reach(o: ObjectMark, u: number, dx: number, dy: number): number {
  const r = objectRadius(o, u);
  if (o.family === "galaxy") {
    // The major axis lies `angle_deg` clockwise from up.
    const a = ((o.angle_deg - 90) * Math.PI) / 180;
    const along = dx * Math.cos(a) + dy * Math.sin(a);
    const across = -dx * Math.sin(a) + dy * Math.cos(a);
    return Math.hypot(r * along, galaxyMinor(o, u) * across);
  }
  if (o.family === "nebula" || o.family === "supernova_remnant") return r * (Math.abs(dx) + Math.abs(dy));
  return r;
}

/** Points along an object's outline, for keeping other labels off it. */
export function outline(o: ObjectMark, u: number): [number, number][] {
  const points: [number, number][] = [];
  for (let i = 0; i < 48; i++) {
    const t = (2 * Math.PI * i) / 48;
    const [dx, dy] = [Math.cos(t), Math.sin(t)];
    if (o.family === "nebula" || o.family === "supernova_remnant") {
      // A square: its edge where the ray from the centre leaves it.
      const r = objectRadius(o, u) / Math.max(Math.abs(dx), Math.abs(dy));
      points.push([o.x + dx * r, o.y + dy * r]);
    } else if (o.family === "galaxy") {
      const a = ((o.angle_deg - 90) * Math.PI) / 180;
      const ex = objectRadius(o, u) * dx;
      const ey = galaxyMinor(o, u) * dy;
      points.push([o.x + ex * Math.cos(a) - ey * Math.sin(a), o.y + ex * Math.sin(a) + ey * Math.cos(a)]);
    } else {
      const r = objectRadius(o, u);
      points.push([o.x + dx * r, o.y + dy * r]);
    }
  }
  return points;
}

/** Whether two boxes overlap. */
export const boxesOverlap = (a: Box, b: Box) => a.x0 < b.x1 && a.x1 > b.x0 && a.y0 < b.y1 && a.y1 > b.y0;

const overlapArea = (a: Box, b: Box) =>
  Math.max(0, Math.min(a.x1, b.x1) - Math.max(a.x0, b.x0)) * Math.max(0, Math.min(a.y1, b.y1) - Math.max(a.y0, b.y0));

/** The eight directions around a mark, the side ones first: names read best beside a mark. */
const DIRECTIONS: [number, number][] = [
  [1, 0],
  [-1, 0],
  [0, -1],
  [0, 1],
  [Math.SQRT1_2, -Math.SQRT1_2],
  [-Math.SQRT1_2, -Math.SQRT1_2],
  [Math.SQRT1_2, Math.SQRT1_2],
  [-Math.SQRT1_2, Math.SQRT1_2],
];

/** Sixteen directions, for labels set further off on a leader, where there is room to choose. */
const FAR_DIRECTIONS: [number, number][] = [
  ...DIRECTIONS,
  ...[22.5, 67.5, 112.5, 157.5, 202.5, 247.5, 292.5, 337.5].map((a): [number, number] => [
    Math.cos((a * Math.PI) / 180),
    Math.sin((a * Math.PI) / 180),
  ]),
];

/**
 * A label `w` wide set `d` from a mark in direction `(dx, dy)`. The box lies wholly beyond
 * the line at `d - 0.3 size` across that direction, so a label set past a symbol's reach
 * never crosses the symbol.
 */
function candidate(x: number, y: number, dx: number, dy: number, d: number, w: number, size: number) {
  const anchor: LabelSpot["anchor"] = dx > 0.3 ? "start" : dx < -0.3 ? "end" : "middle";
  const cx = x + dx * d;
  // The baseline: level with the mark beside it, above or below it otherwise.
  const by = dy < -0.3 ? y + dy * d : dy > 0.3 ? y + dy * d + size * 0.75 : y + size * 0.35;
  const x0 = anchor === "start" ? cx : anchor === "end" ? cx - w : cx - w / 2;
  const box: Box = { x0, y0: by - size * 0.78, x1: x0 + w, y1: by + size * 0.22 };
  return { spot: { x: cx, y: by, anchor } as LabelSpot, box };
}

/**
 * Lay out every label: objects first (in the report's order: the ones in the frame and
 * brightest first), then the named stars brightest first. Keys are `o:<id>` and `s:<id>`.
 */
export function layoutLabels(input: LabelInput): Map<string, LabelSpot> {
  const { W, H, u, measure } = input;
  const pad = 2 * u;
  const grow = (b: Box): Box => ({ x0: b.x0 - pad, y0: b.y0 - pad, x1: b.x1 + pad, y1: b.y1 + pad });
  const around = (x: number, y: number, r: number): Box => ({ x0: x - r, y0: y - r, x1: x + r, y1: y + r });

  // Everything drawn, each with the key of the mark it belongs to ("" for none) and how
  // much covering it costs. Nothing may cover the furniture while any other spot is left.
  const obstacles: (Box & { owner: string; weight?: number })[] = input.furniture.map((b) => ({
    ...b,
    owner: "",
    weight: 1000,
  }));
  for (const s of input.stars) obstacles.push({ ...around(s.x, s.y, (ringRadius(s) + 1) * u), owner: `s:${s.id}` });
  for (const [x, y] of input.detected) obstacles.push({ ...around(x, y, 4 * u), owner: "" });
  for (const o of input.objects) {
    for (const [x, y] of outline(o, u)) obstacles.push({ ...around(x, y, 1.5 * u), owner: `o:${o.id}` });
  }
  const labels: Box[] = [];

  const margin = 3 * u;
  /** The box moved, if need be, to lie inside the frame. */
  const intoFrame = (b: Box): [number, number] => [
    Math.max(margin - b.x0, 0) - Math.max(b.x1 - (W - margin), 0),
    Math.max(margin - b.y0, 0) - Math.max(b.y1 - (H - margin), 0),
  ];
  /** How much a label box would cover: other labels count four times over. */
  const cost = (b: Box, key: string, skipOwn: boolean) => {
    const g = grow(b);
    let c = 0;
    for (const l of labels) c += overlapArea(g, l) * 4;
    for (const o of obstacles) if (!(skipOwn && o.owner === key)) c += overlapArea(g, o) * (o.weight ?? 1);
    return c;
  };

  const spots = new Map<string, LabelSpot>();
  const place = (
    key: string,
    text: string,
    x: number,
    y: number,
    reachTo: (dx: number, dy: number) => number,
    size: number,
    italic: boolean,
    always: boolean,
  ) => {
    const w = measure(text, size / u, italic) * u;
    let best: { spot: LabelSpot; box: Box; cost: number; off: boolean } | null = null;
    search: for (const [ring, gap] of [4, 16, 30, 46].entries()) {
      for (const [dx, dy] of ring === 0 ? DIRECTIONS : FAR_DIRECTIONS) {
        const c = candidate(x, y, dx, dy, reachTo(dx, dy) + gap * u, w, size);
        const [mx, my] = intoFrame(c.box);
        const moved = mx !== 0 || my !== 0;
        // Only objects may be pulled into the frame (an object at the edge still gets its
        // name); a moved box may land on its own symbol, so then the symbol counts too.
        if (moved && !always) continue;
        const box = { x0: c.box.x0 + mx, y0: c.box.y0 + my, x1: c.box.x1 + mx, y1: c.box.y1 + my };
        const spot: LabelSpot = { ...c.spot, x: c.spot.x + mx, y: c.spot.y + my };
        const k = cost(box, key, !moved);
        const off = ring > 0 || moved;
        if (k === 0 && !moved) {
          best = { spot, box, cost: 0, off };
          break search;
        }
        if (always && (!best || k < best.cost)) best = { spot, box, cost: k, off };
      }
    }
    if (!best || (best.cost > 0 && !always)) return;
    const { spot, box } = best;
    if (best.off) {
      // A leader from the symbol's edge to the label's nearest point, short of both.
      const ex = Math.min(Math.max(x, box.x0), box.x1);
      const ey = Math.min(Math.max(y, box.y0), box.y1);
      const len = Math.hypot(ex - x, ey - y);
      const ux = (ex - x) / len;
      const uy = (ey - y) / len;
      const start = reachTo(ux, uy) + 2 * u;
      if (len - start > 6 * u) {
        spot.leader = { x1: x + ux * start, y1: y + uy * start, x2: ex - ux * 2 * u, y2: ey - uy * 2 * u };
      }
    }
    labels.push(grow(box));
    spots.set(key, { ...spot, box });
  };

  for (const o of input.objects) {
    place(`o:${o.id}`, o.id, o.x, o.y, (dx, dy) => reach(o, u, dx, dy), OBJECT_TEXT * u, true, true);
  }
  const named = input.stars.filter((s) => s.name).sort((a, b) => a.mag - b.mag);
  for (const s of named) {
    const r = ringRadius(s) * u;
    place(`s:${s.id}`, s.name ?? "", s.x, s.y, () => r, STAR_TEXT * u, false, false);
  }
  return spots;
}

/** Text width from the page's own font when a canvas can measure it, else an estimate. */
export function measureText(): Measure {
  let context: CanvasRenderingContext2D | null = null;
  try {
    const canvas = typeof document === "undefined" ? null : document.createElement("canvas");
    // jsdom has a canvas element but no 2D context; it says so on the console.
    context =
      canvas && typeof navigator !== "undefined" && !/jsdom/.test(navigator.userAgent) ? canvas.getContext("2d") : null;
  } catch {
    context = null;
  }
  return (text, size, italic) => {
    if (context) {
      context.font = `${italic ? "italic 620" : "560"} ${size}px "Archivo Variable", sans-serif`;
      return context.measureText(text).width;
    }
    return text.length * size * 0.58;
  };
}
