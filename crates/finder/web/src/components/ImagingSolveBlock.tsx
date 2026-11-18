// The Imaging solve block (032): the button that solves the latest saved imaging
// frame, what the solve said, and what the console keeps between the two cameras.
import { useState } from "react";
import { postImagingSolve, type ImagingSolve, type ImagingView } from "../api";

// The outcomes that took no aim reading (032 US6-S3), each with its own words.
const NO_AIM_WORDS: Record<string, string> = {
  moving: "the telescope was moving.",
  no_live_pointing: "no live pointing shared its moment.",
  off_frame: "its centre is outside the finder camera's frame.",
  set_aside: "it disagreed with the earlier ones and was set aside.",
};

function solveLine(solve: ImagingSolve): string | null {
  if (solve.state === "solving") return `Solving imaging frame ${solve.frame_id}`;
  if (solve.state === "failed") return null;
  let line =
    `Solved imaging frame ${solve.frame_id}: RA ${solve.ra_deg?.toFixed(4) ?? ""}° ` +
    `Dec ${solve.dec_deg?.toFixed(4) ?? ""}° · up ${solve.up_east_of_north_deg?.toFixed(1) ?? ""}° east of north · ` +
    `${solve.arcsec_per_px?.toFixed(2) ?? ""}″ per pixel`;
  if (solve.stars != null) line += ` · ${solve.stars} stars`;
  if (solve.mirrored) line += " · mirrored";
  return line;
}

export function ImagingSolveBlock({ view, refresh }: { view: ImagingView; refresh: () => Promise<void> | void }) {
  const [alert, setAlert] = useState<string | null>(null);
  const saved = view.frame?.state === "saved";
  const busy = view.solve?.state === "solving";
  const line = view.solve ? solveLine(view.solve) : null;
  const relation = view.relation;

  const solve = async () => {
    setAlert(null);
    try {
      await postImagingSolve();
    } catch (e) {
      setAlert(e instanceof Error ? e.message : "The imaging frame could not be solved.");
      return;
    }
    await refresh();
  };

  return (
    <section aria-labelledby="imaging-solve-title" className="space-y-3">
      <h2 id="imaging-solve-title" className="section-title">
        Imaging solve
      </h2>
      <div className="space-y-3 border-y border-rule py-4">
        <button type="button" className="button" disabled={!saved || busy} onClick={() => void solve()}>
          Solve imaging frame
        </button>
        {line != null && view.solve?.state !== "failed" && <p role="status">{line}</p>}
        {view.solve?.state === "failed" && <p role="alert">{`Imaging solve failed: ${view.solve.error ?? ""}.`}</p>}
        {alert && <p role="alert">{alert}</p>}
        {relation?.scale && (
          <p className="quiet">{`Scale ${relation.scale.arcsec_per_px.toFixed(3)}″ per sensor pixel, from imaging frame ${relation.scale.frame_id}.`}</p>
        )}
        {relation?.aim && relation.angle_deg != null && relation.readings != null && (
          <p className="quiet">{`Aim point measured at (${relation.aim.x.toFixed(1)}, ${relation.aim.y.toFixed(1)}) px of the finder camera's frame · imaging angle ${relation.angle_deg.toFixed(1)}° · from ${relation.readings} imaging solves.`}</p>
        )}
        {relation?.last && NO_AIM_WORDS[relation.last.outcome] && (
          <p className="quiet">{`The last imaging solve did not measure the aim point: ${NO_AIM_WORDS[relation.last.outcome]}`}</p>
        )}
        {relation?.held && <p className="quiet">The measured aim point waits until Track is off.</p>}
      </div>
    </section>
  );
}
