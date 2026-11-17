/** Occasional node photographs stay below the chart and take its night filter. */
import type { CSSProperties } from "react";
import { rotatedBox } from "../rotation";

/** How a photo of a W×H frame sits rotated inside a box of the rotated frame's aspect. */
export function rotatedPhoto(W: number, H: number, deg: number): { box: CSSProperties; photo: CSSProperties } {
  const b = rotatedBox(W, H, deg);
  return {
    box: { aspectRatio: `${b.width / b.height} / 1`, position: "relative", overflow: "hidden" },
    photo: {
      position: "absolute",
      top: "50%",
      left: "50%",
      width: `${(W / b.width) * 100}%`,
      transform: `translate(-50%, -50%) rotate(${-deg}deg)`,
    },
  };
}

export function LivePreview({
  seq,
  width,
  height,
  rotation,
}: {
  seq: number | null;
  width?: number;
  height?: number;
  /** The live views' rotation, degrees counter-clockwise; absent or 0 draws the photo as captured. */
  rotation?: number;
}) {
  if (seq === null) return null;
  // With a rotation other than 0 and both sizes known, the photo turns inside a box of
  // the turned frame's aspect; otherwise neither element has a style (FR-009).
  const turned =
    rotation != null && rotation !== 0 && width != null && height != null
      ? rotatedPhoto(width, height, rotation)
      : null;
  return (
    <div className="mt-4 border border-ink bg-photo" style={turned?.box}>
      <img
        src={`/api/live/preview.jpg?seq=${seq}`}
        alt={`Live preview, frame ${seq}`}
        className="photo block h-auto w-full"
        style={turned?.photo}
      />
    </div>
  );
}
