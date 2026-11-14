import { useState } from "react";
import type { SolveReport, StarMark } from "./api";

export const CALM_FRAMES = 3;
export const CALM_HITS = 2;
export const CALM_SECONDS = 10;

/** One solved live report: when it was exposed, and which star ids that report alone saw. */
export interface CalmEntry {
  time: number;
  seen: Set<string>;
}

/** Seen in this report alone. */
export const seenNow = (s: StarMark) => s.matched || s.detected === true;

/** The report with `seen` set on every star, given the previous solved live reports. */
export function calm(report: SolveReport, time: number, history: CalmEntry[]): SolveReport {
  const recent = history.slice(-CALM_FRAMES);
  return {
    ...report,
    stars: report.stars.map((s) => {
      const sightings = recent.filter((entry) => entry.seen.has(s.id));
      const latest = sightings.at(-1);
      const held = sightings.length >= CALM_HITS && latest !== undefined && time - latest.time <= CALM_SECONDS;
      return { ...s, seen: seenNow(s) || held };
    }),
  };
}

interface CalmState {
  live: boolean;
  source: SolveReport | null;
  shown: SolveReport | null;
  history: CalmEntry[];
}

/** Keeps the history for live reports and clears it when the page leaves live. */
export function useCalm(report: SolveReport | null, time: number | null, live: boolean): SolveReport | null {
  const [previous, setPrevious] = useState<CalmState>({ live: false, source: null, shown: null, history: [] });
  let state = previous;
  if (report !== state.source || live !== state.live) {
    let history = live ? state.history : [];
    let shown = report;
    if (live && report?.pointing && time !== null) {
      shown = calm(report, time, history);
      const entry = { time, seen: new Set(report.stars.filter(seenNow).map((s) => s.id)) };
      history = [...history, entry].slice(-CALM_FRAMES);
    }
    state = { live, source: report, shown, history };
    setPrevious(state);
  }
  return state.shown;
}
