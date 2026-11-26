// Tools focuses either camera (051). The imaging camera keeps 033's FocusBlock;
// the finder camera reads its own aid beside the live solve.
import { useEffect, useState } from "react";
import { deleteFinderFocus, getFinderFocus, postFinderFocus, type FinderFocus, type ImagingView } from "../api";
import { FocusBlock } from "./FocusBlock";

const TREND_WORDS: Record<NonNullable<FinderFocus["trend"]>, string> = {
  better: "Sharper than before the last change",
  worse: "Softer than before the last change",
  same: "About the same as before the last change",
};

const size = (value: number) => `${value.toFixed(2)} px`;

export function finderStatus(focus: FinderFocus): string {
  const last = focus.last;
  if (last == null) return "Waiting for the first finder camera frame";
  switch (last.state) {
    case "reading":
      return focus.median?.readings === 3
        ? `Median of the last 3 finder camera frames · ${last.stars} stars`
        : `${focus.median?.readings ?? 0} of 3 finder camera frames since the last change · ${last.stars} stars`;
    case "settling":
      return "Settling after a change";
    case "too_few_stars":
      return last.detected < 5
        ? `No reading: ${last.stars} stars measured, and 5 are needed. Far out of focus the stars vanish: turn the focuser slowly until they show.`
        : `No reading: ${last.stars} of the ${last.detected} stars detected could be measured, and 5 are needed: the rest are saturated, crowded or at the frame's edge.`;
    case "far_out":
      return last.value != null ? `Far out of focus: stars about ${size(last.value)} across` : "Far out of focus";
  }
}

// 033's drawing: 36 units at the largest reading, a gap for null, currentColor
// throughout so night mode uses the page's own red ink.
function History({ values }: { values: (number | null)[] }) {
  const shown = values.slice(-60);
  const numbers = shown.filter((v): v is number => v != null);
  const max = numbers.length > 0 ? Math.max(...numbers) : null;
  return (
    <svg
      role="img"
      aria-label={`Focus readings of the last ${shown.length} finder camera frames`}
      viewBox={`0 0 ${shown.length * 4} 40`}
      width="100%"
      height="40"
      preserveAspectRatio="none"
    >
      {shown.map((v, i) => {
        if (v == null || max == null || max <= 0) return null;
        const h = (36 * v) / max;
        return <rect key={i} x={i * 4} width="3" height={h.toFixed(1)} y={(40 - h).toFixed(1)} fill="currentColor" />;
      })}
    </svg>
  );
}

function FinderAid() {
  // undefined is the first read still on its way; null is no run yet.
  const [focus, setFocus] = useState<FinderFocus | null | undefined>(undefined);
  const [pending, setPending] = useState(false);
  const [refusal, setRefusal] = useState<string | null>(null);
  useEffect(() => {
    let active = true;
    const read = async () => {
      try {
        const reply = await getFinderFocus();
        if (active) setFocus(reply.focus);
      } catch {
        // Keep the latest view when a read fails, as useImaging does.
      }
    };
    void read();
    const timer = setInterval(() => void read(), 1000);
    return () => {
      active = false;
      clearInterval(timer);
    };
  }, []);

  const running = focus?.state === "running";
  const toggle = async () => {
    if (pending) return;
    setPending(true);
    setRefusal(null);
    try {
      const reply = await (running ? deleteFinderFocus() : postFinderFocus());
      setFocus(reply.focus);
    } catch (error) {
      setRefusal(error instanceof Error ? error.message : "The finder camera's focus aid could not be reached.");
    } finally {
      setPending(false);
    }
  };

  if (focus === undefined) return <p className="quiet">Reading the finder camera's focus aid.</p>;
  return (
    <div className="space-y-3 border-y border-rule py-4">
      <p className="quiet">
        The finder camera's star size, in its pixels: smaller is sharper. It is read beside the live solve and changes
        no setting. Turn the focuser one way until the reading has passed its lowest and risen again, then come back to
        the lowest.
      </p>
      <button
        type="button"
        className={running ? "button" : "button button-primary"}
        disabled={pending}
        onClick={() => void toggle()}
      >
        {running ? "Stop the finder camera's focus aid" : "Start the finder camera's focus aid"}
      </button>
      {focus != null && (
        <>
          <p className="group-title">Focus reading</p>
          <p className="text-[2.35rem] font-[680] tracking-[-0.02em] sm:text-[2.75rem]">
            {focus.median != null ? size(focus.median.value) : "—"}
          </p>
          {running ? (
            <p role="status">{finderStatus(focus)}</p>
          ) : focus.ended === "stopped" ? (
            <p role="status">The finder camera's focus aid stopped.</p>
          ) : null}
          {focus.ended === "failed" && (
            <p role="alert">{`The finder camera's focus aid stopped: ${focus.error ?? ""}.`}</p>
          )}
          {focus.last != null && <p>{`Stars detected ${focus.last.detected}`}</p>}
          {focus.best != null && <p>{`Best so far ${size(focus.best)}`}</p>}
          {focus.trend != null && <p>{TREND_WORDS[focus.trend]}</p>}
          {focus.found && <p>Passed through focus and came back to the best so far.</p>}
          <History values={focus.history} />
        </>
      )}
      {refusal != null && <p role="alert">{refusal}</p>}
    </div>
  );
}

export function FocusTool({ view, refresh }: { view: ImagingView | null; refresh: () => Promise<void> | void }) {
  const [camera, setCamera] = useState("finder");
  return (
    <section aria-labelledby="focus-tool-title" className="space-y-3">
      <h2 id="focus-tool-title" className="section-title">
        Focus a camera
      </h2>
      <label className="block space-y-1">
        <span className="text-sm font-semibold text-ink-2">Camera</span>
        <select
          aria-label="Camera"
          value={camera}
          onChange={(e) => setCamera(e.target.value)}
          className="h-11 w-full min-w-0 border border-ink bg-transparent px-3 text-base outline-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink"
        >
          <option value="finder">Finder camera</option>
          <option value="imaging">Imaging camera</option>
        </select>
      </label>
      {camera === "finder" ? (
        <FinderAid />
      ) : view?.focus != null ? (
        <FocusBlock view={view} refresh={refresh} />
      ) : (
        <p className="quiet">The imaging camera's focus aid shows here once the imaging camera is ready.</p>
      )}
    </section>
  );
}
