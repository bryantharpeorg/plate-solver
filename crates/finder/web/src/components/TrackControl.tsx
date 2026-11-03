import { useEffect, useState } from "react";
import { postTrack, type ObservingSession } from "../api";

const END_REASON: Record<string, string> = {
  stopped: "stopped by the observer",
  link_lost: "the scope link was lost",
  sun_up: "the Sun is up",
  too_low: "the target is too low",
};

export function TrackControl({ session, onChange }: { session: ObservingSession; onChange?: () => void }) {
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { track } = session;
  useEffect(() => setError(null), [session.target?.id, track?.state, track?.ended_reason]);
  if (!session.mount?.connected) return null;
  const on = track?.wanted === true && track.state !== "off";
  const readout = on ? track?.readout : undefined;
  const exposure = session.imaging_camera ? readout?.longest_exposure_s : undefined;
  const target = track?.target ?? session.target;
  const change = async () => {
    setPending(true);
    setError(null);
    try {
      await postTrack(!on);
      onChange?.();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Track could not be changed. Try again.");
    } finally {
      setPending(false);
    }
  };
  return (
    <section aria-label="Track" className="space-y-2">
      <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
        <label className="flex min-h-11 items-center gap-3 font-semibold">
          Track
          <button
            type="button"
            role="switch"
            aria-label="Track"
            aria-checked={on}
            className="switch"
            disabled={pending}
            onClick={() => void change()}
          />
        </label>
        {on && (
          <p role="status">
            {track?.state === "armed"
              ? "Track armed · Waiting for GoTo"
              : track?.state === "keyhole_wait"
                ? "Waiting at the keyhole"
                : "Tracking"}{" "}
            <span className="italic">{target?.messier ?? target?.id}</span>
          </p>
        )}
      </div>
      {readout && (
        <dl className="flex flex-wrap gap-x-6 gap-y-2 text-sm">
          <div>
            <dt className="text-ink-2">Tracking error (RMS, last minute)</dt>
            <dd>{Math.round(readout.rms_arcsec)}″</dd>
          </div>
          <div>
            <dt className="text-ink-2">Field rotation</dt>
            <dd>{readout.field_rotation_deg_per_min.toFixed(2)}°/min</dd>
          </div>
          {exposure !== undefined && (
            <div>
              <dt className="text-ink-2">Longest exposure (1 px smear)</dt>
              <dd>{exposure === "no limit" ? exposure : `${exposure.toFixed(1)} s`}</dd>
            </div>
          )}
        </dl>
      )}
      {!on && track?.ended_reason && (
        <p role="status">Tracking ended: {END_REASON[track.ended_reason] ?? track.ended_reason}.</p>
      )}
      {error && <p role="alert">{error}</p>}
    </section>
  );
}
