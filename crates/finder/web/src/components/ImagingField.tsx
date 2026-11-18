// The imaging camera's field on the live chart (032): a thin ink rectangle
// centred on the measured aim point, turned by the imaging angle, with a tick
// on its up side. It is drawn like the chart's other furniture.

/** The imaging frame's corners in the finder frame's pixels: top-left, top-right,
 *  bottom-right, bottom-left. Null when the finder scale is not above 0. */
export function imagingFieldCorners(
  aim: { x: number; y: number },
  angleDeg: number,
  mirrored: boolean,
  field: { width_arcsec: number; height_arcsec: number },
  finderArcsecPerPx: number,
): [number, number][] | null {
  if (!(finderArcsecPerPx > 0)) return null;
  const a = (angleDeg * Math.PI) / 180;
  // The imaging frame's up and right as seen in the finder frame's pixels; a
  // positive angle turns the imaging frame's up toward the finder frame's left.
  const up = { x: -Math.sin(a), y: -Math.cos(a) };
  const right = mirrored ? { x: -Math.cos(a), y: Math.sin(a) } : { x: Math.cos(a), y: -Math.sin(a) };
  const hw = field.width_arcsec / (2 * finderArcsecPerPx);
  const hh = field.height_arcsec / (2 * finderArcsecPerPx);
  const corner = (alongRight: number, alongUp: number): [number, number] => [
    aim.x + alongRight * hw * right.x + alongUp * hh * up.x,
    aim.y + alongRight * hw * right.y + alongUp * hh * up.y,
  ];
  return [corner(-1, 1), corner(1, 1), corner(1, -1), corner(-1, -1)];
}

/** Four corners as one path: M, then L for each corner after the first. */
function toPath(corners: [number, number][]): string {
  return corners.map(([x, y], i) => `${i ? "L" : "M"}${x.toFixed(1)} ${y.toFixed(1)}`).join("");
}

export function ImagingField({ corners, u }: { corners: [number, number][]; u: number }) {
  let d = `${toPath(corners)}Z`;
  // A tick on the up side: from the middle of the top edge away from the centre.
  const middle = {
    x: (corners[0]![0] + corners[1]![0]) / 2,
    y: (corners[0]![1] + corners[1]![1]) / 2,
  };
  const centre = {
    x: (corners[0]![0] + corners[1]![0] + corners[2]![0] + corners[3]![0]) / 4,
    y: (corners[0]![1] + corners[1]![1] + corners[2]![1] + corners[3]![1]) / 4,
  };
  const away = { x: middle.x - centre.x, y: middle.y - centre.y };
  const length = Math.hypot(away.x, away.y);
  if (length > 0) {
    const end = { x: middle.x + (6 * u * away.x) / length, y: middle.y + (6 * u * away.y) / length };
    d += `M${middle.x.toFixed(1)} ${middle.y.toFixed(1)}L${end.x.toFixed(1)} ${end.y.toFixed(1)}`;
  }
  return (
    <g data-imaging-field className="furniture pointer-events-none" strokeWidth={1.3 * u}>
      <title>Imaging field</title>
      <path fill="none" d={d} />
    </g>
  );
}
