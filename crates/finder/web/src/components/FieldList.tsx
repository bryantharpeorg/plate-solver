// What is in the field: the deep-sky objects on the frame, and the named stars seen. The
// roster says it in a line under the coordinates on a phone; the list says it in full.

import type { ObjectMark, SolveReport } from "../api";
import { seenNow } from "../calm";
import { mag, objectTitle } from "../format";
import { FamilyIcon } from "./Symbol";

interface Props {
  report: SolveReport;
  live?: boolean;
  selected: string | null;
  onSelect: (id: string) => void;
}

/** The seen stars with proper names, brightest first. */
const namedStars = (report: SolveReport) =>
  report.stars.filter((s) => (s.seen ?? seenNow(s)) && s.name).sort((a, b) => a.mag - b.mag);

/** Named stars as the chart marks them: a ring, then the names, brightest first. */
function NamedStars({ names }: { names: string[] }) {
  return (
    <span className="inline-flex items-start gap-1.5">
      <svg viewBox="-8 -8 16 16" width="14" height="14" aria-hidden="true" className="mt-[0.25em] shrink-0">
        <circle r="5.5" stroke="currentColor" strokeWidth="1.4" fill="none" />
      </svg>
      <span>{names.join(", ")}</span>
    </span>
  );
}

export function FieldList({ report, live = false, selected, onSelect }: Props) {
  const named = namedStars(report);
  const objects = report.objects;
  return (
    <section aria-labelledby="field-title" className="space-y-2">
      <h2 id="field-title" className="section-title">
        In the field
      </h2>
      {objects.length === 0 ? (
        <p className="quiet">No Messier objects in this frame.</p>
      ) : (
        <ul className="divide-y divide-rule border-y border-rule">
          {objects.map((o) => (
            <li key={o.id}>
              <ObjectRow object={o} selected={selected === o.id} onSelect={() => onSelect(o.id)} />
            </li>
          ))}
        </ul>
      )}
      {(live || named.length > 0) && (
        <p className="pt-1 text-[0.9375rem]" data-named-stars={live ? "" : undefined}>
          {named.length > 0 ? <NamedStars names={named.map((s) => s.name ?? "")} /> : "\u00a0"}
        </p>
      )}
    </section>
  );
}

/** Objects and named stars shown in the roster before it points to the full list. */
const ROSTER_OBJECTS = 6;
const ROSTER_STARS = 3;

/**
 * What is in the field in a line or two, for the top of a phone screen: each object's symbol
 * and designation, then the brightest named stars. The full list follows the chart.
 */
export function FieldRoster({ report }: { report: SolveReport }) {
  const objects = report.objects.slice(0, ROSTER_OBJECTS);
  const named = namedStars(report);
  const stars = named.slice(0, ROSTER_STARS);
  const more = report.objects.length - objects.length + named.length - stars.length;
  if (objects.length === 0 && stars.length === 0) {
    return <p className="quiet lg:hidden">No Messier objects or named stars in the field.</p>;
  }
  return (
    <p className="flex flex-wrap items-center gap-x-4 gap-y-1.5 lg:hidden">
      {objects.map((o) => (
        <span key={o.id} className="inline-flex items-center gap-1.5">
          <span className={`icon-${o.family}`}>
            <FamilyIcon family={o.family} size={16} />
          </span>
          <span className="font-semibold italic">{o.id}</span>
        </span>
      ))}
      {stars.length > 0 && <NamedStars names={stars.map((s) => s.name ?? "")} />}
      {more > 0 && (
        <a href="#field-title" className="link text-sm">
          {more} more
        </a>
      )}
    </p>
  );
}

function ObjectRow({ object: o, selected, onSelect }: { object: ObjectMark; selected: boolean; onSelect: () => void }) {
  return (
    <button
      type="button"
      aria-pressed={selected}
      onClick={onSelect}
      className="group grid w-full grid-cols-[1.5rem_3.25rem_1fr_auto] items-center gap-x-2 py-2.5 text-left transition-colors hover:bg-paper-2 aria-pressed:bg-paper-2"
    >
      <span className={`icon-${o.family} justify-self-center`}>
        <FamilyIcon family={o.family} size={18} />
      </span>
      <span className="font-semibold italic">{o.id}</span>
      <span className="min-w-0">
        <span className="block truncate">{objectTitle(o)}</span>
        <span className="block text-sm text-ink-2">
          {o.kind}
          {!o.inside && " · at the edge"}
        </span>
      </span>
      <span className="text-right text-sm text-ink-2">
        mag <span className="text-base text-ink">{mag(o.mag)}</span>
      </span>
    </button>
  );
}
