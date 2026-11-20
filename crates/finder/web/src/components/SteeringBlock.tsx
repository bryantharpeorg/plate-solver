// What steers the tracker, and the run-only settings for tracking from subs (040).
import { useLayoutEffect, useState } from "react";
import { putImagingSteering, type ImagingSteering, type ImagingView } from "../api";

function Status({ s }: { s: ImagingSteering }) {
  if (s.waiting) {
    switch (s.pause) {
      case "starting":
        return <p role="status">Exposures paused: tracking is starting</p>;
      case "reversing":
        return <p role="status">Exposures paused for a moment: the mount is reversing</p>;
      case "measuring":
        return <p role="status">Exposures paused: the mount is measuring its gear play, once, for a few minutes</p>;
      case "overhead":
        return (
          <p role="status">
            Exposures paused: the target is too near overhead to follow. They resume when it has passed
          </p>
        );
      case "pushed":
        return <p role="status">Exposures paused: the telescope is being moved by hand</p>;
      case "stalled":
        return <p role="status">Exposures paused for a moment: the mount stopped and is being started again</p>;
      case "limit":
        if (s.exposure_s !== undefined)
          return (
            <p role="status">{`Exposures paused: corner stars would trail. They resume when the longest exposure for round stars reaches ${(s.exposure_s / 2).toFixed(1)} s`}</p>
          );
    }
    return <p role="status">Exposures paused: the tracker is not steady</p>;
  }
  if (s.source === "subs") return <p role="status">{`Steered by the subs · ${s.subs} used`}</p>;
  if (s.source === "tracker") {
    if (s.far_arcsec !== undefined)
      return (
        <p role="status">{`The target is ${(s.far_arcsec / 60).toFixed(1)}′ from the set point: the tracker steers on its own. Centre it, or turn Track off and on to apply the measured aim point.`}</p>
      );
    if (!s.holding) return <p role="status">Steered by the tracker's own corrections</p>;
    if (!s.on) return <p role="status">The switch is off: the tracker's own corrections steer</p>;
    return (
      <p role="status">
        No sub accepted lately: the tracker's own corrections steer until one is. See Dropped in the stack.
      </p>
    );
  }
  return <p role="status">Not tracking: nothing steers</p>;
}

export function SteeringBlock({ view, onChange }: { view: ImagingView; onChange?: () => void }) {
  const s = view.steering;
  const present = s !== undefined;
  const smear = s?.smear_px ?? 6;
  const enabled = s?.on ?? true;
  const step = s?.step_share ?? 0.5;
  const rate = s?.rate_share ?? 0.3;
  const [smearPx, setSmearPx] = useState(String(smear));
  const [on, setOn] = useState(enabled);
  const [stepShare, setStepShare] = useState(String(step));
  const [rateShare, setRateShare] = useState(String(rate));
  const [pending, setPending] = useState(false);
  const [refusal, setRefusal] = useState<string | null>(null);

  // Polls with the same settings preserve drafts. Presence also matters: the
  // first steering view seeds them even when its values equal the defaults.
  useLayoutEffect(() => {
    setSmearPx(String(smear));
    setOn(enabled);
    setStepShare(String(step));
    setRateShare(String(rate));
  }, [present, smear, enabled, step, rate]);

  const save = async () => {
    setPending(true);
    setRefusal(null);
    try {
      await putImagingSteering({
        on,
        smear_px: Number(smearPx),
        step_share: Number(stepShare),
        rate_share: Number(rateShare),
      });
    } catch (error) {
      setRefusal(error instanceof Error ? error.message : "The tracking settings could not be saved.");
      return;
    } finally {
      setPending(false);
    }
    onChange?.();
  };

  return (
    <section aria-labelledby="steering-title" className="space-y-3">
      <h2 id="steering-title" className="section-title">
        Tracking from the subs
      </h2>
      {s && <Status s={s} />}
      {s && (
        <dl className="flex flex-wrap gap-x-6 gap-y-2 text-sm">
          {s.error_rms_arcsec !== undefined && s.error_rms_px !== undefined && (
            <div>
              <dt className="text-ink-2">Tracking error from the subs (RMS, last 10)</dt>
              <dd>{`${s.error_rms_arcsec.toFixed(1)}″ · ${s.error_rms_px.toFixed(1)} px`}</dd>
            </div>
          )}
          {s.drift_arcsec !== undefined && s.drift_px !== undefined && (
            <div>
              <dt className="text-ink-2">Drift per sub</dt>
              <dd>{`${s.drift_arcsec.toFixed(1)}″ · ${s.drift_px.toFixed(1)} px`}</dd>
            </div>
          )}
          {s.exposure_s !== undefined && (
            <div>
              <dt className="text-ink-2">Exposure</dt>
              <dd>{`${s.exposure_s.toFixed(1)} s${s.limited ? ", shortened to keep corner stars round" : ""}`}</dd>
            </div>
          )}
          {s.longest_exposure_s !== undefined && (
            <div>
              <dt className="text-ink-2">{`Longest exposure (${s.smear_px} px corner smear)`}</dt>
              <dd>{s.longest_exposure_s === "no limit" ? "none" : `${s.longest_exposure_s.toFixed(1)} s`}</dd>
            </div>
          )}
          {s.would_take_s !== undefined && (
            <div>
              <dt className="text-ink-2">A stack started now would take</dt>
              <dd>{`${s.would_take_s.toFixed(1)} s`}</dd>
            </div>
          )}
        </dl>
      )}
      {s?.over_limit && typeof s.longest_exposure_s === "number" && s.exposure_s !== undefined && (
        <p role="alert">{`Corner stars are trailing: the longest exposure for round stars is now ${s.longest_exposure_s.toFixed(1)} s, and this stack takes ${s.exposure_s.toFixed(1)} s. It keeps running. To shorten the exposure, stop it and start a new stack.`}</p>
      )}
      {s?.source === "none" && s.holding && (
        <p role="alert">Nothing is tracking, and the stack is still exposing. Turn Track on or stop the stack.</p>
      )}
      <form
        className="space-y-3 border-y border-rule py-4"
        onSubmit={(event) => {
          event.preventDefault();
          void save();
        }}
      >
        <NumberField label="Corner smear (px)" value={smearPx} setValue={setSmearPx} min={0.1} max={20} />
        <label className="flex min-h-11 items-center gap-3">
          <input
            type="checkbox"
            checked={on}
            onChange={(event) => setOn(event.target.checked)}
            className="appearance-none size-5 border border-ink checked:bg-ink"
          />
          Steer the tracker from the subs
        </label>
        <p className="quiet">{`The corner smear is used when a stack starts. After a restart of the console it is 6 px again and the switch is on.`}</p>
        <h3 className="font-semibold">Tuning</h3>
        <div className="grid grid-cols-2 gap-3">
          <NumberField label="Step share" value={stepShare} setValue={setStepShare} min={0} max={1} />
          <NumberField label="Rate share" value={rateShare} setValue={setRateShare} min={0} max={1} />
        </div>
        <p className="quiet">{`For tuning in the field: the share of a tracking error one gap steps out, and of a drift it takes out of the rate. After a restart of the console they are 0.5 and 0.3 again.`}</p>
        <button type="button" className="button" disabled={pending} onClick={() => void save()}>
          Save tracking settings
        </button>
        {refusal && <p role="alert">{refusal}</p>}
      </form>
    </section>
  );
}

function NumberField({
  label,
  value,
  setValue,
  min,
  max,
}: {
  label: string;
  value: string;
  setValue: (value: string) => void;
  min: number;
  max: number;
}) {
  return (
    <label className="block space-y-1">
      <span className="text-sm font-semibold text-ink-2">{label}</span>
      <input
        type="number"
        inputMode="decimal"
        step="any"
        min={min}
        max={max}
        value={value}
        onChange={(event) => setValue(event.target.value)}
        className="h-11 w-full min-w-0 border border-ink bg-transparent px-3 text-base outline-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink"
      />
    </label>
  );
}
