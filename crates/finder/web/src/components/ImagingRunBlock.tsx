// One place to start, follow and stop the console's imaging run (042 US9).
import { useEffect, useLayoutEffect, useState } from "react";
import { deleteImagingRun, postImagingRun, postImagingRunContinue, type ImagingRun, type ImagingView } from "../api";

function time(seconds: number): string {
  const whole = Math.max(0, Math.floor(seconds));
  return `${Math.floor(whole / 60)}:${String(whole % 60).padStart(2, "0")}`;
}

function positive(value: string): number | undefined {
  const number = Number(value);
  return Number.isFinite(number) && number > 0 ? number : undefined;
}

function endedLine(run: ImagingRun, ended: NonNullable<ImagingRun["ended"]>): string {
  const count = run.subs.stacked;
  if (count === 0) return `${ended.message} No sub was stacked.`;
  if (!ended.saved) {
    return `${ended.message} ${count === 1 ? "1 sub was" : `${count} subs were`} stacked but the stack was not saved: the subs are in ${run.folder}/subs.`;
  }
  if (ended.reason === "count_limit") return `${ended.message} Saved on the console in ${run.folder}.`;
  return `${ended.message} ${count} ${count === 1 ? "sub" : "subs"} stacked · saved on the console in ${run.folder}.`;
}

export function ImagingRunBlock({ view, refresh }: { view: ImagingView; refresh: () => Promise<void> | void }) {
  const run = view.run;
  // These are drafts, independent of the new view useImaging reads each second.
  const [subs, setSubs] = useState("");
  const [minutes, setMinutes] = useState("");
  const [keepSubs, setKeepSubs] = useState(true);
  const [pending, setPending] = useState<"start" | "check" | "anyway" | "stop" | null>(null);
  const [refusal, setRefusal] = useState<string | null>(null);
  const [confirmStop, setConfirmStop] = useState(false);

  useLayoutEffect(() => {
    setRefusal(null);
    setConfirmStop(false);
  }, [run?.id, run?.state]);

  useEffect(() => {
    if (!confirmStop) return;
    const timer = setTimeout(() => setConfirmStop(false), 5000);
    return () => clearTimeout(timer);
  }, [confirmStop]);

  const request = async (action: NonNullable<typeof pending>, call: () => Promise<ImagingView>) => {
    if (pending !== null) return;
    setPending(action);
    setRefusal(null);
    setConfirmStop(false);
    try {
      await call();
    } catch (error) {
      setRefusal(
        error instanceof Error
          ? error.message
          : "The console did not answer. The run itself is not affected; try again.",
      );
      return;
    } finally {
      setPending(null);
    }
    await refresh();
  };

  const start = () => {
    const count = positive(subs);
    const duration = positive(minutes);
    void request("start", () =>
      postImagingRun({
        ...(count !== undefined && Number.isInteger(count) ? { subs: count } : {}),
        ...(duration !== undefined ? { duration_s: duration * 60 } : {}),
        keep_subs: keepSubs,
      }),
    );
  };

  if (view.state !== "ready" && !run) return null;
  const active = run !== undefined && run.state !== "ended";
  const ended = run?.ended;
  const endedAlert =
    run &&
    ended &&
    (!["stopped", "count_limit", "duration_limit"].includes(ended.reason) || (!ended.saved && run.subs.stacked > 0));
  const disabled = pending !== null;
  const durationMinutes = positive(minutes) ?? 120;
  const size =
    view.camera && view.settings
      ? (((view.camera.width * view.camera.height) / view.settings.binning ** 2) *
          2 *
          Math.floor((durationMinutes * 60000) / view.settings.exposure_ms)) /
        1e9
      : undefined;
  const dropped = run ? Object.values(run.subs.dropped).reduce((total, count) => total + count, 0) : 0;
  const reasons = run
    ? Object.entries(run.subs.dropped)
        .map(([reason, count]) => `${reason.replaceAll("_", " ")} ${count}`)
        .join(", ")
    : "";
  const warnings = run?.warnings?.map((warning, index) => (
    <p key={index} className="quiet">
      {warning}
    </p>
  ));

  return (
    <section aria-labelledby="imaging-run-title" className="space-y-3">
      <h2 id="imaging-run-title" className="section-title">
        Imaging run
      </h2>
      {run && !active && ended && (
        <>
          <p role={endedAlert ? "alert" : "status"}>{endedLine(run, ended)}</p>
          {ended.track_on && <p className="quiet">Track is still on: the top bar's Stop stops the mount.</p>}
          {warnings}
        </>
      )}
      {active ? (
        <>
          <p role={run.state === "focus_pause" ? "alert" : "status"}>{run.message}</p>
          <p>{`${run.subs.stacked} stacked · ${dropped} dropped${dropped > 0 ? ` (${reasons})` : ""}`}</p>
          <p>{`Elapsed ${time(run.elapsed_s)}`}</p>
          {run.stacking_since !== undefined && (
            <p>{`Stacking for ${time(run.started_at + run.elapsed_s - run.stacking_since)} of ${time(run.settings.duration_s)}`}</p>
          )}
          {run.state === "focus_pause" && run.pause_ends_at !== undefined && (
            <p>{`Focus aid open below · ${time(run.pause_ends_at - (run.started_at + run.elapsed_s))} left`}</p>
          )}
          {run.by === "mcp" && <p className="min-w-0 break-words text-sm text-ink-2">Started by an MCP client</p>}
          {warnings}
        </>
      ) : (
        <>
          {!run && view.unfinished_run && (
            <p role="alert">{`The last imaging run did not finish: the console stopped while it ran. Its subs are in ${view.unfinished_run}/subs.`}</p>
          )}
          <p className="quiet">
            {`Goes to the target, checks focus, centres it on the imaging camera's sensor and stacks subs until you stop it or a limit is reached (two hours unless you give another).`}
          </p>
          <div className="grid grid-cols-2 gap-3">
            <NumberField label="Subs to stack" value={subs} setValue={setSubs} />
            <NumberField label="Minutes to stack" value={minutes} setValue={setMinutes} placeholder="120" />
          </div>
          <label className="flex min-h-11 items-center gap-3">
            <input
              type="checkbox"
              checked={keepSubs}
              onChange={(event) => setKeepSubs(event.target.checked)}
              className="appearance-none size-5 border border-ink checked:bg-ink"
            />
            Keep every sub
          </label>
          {size !== undefined && (
            <p className="quiet">{`About ${size.toFixed(1)} GB of subs in ${durationMinutes} minutes at the current exposure.`}</p>
          )}
          <button type="button" className="button button-primary" disabled={disabled} onClick={start}>
            {pending === "start" ? "Starting…" : "Start imaging run"}
          </button>
        </>
      )}
      {active && (
        <>
          {run.state === "focus_pause" && (
            <div className="flex flex-wrap gap-3">
              <button
                type="button"
                className="button button-primary"
                disabled={disabled}
                onClick={() => void request("check", () => postImagingRunContinue(false))}
              >
                {pending === "check" ? "Checking…" : "Check focus again"}
              </button>
              <button
                type="button"
                className="button"
                disabled={disabled}
                onClick={() => void request("anyway", () => postImagingRunContinue(true))}
              >
                Go anyway
              </button>
            </div>
          )}
          <div className="flex flex-wrap gap-3">
            <button
              type="button"
              className="button"
              disabled={disabled || run.message.startsWith("Saving the stack")}
              onClick={() => {
                if (confirmStop) void request("stop", deleteImagingRun);
                else setConfirmStop(true);
              }}
            >
              {pending === "stop" ? "Stopping…" : confirmStop ? "Tap again to stop the run" : "Stop imaging run"}
            </button>
          </div>
        </>
      )}
      {refusal && <p role="alert">{refusal}</p>}
    </section>
  );
}

function NumberField({
  label,
  value,
  setValue,
  placeholder,
}: {
  label: string;
  value: string;
  setValue: (value: string) => void;
  placeholder?: string;
}) {
  return (
    <label className="block space-y-1">
      <span className="text-sm font-semibold text-ink-2">{label}</span>
      <input
        type="number"
        inputMode="decimal"
        step="any"
        value={value}
        placeholder={placeholder}
        onChange={(event) => setValue(event.target.value)}
        className="h-11 w-full min-w-0 border border-ink bg-transparent px-3 text-base outline-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink"
      />
    </label>
  );
}
