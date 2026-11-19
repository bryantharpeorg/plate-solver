// The Imaging page's calibration library and controls for darks and flats (034).
import { useState } from "react";
import {
  deleteCalibration,
  postCalibrationDarks,
  postCalibrationFlatDarks,
  postCalibrationFlats,
  type CalibrationMaster,
  type CalibrationSet,
  type ImagingCalibration,
} from "../api";

function masterLine(master: CalibrationMaster): string {
  const parts = [master.kind === "dark" ? "Master dark" : `Master flat ${master.label}`];
  if (master.kind === "dark") parts.push(`${String(master.exposure_ms)} ms`);
  if (master.gain != null) parts.push(`gain ${String(master.gain)}`);
  if (master.offset != null) parts.push(`offset ${String(master.offset)}`);
  parts.push(`${String(master.binning)} × ${String(master.binning)}`);
  if (master.set_temp_c != null) parts.push(`set ${master.set_temp_c.toFixed(1)} °C`);
  else if (master.temp_c != null) parts.push(`sensor ${master.temp_c.toFixed(1)} °C`);
  parts.push(`${String(master.frames)} frames`);
  if (master.kind === "dark") {
    parts.push(master.bad_pixels != null ? `${String(master.bad_pixels)} bad pixels` : "no bad-pixel map");
  }
  parts.push(new Date(master.made_at * 1000).toISOString().slice(0, 10));
  if (master.kind === "flat" && master.imaging_angle_deg != null) {
    parts.push(`camera at ${master.imaging_angle_deg.toFixed(1)}°`);
  }
  if (!master.checked) parts.push("dark test skipped");
  return parts.join(" · ");
}

function setStatus(set: CalibrationSet): string | null {
  const kind = set.kind === "dark" ? "darks" : "flats";
  switch (set.state) {
    case "offset":
      return "Taking darks: reading the offset level";
    case "search":
      return "Taking flats: finding the exposure";
    case "frames":
      return `Taking ${kind}: ${String(set.taken)} of ${String(set.wanted)} at ${String(set.exposure_ms)} ms`;
    case "needs_darks":
      return `Flats in hand: ${String(set.wanted)} at ${String(set.exposure_ms)} ms. Cover the telescope, then take their darks.`;
    case "flat_darks":
      return `Taking the flats' darks: ${String(set.darks_taken ?? 0)} of ${String(set.wanted)}`;
    case "done":
      return set.kind === "dark"
        ? `Master dark saved from ${String(set.wanted)} darks.`
        : `Master flat ${set.label} saved from ${String(set.wanted)} flats.`;
    case "stopped":
      return set.kind === "dark" ? "Darks stopped." : "Flats stopped.";
    case "failed":
      return null;
  }
}

export function CalibrationBlock({ calibration, onChange }: { calibration: ImagingCalibration; onChange: () => void }) {
  // Drafts belong to the observer, so the one-second imaging reads leave them alone.
  const [darkCount, setDarkCount] = useState("20");
  const [flatCount, setFlatCount] = useState("20");
  const [label, setLabel] = useState("");
  const [pending, setPending] = useState(false);
  const [refusal, setRefusal] = useState<string | null>(null);
  const { masters, match, set } = calibration;
  const flatLabel = masters.find((master) => master.id === match?.flat)?.label;
  const running = set != null && ["offset", "frames", "search", "flat_darks"].includes(set.state);
  const waiting = set?.kind === "flat" && set.state === "needs_darks";
  const busy = pending || running || waiting;
  const status = set ? setStatus(set) : null;
  const allowAnyway = set?.not_dark && ((set.kind === "dark" && set.state === "failed") || waiting);

  const send = async (request: () => Promise<ImagingCalibration>) => {
    setPending(true);
    setRefusal(null);
    try {
      await request();
    } catch (e) {
      setRefusal(e instanceof Error ? e.message : "The calibration library could not be reached.");
      return;
    } finally {
      setPending(false);
    }
    onChange();
  };

  return (
    <section aria-labelledby="calibration-title" className="space-y-3">
      <h2 id="calibration-title" className="section-title">
        Calibration
      </h2>
      {match ? (
        <p>{`Imaging frame ${String(match.frame)} would get: ${match.dark ? "its master dark" : "no master dark"} · ${match.flat ? `the master flat ${flatLabel}` : "no master flat"} · ${match.bad_pixels != null ? `a bad-pixel map of ${String(match.bad_pixels)}` : "hot pixels found in the sub itself"}`}</p>
      ) : (
        <p className="quiet">Take an imaging frame to see which masters match it.</p>
      )}
      {match?.flat_turned_deg != null && (
        <p role="alert">{`The camera has turned ${match.flat_turned_deg.toFixed(1)}° since the flats ${flatLabel} were taken. Take new flats.`}</p>
      )}
      {calibration.error && <p role="alert">{`The calibration library is unreadable: ${calibration.error}.`}</p>}
      {masters.length === 0 ? (
        <p className="quiet">No master darks or flats yet.</p>
      ) : (
        <ul>
          {masters.map((master) => (
            <li key={master.id} className="flex flex-wrap items-center justify-between gap-3 border-b border-rule py-3">
              <p>{masterLine(master)}</p>
              <button
                type="button"
                className="button"
                aria-label={`Remove ${master.id}`}
                disabled={pending}
                onClick={() => void send(() => deleteCalibration(master.id))}
              >
                Remove
              </button>
            </li>
          ))}
        </ul>
      )}
      <form
        className="space-y-3"
        onSubmit={(e) => {
          e.preventDefault();
          if (!busy) void send(() => postCalibrationDarks(Number(darkCount)));
        }}
      >
        <div className="flex flex-wrap items-end gap-3">
          <CountField name="Dark frames" value={darkCount} onChange={setDarkCount} />
          <button type="submit" className="button button-primary" disabled={busy}>
            Take darks
          </button>
        </div>
        <p className="quiet">
          {"Cover the telescope first. Darks use the exposure, gain, offset, binning and set temperature in force."}
        </p>
      </form>
      <form
        className="space-y-3"
        onSubmit={(e) => {
          e.preventDefault();
          if (!busy && label !== "") void send(() => postCalibrationFlats(Number(flatCount), label));
        }}
      >
        <div className="flex flex-wrap items-end gap-3">
          <CountField name="Flat frames" value={flatCount} onChange={setFlatCount} />
          <label className="block space-y-1">
            <span className="text-sm font-semibold text-ink-2">Flat label</span>
            <input
              type="text"
              value={label}
              onChange={(e) => setLabel(e.target.value)}
              className="h-11 w-full min-w-0 border border-ink bg-transparent px-3 text-base outline-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink"
            />
          </label>
          <button type="submit" className="button button-primary" disabled={busy || label === ""}>
            Take flats
          </button>
        </div>
        <p className="quiet">
          {
            "Light the flat panel first. The console finds the exposure; the flats' darks follow with the telescope covered."
          }
        </p>
      </form>
      {set && (
        <div className="space-y-3">
          {status && <p role="status">{status}</p>}
          {set.state === "failed" && (
            <p role="alert">{`${set.kind === "dark" ? "Darks" : "Flats"} stopped: ${set.error}.`}</p>
          )}
          {waiting && set.error && <p role="alert">{`The flats' darks stopped: ${set.error}.`}</p>}
          {running && (
            <button
              type="button"
              className="button"
              disabled={pending}
              onClick={() => void send(() => deleteCalibration(set.id))}
            >
              Stop
            </button>
          )}
          {waiting && (
            <div className="flex flex-wrap gap-3">
              <button
                type="button"
                className="button button-primary"
                disabled={pending}
                onClick={() => void send(() => postCalibrationFlatDarks())}
              >
                Take the flats' darks
              </button>
              <button
                type="button"
                className="button"
                disabled={pending}
                onClick={() => void send(() => deleteCalibration(set.id))}
              >
                Drop these flats
              </button>
            </div>
          )}
          {allowAnyway && (
            <>
              <p className="quiet">
                {"Only if the telescope is covered: a warm sensor or a long exposure can read above the offset level."}
              </p>
              {waiting ? (
                <button
                  type="button"
                  className="button"
                  disabled={pending}
                  onClick={() => void send(() => postCalibrationFlatDarks(true))}
                >
                  Take the flats' darks anyway
                </button>
              ) : (
                <button
                  type="button"
                  className="button"
                  disabled={pending}
                  onClick={() => void send(() => postCalibrationDarks(set.wanted, true))}
                >
                  Take them anyway
                </button>
              )}
            </>
          )}
        </div>
      )}
      {refusal && <p role="alert">{refusal}</p>}
    </section>
  );
}

function CountField({ name, value, onChange }: { name: string; value: string; onChange: (value: string) => void }) {
  return (
    <label className="block space-y-1">
      <span className="text-sm font-semibold text-ink-2">{name}</span>
      <input
        type="number"
        min="3"
        max="64"
        step="1"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="h-11 w-full min-w-0 border border-ink bg-transparent px-3 text-base outline-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink"
      />
    </label>
  );
}
