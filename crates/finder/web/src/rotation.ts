// The live views' rotation: one angle from the session, applied alike to the live chart,
// the live photo and the aim-point photo. Display only: marks, taps and everything sent
// to the server stay in the frame's own pixels.

import type { ObservingSession } from "./api";

/** The rotation in degrees counter-clockwise, from the session; 0 when unset. */
export function rotationOf(session: ObservingSession | null | undefined): number {
  const deg = session?.chart_rotation_deg;
  if (deg === undefined || !Number.isFinite(deg)) return 0;
  return ((deg % 360) + 360) % 360;
}

/** Sine and cosine of `deg`, exact at quarter turns so a frame on its side keeps whole pixels. */
export function sinCos(deg: number): [number, number] {
  const quarter = (((deg % 360) + 360) % 360) / 90;
  if (Number.isInteger(quarter))
    return (
      [
        [0, 1],
        [1, 0],
        [0, -1],
        [-1, 0],
      ] as const
    )[quarter] as [number, number];
  const a = (deg * Math.PI) / 180;
  return [Math.sin(a), Math.cos(a)];
}

/** The bounding box of a W×H frame rotated by `deg`. */
export function rotatedBox(W: number, H: number, deg: number): { width: number; height: number } {
  const [sin, cos] = sinCos(deg);
  return { width: W * Math.abs(cos) + H * Math.abs(sin), height: W * Math.abs(sin) + H * Math.abs(cos) };
}

/** The SVG transform that draws frame pixels rotated in the box, or undefined at 0. */
export function rotationTransform(W: number, H: number, deg: number): string | undefined {
  if (deg === 0) return undefined;
  const b = rotatedBox(W, H, deg);
  return `translate(${b.width / 2} ${b.height / 2}) rotate(${-deg}) translate(${-W / 2} ${-H / 2})`;
}

/** Where frame pixel (x, y) is drawn in the rotated box, in frame pixels from its top left. */
export function toView(W: number, H: number, deg: number, x: number, y: number): [number, number] {
  const b = rotatedBox(W, H, deg);
  const [sin, cos] = sinCos(deg);
  const [dx, dy] = [x - W / 2, y - H / 2];
  return [dx * cos + dy * sin + b.width / 2, -dx * sin + dy * cos + b.height / 2];
}

/** The frame pixel drawn at (x, y) of the rotated box: `toView`'s inverse. */
export function fromView(W: number, H: number, deg: number, x: number, y: number): [number, number] {
  const b = rotatedBox(W, H, deg);
  const [sin, cos] = sinCos(deg);
  const [dx, dy] = [x - b.width / 2, y - b.height / 2];
  return [dx * cos - dy * sin + W / 2, dx * sin + dy * cos + H / 2];
}

/** The frame pixel under CSS point (x, y) of a photo drawn rotated in `box`. */
export function boxToFrame(
  W: number,
  H: number,
  deg: number,
  box: { width: number; height: number },
  x: number,
  y: number,
): [number, number] {
  // Unrotated, this is the plain scaling the tap has always used, to the last bit.
  if (deg === 0) return [(x * W) / box.width, (y * H) / box.height];
  const b = rotatedBox(W, H, deg);
  return fromView(W, H, deg, (x * b.width) / box.width, (y * b.height) / box.height);
}
