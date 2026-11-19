// The Focus block (033 US6): the focus reading as one large number, the focus aid's
// controls, and its verdicts against this camera's own focus record, never a fixed
// number. Existing ink tokens and currentColor only, so it is red at night with
// everything else.
import { useState } from "react";
import {
  deleteImagingFocus,
  postImagingFocus,
  putImagingWholeSensor,
  type ImagingFocusAid,
  type ImagingFocusAsk,
  type ImagingView,
} from "../api";

const STANDING =
  "A focus reading is the size of the stars in the focus region: smaller is sharper. The console judges it " +
  "against this camera's own focus record and never against a fixed number, because seeing, collimation and a " +
  "warm mirror enlarge stars too. Use a field of stars away from any bright nebula.";
const PX_NOTE =
  "No imaging scale is known yet, so sizes are in sensor pixels. Solve an imaging frame, or start the console with --focal-length-mm.";
const SPARSE = "Try a longer Focus frame time, or a field with more stars.";
const TURNING =
  "Turn the focuser one way until the reading has passed its lowest and risen again, then come back to the lowest.";
const PASSED = "Passed through focus and came back to the best so far.";
const NONE = "—";

// The nine tiles of the sensor, by row then col, as the select lists them.
const TILES = [
  { col: 0, row: 0, name: "top left" },
  { col: 1, row: 0, name: "top centre" },
  { col: 2, row: 0, name: "top right" },
  { col: 0, row: 1, name: "middle left" },
  { col: 1, row: 1, name: "centre" },
  { col: 2, row: 1, name: "middle right" },
  { col: 0, row: 2, name: "bottom left" },
  { col: 1, row: 2, name: "bottom centre" },
  { col: 2, row: 2, name: "bottom right" },
];

const TREND_WORDS: Record<NonNullable<ImagingFocusAid["trend"]>, string> = {
  better: "Sharper than before the last change",
  worse: "Softer than before the last change",
  same: "About the same as before the last change",
};

const tileName = (col: number, row: number) => TILES.find((t) => t.col === col && t.row === row)?.name ?? "";
const capital = (name: string) => name.charAt(0).toUpperCase() + name.slice(1);

// The status line of the reading block, by what the last imaging frame was.
function lastStatus(aid: ImagingFocusAid, size: (v: number) => string): string {
  const last = aid.last;
  if (last == null) return "Waiting for the first imaging frame";
  const stars = last.stars != null ? ` · ${last.stars} stars` : "";
  switch (last.state) {
    case "reading":
      return aid.median?.readings === 3
        ? `Median of the last 3 imaging frames${stars}`
        : `${aid.median?.readings ?? 0} of 3 imaging frames since the last change${stars}`;
    case "settling":
      return "Settling after a change";
    case "too_few_stars":
      return `Not measurable: ${last.stars ?? 0} stars in the focus region, and 5 are needed`;
    case "far_out":
      return last.value != null ? `Far out of focus: stars about ${size(last.value)} across` : "Far out of focus";
  }
}

// The verdict against the record, in its own words: no line says focus is good
// without naming the record it is compared with (033 FR-032).
function verdictWords(aid: ImagingFocusAid, size: (v: number) => string): string | null {
  const verdict = aid.verdict;
  if (verdict == null) return null;
  if (verdict === "no_record")
    return "No focus record for this camera yet. Run the focus aid through focus to make one.";
  if (verdict === "unmeasurable")
    return aid.last?.state === "too_few_stars"
      ? "Not measurable: too few stars in the focus region."
      : "Not measurable: the reading did not settle.";
  const reference = aid.reference;
  if (reference != null && aid.median != null) {
    const where = reference.of === "tonight" ? " tonight" : " from earlier nights";
    const pct = Math.round((aid.median.value / reference.value - 1) * 100);
    if (verdict === "good") return `Within 10% of the focus record of ${size(reference.value)}${where}.`;
    if (verdict === "slightly_soft")
      return `Slightly soft: ${pct}% larger than the focus record of ${size(reference.value)}${where}.`;
    return `Clearly out: ${pct}% larger than the focus record of ${size(reference.value)}${where}.`;
  }
  if (verdict === "clearly_out") return "Clearly out: the stars are too large to measure.";
  return null;
}

// The history as ink bars: one currentColor bar a measured frame, 36 units tall at
// the largest value shown, and an empty place for an unmeasurable one (033 US6-S5).
function History({ values }: { values: (number | null)[] }) {
  const shown = values.slice(-60);
  const numbers = shown.filter((v): v is number => v != null);
  const max = numbers.length > 0 ? Math.max(...numbers) : null;
  return (
    <svg
      data-focus-history
      role="img"
      aria-label={`Focus readings of the last ${shown.length} imaging frames`}
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

export function FocusBlock({ view, refresh }: { view: ImagingView; refresh: () => Promise<void> | void }) {
  // Drafts of the select and the field are the observer's own and survive the
  // one-second reads.
  const [region, setRegion] = useState("kept");
  const [time, setTime] = useState("1000");
  const [refusal, setRefusal] = useState<string | null>(null);
  const focus = view.focus;
  if (focus == null) return null;
  const aid = focus.aid;
  const running = aid != null && aid.state !== "ended";
  const unit = focus.unit === "arcsec" ? "″" : " px";
  const size = (v: number) => `${v.toFixed(2)}${unit}`;

  const send = async (request: () => Promise<unknown>) => {
    setRefusal(null);
    try {
      await request();
    } catch (e) {
      setRefusal(e instanceof Error ? e.message : "The focus aid could not be reached.");
      return;
    }
    await refresh();
  };

  const start = async (check: boolean) => {
    // The body's keys in this order, each only when it is set (033 US6-S2).
    const ask: ImagingFocusAsk = { exposure_ms: Number(time) };
    if (region === "find") ask.find = true;
    else {
      const tile = TILES.find((t) => `${t.col},${t.row}` === region);
      if (tile != null) ask.tile = { col: tile.col, row: tile.row };
    }
    if (check) ask.check = true;
    await send(() => postImagingFocus(ask));
  };

  let regionLine: string | null = null;
  const kept = focus.region;
  if (kept != null) {
    const name = tileName(kept.col, kept.row);
    regionLine =
      kept.source === "provisional"
        ? `Focus region: the ${name} tile for now. The sharpest tile is found once the stars are small enough to measure.`
        : `Focus region: the ${name} tile, ${kept.source === "found" ? "found from an imaging frame" : "set by hand"}.`;
  }

  let recordLine: string | null = null;
  const record = focus.record;
  if (record?.tonight != null && record.earlier != null)
    recordLine = `Focus record: ${size(record.tonight.value)} tonight · ${size(record.earlier.value)} on earlier nights.`;
  else if (record?.tonight != null) recordLine = `Focus record: ${size(record.tonight.value)} tonight.`;
  else if (record?.earlier != null) recordLine = `Focus record: ${size(record.earlier.value)} on earlier nights.`;

  const ended = aid?.state === "ended";
  const stopped = ended && aid.ended_reason === "stopped";
  const done = ended && aid.ended_reason === "done";
  const startedStatus =
    aid?.state === "starting"
      ? "Starting the focus aid"
      : aid?.state === "finding"
        ? "Finding the focus region on a full imaging frame"
        : null;
  const endedStatus = stopped ? "Focus aid stopped." : done ? "Focus check done." : null;
  const endedAlert =
    ended && aid.ended_reason != null && !stopped && !done ? `Focus aid stopped: ${aid.error ?? ""}.` : null;
  const restoreAlert =
    aid?.restore_error != null ? `The camera's settings were not put back: ${aid.restore_error}.` : null;
  const showReading = aid != null && (aid.state === "running" || done);
  const verdict = aid != null ? verdictWords(aid, size) : null;
  const roi = view.settings?.roi;
  const camera = view.camera;
  const wholeSensor =
    roi != null &&
    camera != null &&
    roi.x === 0 &&
    roi.y === 0 &&
    roi.width === camera.width &&
    roi.height === camera.height;
  const partialSensor = !running && roi != null && camera != null && !wholeSensor;

  return (
    <section aria-labelledby="focus-title" className="space-y-3">
      <h2 id="focus-title" className="section-title">
        Focus
      </h2>
      <div className="space-y-3 border-y border-rule py-4">
        <p className="quiet">{STANDING}</p>
        {focus.unit === "px" && <p className="quiet">{PX_NOTE}</p>}
        {regionLine != null && <p className="quiet">{regionLine}</p>}
        {recordLine != null && <p className="quiet">{recordLine}</p>}
        {running ? (
          <button type="button" className="button" onClick={() => void send(() => deleteImagingFocus())}>
            Stop focus aid
          </button>
        ) : (
          <>
            <div className="grid grid-cols-2 gap-3">
              <label className="block space-y-1">
                <span className="text-sm font-semibold text-ink-2">Focus region</span>
                <select
                  aria-label="Focus region"
                  value={region}
                  onChange={(e) => setRegion(e.target.value)}
                  className="h-11 w-full min-w-0 border border-ink bg-transparent px-3 text-base outline-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink"
                >
                  <option value="kept">Kept, or the sharpest tile</option>
                  <option value="find">Find the sharpest tile again</option>
                  {TILES.map((t) => (
                    <option key={`${t.col},${t.row}`} value={`${t.col},${t.row}`}>
                      {capital(t.name)}
                    </option>
                  ))}
                </select>
              </label>
              <label className="block space-y-1">
                <span className="text-sm font-semibold text-ink-2">Focus frame time</span>
                <span className="flex items-center border border-ink focus-within:outline-2 focus-within:outline-offset-2 focus-within:outline-ink">
                  <input
                    aria-label="Focus frame time"
                    inputMode="decimal"
                    value={time}
                    onChange={(e) => setTime(e.target.value)}
                    className="h-11 w-full min-w-0 bg-transparent px-3 text-base outline-none"
                  />
                  <span aria-hidden="true" className="pr-3 text-sm whitespace-nowrap text-ink-2">
                    ms
                  </span>
                </span>
              </label>
            </div>
            <div className="flex flex-wrap gap-3">
              <button type="button" className="button button-primary" onClick={() => void start(false)}>
                Start focus aid
              </button>
              <button type="button" className="button" onClick={() => void start(true)}>
                Check focus
              </button>
            </div>
            {partialSensor && (
              <>
                <p className="quiet">The camera is reading a region of its sensor.</p>
                <button type="button" className="button" onClick={() => void send(() => putImagingWholeSensor())}>
                  Use the whole sensor
                </button>
              </>
            )}
          </>
        )}
        {startedStatus != null && <p role="status">{startedStatus}</p>}
        {endedStatus != null && <p role="status">{endedStatus}</p>}
        {endedAlert != null && <p role="alert">{endedAlert}</p>}
        {restoreAlert != null && <p role="alert">{restoreAlert}</p>}
        {aid?.ended_reason === "no_region" && <p className="quiet">{SPARSE}</p>}
        {showReading && aid != null && (
          <>
            <p className="group-title">Focus reading</p>
            <p className="text-[2.35rem] font-[680] tracking-[-0.02em] sm:text-[2.75rem]">
              {aid.median != null ? size(aid.median.value) : NONE}
            </p>
            <p role="status">{lastStatus(aid, size)}</p>
            {aid.last?.state === "too_few_stars" && <p className="quiet">{SPARSE}</p>}
            {aid.best != null && <p>{`Best so far ${size(aid.best)}`}</p>}
            {aid.trend != null && <p>{TREND_WORDS[aid.trend]}</p>}
            {aid.found ? <p>{PASSED}</p> : running ? <p>{TURNING}</p> : null}
            {verdict != null && <p>{verdict}</p>}
            <History values={aid.history} />
          </>
        )}
        {refusal != null && <p role="alert">{refusal}</p>}
      </div>
    </section>
  );
}
