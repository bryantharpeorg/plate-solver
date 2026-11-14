import { renderHook } from "@testing-library/react";
import { createElement, StrictMode } from "react";
import { describe, expect, it } from "vitest";
import type { SolveReport } from "./api";
import { calm, seenNow, useCalm, type CalmEntry } from "./calm";

function report(matched: boolean, detected?: boolean): SolveReport {
  return {
    name: null,
    status: "match_found",
    hint: null,
    width: 100,
    height: 100,
    timing_ms: { decode: 0, detect: 0, solve: 0 },
    detected: [],
    used: 0,
    pointing: {
      ra_deg: 0,
      dec_deg: 0,
      ra_text: "0h",
      dec_text: "0°",
      constellation: { abbr: "Ori", name: "Orion" },
      roll_deg: 0,
      north_deg: 0,
      fov_deg: 10,
      fov_vertical_deg: 10,
      arcsec_per_px: 360,
      distortion: 0,
      matched: 0,
      rmse_arcsec: null,
      p90_arcsec: null,
      max_arcsec: null,
      false_alarm: null,
    },
    stars: [{ id: "S", name: "Star S", x: 25, y: 25, mag: 2, matched, detected, error_px: null }],
    objects: [],
    nearby: [],
    grid: [],
  };
}

// Oracle: the spec's sighting sequences and supplied exposure times. Booleans and
// identity are exact (zero tolerance); history is built from raw sightings, never holds.
function sequence(sightings: boolean[], times = sightings.map((_, i) => i * 3)) {
  const history: CalmEntry[] = [];
  return sightings.map((saw, i) => {
    const drawn = calm(report(saw), times[i]!, history);
    history.push({ time: times[i]!, seen: new Set(saw ? ["S"] : []) });
    return drawn.stars[0]!.seen;
  });
}

describe("the live star hold", () => {
  it("holds an alternating star on the fourth report (S1)", () => {
    expect(sequence([true, false, true, false])).toEqual([true, false, true, true]);
  });

  it("does not hold a single sighting (S2, negative control)", () => {
    expect(sequence([true, false])).toEqual([true, false]);
  });

  it("holds through the fifth report, then drops after three misses (S3)", () => {
    expect(sequence([true, true, true, false, false, false])).toEqual([true, true, true, true, true, false]);
  });

  it("drops twelve seconds after the latest sighting (S4)", () => {
    expect(sequence([true, true, false, false], [0, 4, 8, 16])).toEqual([true, true, true, false]);
  });

  it("holds at ten seconds, but not beyond, and ignores sightings outside the last three", () => {
    const history = [
      { time: 0, seen: new Set(["S"]) },
      { time: 4, seen: new Set(["S"]) },
    ];
    expect(calm(report(false), 14, history).stars[0]!.seen).toBe(true);
    expect(calm(report(false), 14.001, history).stars[0]!.seen).toBe(false);
    expect(
      calm(report(false), 10, [...history, { time: 7, seen: new Set() }, { time: 9, seen: new Set() }]).stars[0]!.seen,
    ).toBe(false);
  });

  it("uses matched or detected in the current report and copies without mutation", () => {
    const source = report(false, true);
    const drawn = calm(source, 0, []);
    expect(seenNow(source.stars[0]!)).toBe(true);
    expect(seenNow(report(true).stars[0]!)).toBe(true);
    expect(seenNow(report(false).stars[0]!)).toBe(false);
    expect(drawn.stars[0]!.seen).toBe(true);
    expect(drawn).not.toBe(source);
    expect(drawn.stars[0]).not.toBe(source.stars[0]);
    expect(source.stars[0]).not.toHaveProperty("seen");
  });
});

describe("useCalm history", () => {
  it("counts each source once across StrictMode and repeated timer renders", () => {
    const first = report(true);
    const { result, rerender } = renderHook(({ source, time }) => useCalm(source, time, true), {
      initialProps: { source: first, time: 0 },
      wrapper: ({ children }) => createElement(StrictMode, null, children),
    });
    const drawn = result.current;
    rerender({ source: first, time: 1 });
    expect(result.current).toBe(drawn);
    rerender({ source: report(false), time: 3 });
    expect(result.current!.stars[0]!.seen).toBe(false);
  });

  it("records detections alone and clears history when leaving live", () => {
    const { result, rerender } = renderHook(({ source, live }) => useCalm(source, 0, live), {
      initialProps: { source: report(false, true), live: true },
    });
    rerender({ source: report(false, true), live: true });
    rerender({ source: report(false), live: true });
    expect(result.current!.stars[0]!.seen).toBe(true);
    const upload = report(false);
    rerender({ source: upload, live: false });
    expect(result.current).toBe(upload);
    rerender({ source: report(false), live: true });
    expect(result.current!.stars[0]!.seen).toBe(false);
  });

  it("passes through unsolved, null and untimed reports without advancing solved history", () => {
    const { result, rerender } = renderHook(
      ({ source, time }: { source: SolveReport | null; time: number | null }) => useCalm(source, time, true),
      { initialProps: { source: report(true) as SolveReport | null, time: 0 as number | null } },
    );
    rerender({ source: report(true), time: 3 });
    const unsolved = { ...report(false), pointing: null, status: "no_match" as const };
    const untimed = report(false);
    for (const source of [unsolved, null, untimed]) {
      rerender({ source, time: source === untimed ? null : 4 });
      expect(result.current).toBe(source);
    }
    rerender({ source: report(false), time: 6 });
    expect(result.current!.stars[0]!.seen).toBe(true);
  });
});
