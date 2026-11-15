// The chart: the frame with the solution printed over it the way a star atlas prints a
// chart. The RA/Dec grid is labelled in the paper margin, matched stars are ringed and
// named, and deep-sky objects are drawn in the atlas's symbols. Everything is drawn in the
// frame's own pixel coordinates, so each mark sits where the server put it.

import { useEffect, useId, useLayoutEffect, useRef, useState, type CSSProperties, type RefObject } from "react";
import type { Edge, Family, GridLine, ObjectMark, SolveReport, StarMark } from "../api";
import { seenNow } from "../calm";
import { mag, objectTitle, scaleBar } from "../format";
import {
  galaxyMinor,
  layoutLabels,
  measureText,
  objectRadius,
  OBJECT_TEXT,
  ringRadius,
  STAR_TEXT,
  type Box,
  type LabelSpot,
  type Measure,
} from "../labels";
import { FamilyShape } from "./Symbol";
import { TargetMark } from "./TargetMark";
import type { ObservingSession } from "../useSession";

/** What the chart draws; each is a chip in the legend. */
export interface Layers {
  grid: boolean;
  /** Catalog stars seen in this report or held from recent live reports. */
  matched: boolean;
  /** Catalog stars in the frame that were not detected. */
  expected: boolean;
  objects: boolean;
  /** Every star the detector found. */
  detected: boolean;
}

export const DEFAULT_LAYERS: Layers = { grid: true, matched: true, expected: true, objects: true, detected: false };

type Picked = { kind: "star"; star: StarMark } | { kind: "object"; object: ObjectMark } | null;

interface Props {
  report: SolveReport | null;
  imageUrl: string;
  label: string;
  busy: boolean;
  layers: Layers;
  onLayers: (layers: Layers) => void;
  /** The report is the newest live frame's, not an uploaded or sample frame's. */
  live?: boolean;
  session?: ObservingSession;
}

/** Unsolved live frames in a row that keep the last live solve on the chart. */
export const LIVE_HOLD = 2;

interface Steady {
  live: boolean;
  /** The report this state was last advanced for. */
  seen: SolveReport | null;
  /** The newest solved live report. */
  last: SolveReport | null;
  /** Unsolved live reports since `last`. */
  misses: number;
  /** A solved live chart has been drawn since the chart switched to live. */
  settled: boolean;
}

const UNSTEADY: Steady = { live: false, seen: null, last: null, misses: 0, settled: false };

/** Advance the live chart's state by one report. */
export function advance(state: Steady, report: SolveReport | null, live: boolean): Steady {
  if (!live || !report) return { ...UNSTEADY, live, seen: report };
  const base = state.live ? state : { ...UNSTEADY, live };
  if (report === base.seen) return base;
  const settled = base.settled || base.last !== null;
  if (report.pointing) return { live, seen: report, last: report, misses: 0, settled };
  return { ...base, seen: report, misses: base.misses + 1, settled };
}

function useSteady(report: SolveReport | null, live: boolean) {
  const [previous, setPrevious] = useState(UNSTEADY);
  let state = previous;
  if (report !== state.seen || live !== state.live) {
    state = advance(state, report, live);
    setPrevious(state);
  }
  return {
    shown: live && !report?.pointing && state.last && state.misses <= LIVE_HOLD ? state.last : report,
    intro: live ? !state.settled : true,
  };
}

/** Paper margin around the frame for the grid's labels, in CSS pixels. */
const MARGIN = 30;

let measurer: Measure | null = null;
/** Text widths in the page's font, measured once the font has loaded. */
function useMeasure(): Measure {
  // Lay the labels out again once the font has loaded, in its own widths.
  const [, setLoaded] = useState(false);
  useEffect(() => {
    let live = true;
    document.fonts?.ready.then(() => live && setLoaded(true));
    return () => {
      live = false;
    };
  }, []);
  measurer ??= measureText();
  return measurer;
}

function useWidth(ref: RefObject<HTMLElement | null>): number {
  const [width, setWidth] = useState(0);
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    setWidth(el.getBoundingClientRect().width);
    const observer = new ResizeObserver(([entry]) => entry && setWidth(entry.contentRect.width));
    observer.observe(el);
    return () => observer.disconnect();
  }, [ref]);
  return width;
}

export function Chart({ report: newest, imageUrl, label, busy, layers, onLayers, live = false, session }: Props) {
  const { shown: report, intro } = useSteady(newest, live);
  const box = useRef<HTMLDivElement>(null);
  const width = useWidth(box);
  const [picked, setPicked] = useState<Picked>(null);
  const clip = useId();
  const measure = useMeasure();

  if (!report) {
    return (
      <figure className="chart" aria-busy={busy}>
        <div ref={box}>
          <div className="relative border border-ink" style={{ margin: MARGIN }}>
            <img src={imageUrl} alt={`Frame ${label}`} className="photo block w-full opacity-60" />
            {busy && (
              <p className="absolute inset-x-0 bottom-0 m-0 bg-photo/80 px-3 py-2 text-sm font-medium text-on-photo">
                Charting {label}…
              </p>
            )}
          </div>
        </div>
      </figure>
    );
  }

  const W = report.width;
  const H = report.height;
  // Image pixels per CSS pixel, so strokes and labels keep their on-screen size.
  const u = width > 2 * MARGIN + 40 ? W / (width - 2 * MARGIN) : W / 320;
  const m = MARGIN * u;
  const solved = report.pointing !== null;
  // With nothing solved, the detected stars are the whole story: show them.
  const showDetected = layers.detected || !solved;
  const key = live ? "live" : `${report.name}:${report.pointing?.ra_deg ?? "none"}`;
  const stars = report.stars.filter((s) => ((s.seen ?? seenNow(s)) ? layers.matched : layers.expected));
  const objects = layers.objects ? report.objects : [];
  const detected = showDetected ? report.detected : [];
  const furniture = solved ? furnitureGeometry(report, u) : null;
  const spots = layoutLabels({
    W,
    H,
    u,
    stars,
    objects,
    detected,
    furniture: furniture?.boxes ?? [],
    measure,
  });

  return (
    <figure
      className="chart lg:mx-auto lg:max-w-[calc((100dvh-11rem)*var(--chart-aspect))]"
      style={{ "--chart-aspect": W / H } as CSSProperties}
    >
      {/* Leave room for guidance on a phone while keeping the legend at full width. */}
      <div ref={box} className={session?.target ? "max-sm:mx-5" : undefined}>
        <svg
          key={key}
          viewBox={`${-m} ${-m} ${W + 2 * m} ${H + 2 * m}`}
          className="block h-auto w-full select-none"
          role="img"
          aria-label={solved ? `Chart of ${label} with the solution drawn on it` : `Frame ${label}`}
          onClick={() => setPicked(null)}
        >
          <defs>
            <clipPath id={clip}>
              <rect width={W} height={H} />
            </clipPath>
          </defs>
          <rect
            x={-m + 1.5 * u}
            y={-m + 1.5 * u}
            width={W + 2 * m - 3 * u}
            height={H + 2 * m - 3 * u}
            fill="none"
            stroke="var(--color-ink)"
            strokeWidth={u}
          />
          <image href={imageUrl} width={W} height={H} preserveAspectRatio="none" className="photo" />
          <rect width={W} height={H} fill="none" stroke="var(--color-ink)" strokeWidth={1.4 * u} />

          {layers.grid && solved && <Grid lines={report.grid} W={W} H={H} u={u} clip={clip} intro={intro} />}

          <g clipPath={`url(#${clip})`} fontSize={STAR_TEXT * u}>
            {detected.map(([x, y], i) => (
              <circle key={i} cx={x} cy={y} r={3 * u} strokeWidth={u} className="mark-detected" />
            ))}
            {objects.map((o, i) => (
              <ObjectSymbol
                key={o.id}
                object={o}
                u={u}
                spot={spots.get(`o:${o.id}`)}
                delay={450 + i * 60}
                picked={picked?.kind === "object" && picked.object.id === o.id}
                onPick={() => setPicked({ kind: "object", object: o })}
              />
            ))}
            {stars.map((s) => (
              <StarSymbol
                key={s.id}
                star={s}
                u={u}
                spot={spots.get(`s:${s.id}`)}
                picked={picked?.kind === "star" && picked.star.id === s.id}
                onPick={() => setPicked({ kind: "star", star: s })}
              />
            ))}
            {furniture && <Furniture geometry={furniture} u={u} intro={intro} />}
            {solved &&
              session?.target &&
              session.guidance?.placement &&
              session.pointing &&
              report.pointing_seq !== undefined &&
              report.pointing_seq === session.pointing.seq && (
                <TargetMark target={session.target} placement={session.guidance.placement} W={W} H={H} u={u} />
              )}
          </g>
        </svg>
      </div>
      <figcaption className="mt-2.5 min-h-11 sm:mx-[30px] sm:mt-3">
        {picked ? (
          <PickedLine picked={picked} />
        ) : (
          <Legend report={report} layers={{ ...layers, detected: showDetected }} fixed={!solved} onLayers={onLayers} />
        )}
      </figcaption>
    </figure>
  );
}

function toPath(points: [number, number][]): string {
  return points.map(([x, y], i) => `${i ? "L" : "M"}${x.toFixed(1)} ${y.toFixed(1)}`).join("");
}

/** The grid lines, clipped to the frame, with their labels in the margin at its edges. */
function Grid({
  lines,
  W,
  H,
  u,
  clip,
  intro: introNow,
}: {
  lines: GridLine[];
  W: number;
  H: number;
  u: number;
  clip: string;
  intro: boolean;
}) {
  const [intro] = useState(introNow);
  const gap = 6 * u;
  const labels = placeLabels(lines, W, H, u);
  return (
    <g>
      <g clipPath={`url(#${clip})`}>
        {lines.flatMap((line, i) =>
          line.paths.map((path, j) => (
            <path
              key={`${line.kind}:${line.value_deg}.${j}`}
              d={toPath(path)}
              pathLength={1}
              className={intro ? "grid-line draw" : "grid-line"}
              strokeWidth={u}
              style={intro ? { animationDelay: `${i * 40}ms` } : undefined}
            />
          )),
        )}
      </g>
      <g
        fontSize={11 * u}
        className={intro ? "ink-in" : undefined}
        style={intro ? { animationDelay: "250ms" } : undefined}
      >
        {labels.map(({ x, y, edge, text }, i) => {
          const tick =
            edge === "top"
              ? `M${x} 0V${-4 * u}`
              : edge === "bottom"
                ? `M${x} ${H}V${H + 4 * u}`
                : edge === "left"
                  ? `M0 ${y}H${-4 * u}`
                  : `M${W} ${y}H${W + 4 * u}`;
          const [tx, ty, rotate, baseline]: [number, number, number, "auto" | "hanging"] =
            edge === "top"
              ? [x, -gap, 0, "auto"]
              : edge === "bottom"
                ? [x, H + gap, 0, "hanging"]
                : edge === "left"
                  ? [-gap, y, -90, "auto"]
                  : [W + gap, y, 90, "auto"];
          return (
            <g key={i}>
              <path d={tick} stroke="var(--color-ink)" strokeWidth={u} />
              <text
                x={tx}
                y={ty}
                textAnchor="middle"
                dominantBaseline={baseline}
                transform={rotate ? `rotate(${rotate} ${tx} ${ty})` : undefined}
                className="edge-label"
              >
                {text}
              </text>
            </g>
          );
        })}
      </g>
    </g>
  );
}

/** One label per grid line end, dropping any that would crowd a neighbour or a corner. */
function placeLabels(lines: GridLine[], W: number, H: number, u: number) {
  const spacing = 52 * u;
  const corner = 16 * u;
  const all = lines.flatMap((line) => line.ticks.map((t) => ({ ...t, text: line.label })));
  const kept: { x: number; y: number; edge: Edge; text: string }[] = [];
  for (const edge of ["top", "right", "bottom", "left"] as const) {
    const along = (t: { x: number; y: number }) => (edge === "top" || edge === "bottom" ? t.x : t.y);
    const length = edge === "top" || edge === "bottom" ? W : H;
    let last = -Infinity;
    for (const t of all.filter((t) => t.edge === edge).sort((a, b) => along(a) - along(b))) {
      const at = along(t);
      if (at < corner || at > length - corner || at - last < spacing) continue;
      kept.push(t);
      last = at;
    }
  }
  return kept;
}

/** A leader from a mark to its label, cased in the photo's dark like the labels. */
function Leader({ spot, u }: { spot: LabelSpot; u: number }) {
  if (!spot.leader) return null;
  const { x1, y1, x2, y2 } = spot.leader;
  const d = `M${x1} ${y1}L${x2} ${y2}`;
  return (
    <>
      <path d={d} strokeWidth={3 * u} className="leader-casing" />
      <path d={d} strokeWidth={u} stroke="currentColor" className="leader" />
    </>
  );
}

function ObjectSymbol({
  object: o,
  u,
  spot,
  delay,
  picked,
  onPick,
}: {
  object: ObjectMark;
  u: number;
  spot: LabelSpot | undefined;
  delay: number;
  picked: boolean;
  onPick: () => void;
}) {
  const r = objectRadius(o, u);
  const galaxy = o.family === "galaxy";
  // Labels and leaders are laid out in image coordinates; this group sits at the object.
  const local = spot && {
    ...spot,
    x: spot.x - o.x,
    y: spot.y - o.y,
    leader: spot.leader && {
      x1: spot.leader.x1 - o.x,
      y1: spot.leader.y1 - o.y,
      x2: spot.leader.x2 - o.x,
      y2: spot.leader.y2 - o.y,
    },
  };
  return (
    <g
      transform={`translate(${o.x} ${o.y})`}
      className={`object-${o.family} ink-in cursor-pointer`}
      style={{ animationDelay: `${delay}ms` }}
      data-picked={picked}
      data-object={o.id}
      onClick={(e) => {
        e.stopPropagation();
        onPick();
      }}
    >
      <title>{`${o.id} ${objectTitle(o)}`}</title>
      {galaxy ? (
        <ellipse
          rx={r}
          ry={galaxyMinor(o, u)}
          transform={`rotate(${o.angle_deg - 90})`}
          fill="none"
          stroke="currentColor"
          strokeWidth={1.6}
          vectorEffect="non-scaling-stroke"
          className="shape"
        />
      ) : (
        <g className="shape">
          <FamilyShape family={o.family} r={r} />
        </g>
      )}
      <circle r={Math.max(r, 18 * u)} className="hit" />
      {local && (
        <>
          <Leader spot={local} u={u} />
          <text
            x={local.x}
            y={local.y}
            textAnchor={local.anchor}
            fontSize={OBJECT_TEXT * u}
            strokeWidth={3 * u}
            className="object-label"
          >
            {o.id}
          </text>
        </>
      )}
    </g>
  );
}

function StarSymbol({
  star: s,
  u,
  spot,
  picked,
  onPick,
}: {
  star: StarMark;
  u: number;
  spot: LabelSpot | undefined;
  picked: boolean;
  onPick: () => void;
}) {
  const seen = s.seen ?? seenNow(s);
  return (
    <g
      className={`${seen ? "mark-matched" : "mark-expected"} ink-in`}
      style={{ animationDelay: seen ? "350ms" : "600ms" }}
      data-picked={picked}
      data-star={s.id}
      onClick={(e) => {
        e.stopPropagation();
        onPick();
      }}
    >
      <circle cx={s.x} cy={s.y} r={ringRadius(s) * u} strokeWidth={1.3 * u} className="ring" />
      <circle cx={s.x} cy={s.y} r={16 * u} className="hit" />
      {s.name && spot && (
        <>
          <Leader spot={spot} u={u} />
          <text x={spot.x} y={spot.y} textAnchor={spot.anchor} strokeWidth={3 * u} className="mark-label">
            {s.name}
          </text>
        </>
      )}
    </g>
  );
}

interface FurnitureGeometry {
  /** The frame's centre, where the reticle goes. */
  cx: number;
  cy: number;
  /** The compass's centre, its arm length, and where north points. */
  ax: number;
  ay: number;
  arm: number;
  north: number;
  /** The scale bar: left end, length, baseline, label. */
  pad: number;
  bar: { px: number; label: string };
  by: number;
  /** What the furniture covers, for keeping labels off it. */
  boxes: Box[];
}

/** Where the chart's furniture goes: compass top right, scale bar bottom left, reticle at the centre. */
export function furnitureGeometry(report: SolveReport, u: number): FurnitureGeometry {
  const W = report.width;
  const H = report.height;
  const p = report.pointing;
  const pad = 16 * u;
  const arm = 24 * u;
  const ax = W - pad - arm;
  const ay = pad + arm;
  const bar = scaleBar(p?.arcsec_per_px ?? 1, Math.min(W, H) * 0.22);
  const by = H - pad;
  const reach = arm + 16 * u;
  const boxes: Box[] = [
    { x0: ax - reach, y0: ay - reach, x1: ax + reach, y1: ay + reach },
    { x0: pad - 2 * u, y0: by - 22 * u, x1: pad + Math.max(bar.px, bar.px / 2 + 16 * u) + 2 * u, y1: by + 2 * u },
    { x0: W / 2 - 12 * u, y0: H / 2 - 12 * u, x1: W / 2 + 12 * u, y1: H / 2 + 12 * u },
  ];
  return { cx: W / 2, cy: H / 2, ax, ay, arm, north: p?.north_deg ?? 0, pad, bar, by, boxes };
}

/** The chart's furniture: centre reticle, north and east, and a scale bar. */
function Furniture({ geometry: g, u, intro: introNow }: { geometry: FurnitureGeometry; u: number; intro: boolean }) {
  const [intro] = useState(introNow);
  const { cx, cy } = g;
  const at = (angle: number, length: number) => {
    const a = (angle * Math.PI) / 180;
    return [g.ax + Math.sin(a) * length, g.ay - Math.cos(a) * length] as const;
  };
  // On a frame of the sky, east is a quarter turn counterclockwise from north.
  const east = g.north - 90;
  const [nx, ny] = at(g.north, g.arm);
  const [ex, ey] = at(east, g.arm * 0.7);
  const [nlx, nly] = at(g.north, g.arm + 9 * u);
  const [elx, ely] = at(east, g.arm * 0.7 + 9 * u);
  return (
    <g
      className={intro ? "furniture ink-in" : "furniture"}
      strokeWidth={1.3 * u}
      style={intro ? { animationDelay: "300ms" } : undefined}
    >
      <path
        d={`M${cx - 11 * u} ${cy}H${cx - 4 * u}M${cx + 4 * u} ${cy}H${cx + 11 * u}M${cx} ${cy - 11 * u}V${cy - 4 * u}M${cx} ${cy + 4 * u}V${cy + 11 * u}`}
        fill="none"
      />
      <path d={`M${g.ax} ${g.ay}L${nx} ${ny}M${g.ax} ${g.ay}L${ex} ${ey}`} fill="none" />
      <text x={nlx} y={nly} textAnchor="middle" dominantBaseline="middle" fontSize={11 * u}>
        N
      </text>
      <text x={elx} y={ely} textAnchor="middle" dominantBaseline="middle" fontSize={11 * u}>
        E
      </text>
      <path d={`M${g.pad} ${g.by - 5 * u}V${g.by}H${g.pad + g.bar.px}V${g.by - 5 * u}`} fill="none" />
      <text x={g.pad + g.bar.px / 2} y={g.by - 8 * u} textAnchor="middle" fontSize={11 * u}>
        {g.bar.label}
      </text>
    </g>
  );
}

/** A legend key drawn like the mark it stands for. */
function Swatch({ layer, families }: { layer: keyof Layers; families: Family[] }) {
  if (layer === "objects") {
    return (
      <span className="swatch inline-flex gap-0.5" aria-hidden="true">
        {families.map((f) => (
          <svg key={f} viewBox="-8 -8 16 16" width="14" height="14" className={`icon-${f}`}>
            <FamilyShape family={f} r={f === "galaxy" ? 6.5 : 5} />
          </svg>
        ))}
      </span>
    );
  }
  return (
    <svg viewBox="-8 -8 16 16" width="14" height="14" aria-hidden="true" className="swatch">
      {layer === "grid" && (
        <path d="M-7 -3H7M-7 3H7M-3 -7V7M3 -7V7" stroke="currentColor" strokeWidth="1.2" fill="none" />
      )}
      {layer === "matched" && <circle r="5.5" stroke="currentColor" strokeWidth="1.4" fill="none" />}
      {layer === "expected" && (
        <circle r="4.5" stroke="currentColor" strokeWidth="1.3" strokeDasharray="2 2.2" fill="none" />
      )}
      {layer === "detected" && <circle r="3" stroke="currentColor" strokeWidth="1.2" fill="none" />}
    </svg>
  );
}

function Legend({
  report,
  layers,
  fixed,
  onLayers,
}: {
  report: SolveReport;
  layers: Layers;
  /** Unsolved: the detected stars are all there is, so they stay on. */
  fixed: boolean;
  onLayers: (l: Layers) => void;
}) {
  const matched = report.stars.filter((s) => s.seen ?? seenNow(s)).length;
  // Matched and Unseen split the catalog stars in the frame; Centroids are what the
  // detector found, a different set, so its name does not pair with theirs.
  // The deep-sky key shows the symbols actually on this chart, up to three of them.
  const families = [...new Set(report.objects.map((o) => o.family))].slice(0, 3);
  const items: { key: keyof Layers; label: string; count: number }[] = fixed
    ? [{ key: "detected", label: "Centroids", count: report.detected.length }]
    : [
        { key: "grid", label: "Grid", count: report.grid.length },
        { key: "matched", label: "Matched", count: matched },
        { key: "expected", label: "Unseen", count: report.stars.length - matched },
        { key: "objects", label: "Deep sky", count: report.objects.length },
        { key: "detected", label: "Centroids", count: report.detected.length },
      ];
  return (
    <div className="flex flex-wrap gap-1.5 sm:gap-2" role="group" aria-label="Show on the chart">
      {items
        .filter((item) => item.count > 0 || item.key === "detected")
        .map((item) => (
          <button
            key={item.key}
            type="button"
            className="chip"
            aria-pressed={layers[item.key]}
            disabled={fixed}
            onClick={() => onLayers({ ...layers, [item.key]: !layers[item.key] })}
          >
            <Swatch layer={item.key} families={families} />
            {item.label}
            <span className="chip-count">{item.count}</span>
          </button>
        ))}
    </div>
  );
}

function PickedLine({ picked }: { picked: NonNullable<Picked> }) {
  const fact = "text-ink-2";
  if (picked.kind === "star") {
    const s = picked.star;
    return (
      <p className="flex flex-wrap items-baseline gap-x-3 gap-y-1 text-[0.9375rem]">
        <strong className="font-semibold">{s.name ?? s.id}</strong>
        {s.name && <span className={fact}>{s.id}</span>}
        <span className={fact}>mag {mag(s.mag)}</span>
        <span className={fact}>
          {s.matched
            ? `Matched; the solution puts it ${s.error_px?.toFixed(2) ?? "—"} px from where it was found`
            : s.detected
              ? "Detected; not used in the solve"
              : s.seen
                ? "Seen in recent frames, not in this one"
                : "In the catalog but not detected"}
        </span>
      </p>
    );
  }
  const o = picked.object;
  return (
    <p className="flex flex-wrap items-baseline gap-x-3 gap-y-1 text-[0.9375rem]">
      <strong className="font-semibold">{o.id}</strong>
      <span>{objectTitle(o)}</span>
      <span className={fact}>{o.kind}</span>
      <span className={fact}>mag {mag(o.mag)}</span>
      {o.major_arcmin !== null && <span className={fact}>{Math.round(o.major_arcmin)}′ across</span>}
    </p>
  );
}
