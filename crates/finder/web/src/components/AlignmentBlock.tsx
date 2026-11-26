// The finder alignment moves nothing: the observer turns the ring screws (050).
import { useState } from "react";
import { deleteImagingAlign, postImagingAlign, type ImagingAlignCentre, type ImagingView } from "../api";

export function centreLine(centre: ImagingAlignCentre, done: boolean): string {
  const place = `The imaging camera points ${Math.abs(centre.right_deg).toFixed(1)}° ${centre.right_deg < 0 ? "left" : "right"} and ${Math.abs(centre.up_deg).toFixed(1)}° ${centre.up_deg < 0 ? "down" : "up"} of the finder camera's centre, as the finder camera sees it: ${centre.separation_deg.toFixed(1)}° in all`;
  if (done) {
    return `Done: the imaging camera's centre has held in the middle of the finder camera's frame. ${place}.`;
  }
  if (centre.middle) {
    return `${place}, in the middle of the finder camera's frame. Done once the next frame holds it there.`;
  }
  return `${place}, ${centre.on_frame ? "on" : "off"} the finder camera's frame.`;
}

export function aimLine(aim: string): string {
  return aim === "measured" || aim === "replaced"
    ? `The aim point is measured from this frame (${aim}).`
    : `The aim point is not measured from this frame (${aim}); a later imaging solve measures it.`;
}

function Drawing({ centre }: { centre: ImagingAlignCentre }) {
  const h = (160 * centre.finder_height) / centre.finder_width;
  const dx = ((centre.x - centre.finder_width / 2) * 160) / centre.finder_width;
  const dy = ((centre.y - centre.finder_height / 2) * h) / centre.finder_height;
  // Intersect the ray with the frame, preserving its direction off a corner.
  const t = Math.min(1, 80 / Math.abs(dx), h / 2 / Math.abs(dy));
  const x = 12 + 80 + t * dx;
  const y = 12 + h / 2 + t * dy;
  return (
    <svg
      role="img"
      aria-label={`The imaging camera's centre ${centre.on_frame ? "on" : "off"} the finder camera's frame`}
      viewBox={`0 0 184 ${h + 24}`}
      width="184"
      height={h + 24}
      className="max-w-full"
      fill="none"
      stroke="currentColor"
    >
      <rect x="12" y="12" width="160" height={h} />
      <rect x="52" y={12 + h / 4} width="80" height={h / 2} strokeDasharray="4 3" />
      <line x1="92" y1={12 + h / 2} x2={x} y2={y} />
      <circle cx={x} cy={y} r="4" fill={centre.on_frame ? "currentColor" : "none"} />
    </svg>
  );
}

export function AlignmentBlock({ view, refresh }: { view: ImagingView | null; refresh: () => Promise<void> | void }) {
  const [pending, setPending] = useState(false);
  const [refusal, setRefusal] = useState<string | null>(null);
  const align = view?.align;
  const running = align?.state === "running";
  const last = align?.last;
  let status: string | undefined;
  if (last?.outcome === "measured" && last.centre) {
    status = centreLine(last.centre, last.done);
  } else if (last?.outcome === "not_solved") {
    status = `Imaging frame ${last.frame_id} did not solve: ${last.error}.`;
    if ((last.unsolved ?? 0) >= 3) {
      status += ` ${last.unsolved} frames in a row have not solved: the imaging camera may be more than 10° from the finder camera's centre, the sky may hold too few stars, or the field height may be wrong.`;
    }
    status += " The next frame looks again.";
  } else if (last?.outcome === "no_live_solve") {
    status = `The finder camera had no live solve for imaging frame ${last.frame_id}.`;
  } else if (running) {
    status = "Taking the first imaging frame.";
  }

  const toggle = async () => {
    if (pending) return;
    setPending(true);
    setRefusal(null);
    try {
      await (running ? deleteImagingAlign() : postImagingAlign());
      await refresh();
    } catch (error) {
      setRefusal(error instanceof Error ? error.message : "The console did not answer. Try again.");
    } finally {
      setPending(false);
    }
  };

  return (
    <section aria-labelledby="finder-alignment-title" className="space-y-3">
      <h2 id="finder-alignment-title" className="section-title">
        Align the finder camera
      </h2>
      <p className="quiet">
        Imaging frames show where the imaging camera points from the finder camera's centre. Turn Track off, then turn
        the finder camera's ring screws until it says Done. Nothing moves the telescope.
      </p>
      {view ? (
        <button type="button" className="button button-primary" disabled={pending} onClick={() => void toggle()}>
          {running ? "Stop aligning" : "Start aligning"}
        </button>
      ) : (
        <p className="quiet">Reading the imaging camera.</p>
      )}
      {status && <p role="status">{status}</p>}
      {last?.aim && <p className="quiet">{aimLine(last.aim)}</p>}
      {last?.outcome === "measured" && last.centre && <Drawing centre={last.centre} />}
      {align?.ended === "failed" && <p role="alert">{`The finder alignment stopped: ${align.error}.`}</p>}
      {refusal && <p role="alert">{refusal}</p>}
    </section>
  );
}
