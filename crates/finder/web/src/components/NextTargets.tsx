// What to look at next: objects like the selected one, ranked for the observing site, and
// objects just off the frame with which way to move.

import { useEffect, useState, type ReactNode } from "react";
import { getSimilar, type DeepSkyObject, type Family, type NearbyMark, type SimilarReport } from "../api";
import { compass, deg, mag, objectTitle } from "../format";
import type { Site } from "../settings";
import { Horizon } from "./Icons";
import { FamilyIcon } from "./Symbol";

interface Props {
  reference: DeepSkyObject | null;
  nearby: NearbyMark[];
  site: Site | null;
  onSetSite: () => void;
  minAlt?: number;
  /** The live views' rotation, so the off-frame arrows turn with the chart. */
  rotation?: number;
}

export function NextTargets({ reference, nearby, site, onSetSite, minAlt = 15, rotation = 0 }: Props) {
  if (!reference && nearby.length === 0) return null;
  return (
    <section aria-labelledby="next-title" className="space-y-6">
      <h2 id="next-title" className="section-title">
        What to look at next
      </h2>
      {reference && <Similar reference={reference} site={site} onSetSite={onSetSite} minAlt={minAlt} />}
      {nearby.length > 0 && (
        <div className="space-y-2">
          <h3 className="group-title">Just off the frame</h3>
          <ul className="divide-y divide-rule border-y border-rule">
            {nearby.map((o) => (
              <Row
                key={o.id}
                family={o.family}
                id={o.id}
                title={objectTitle(o)}
                detail={o.kind}
                where={
                  <span className="inline-flex items-center gap-1.5">
                    <DirectionArrow degrees={o.direction_deg} rotation={rotation} />
                    {deg(o.separation_deg)}
                  </span>
                }
              />
            ))}
          </ul>
        </div>
      )}
    </section>
  );
}

function Row({
  family,
  id,
  title,
  detail,
  where,
  dim = false,
}: {
  family: Family;
  id: string;
  title: string;
  detail: string;
  where: ReactNode;
  dim?: boolean;
}) {
  return (
    // A row for an object that is down keeps its text readable and shows the state in its
    // symbol, drawn in the faint ink, and in the horizon mark beside the minimum altitude.
    <li className={`grid grid-cols-[1.5rem_3.25rem_1fr_auto] items-center gap-x-2 py-2.5 ${dim ? "text-ink-2" : ""}`}>
      <span className={`justify-self-center ${dim ? "text-ink-3" : `icon-${family}`}`}>
        <FamilyIcon family={family} size={18} />
      </span>
      <span className="font-semibold italic">{id}</span>
      <span className="min-w-0">
        <span className="block truncate">{title}</span>
        <span className="block text-sm text-ink-2">{detail}</span>
      </span>
      <span className="text-right text-sm tabular-nums">{where}</span>
    </li>
  );
}

function Similar({
  reference,
  site,
  onSetSite,
  minAlt,
}: {
  reference: DeepSkyObject;
  site: Site | null;
  onSetSite: () => void;
  minAlt: number;
}) {
  const [report, setReport] = useState<SimilarReport | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let live = true;
    setFailed(false);
    getSimilar(reference.id, site)
      .then((r) => live && setReport(r))
      .catch(() => live && setFailed(true));
    return () => {
      live = false;
    };
  }, [reference.id, site, minAlt]);

  const fresh = report?.reference.id === reference.id ? report : null;
  return (
    <div className="space-y-2">
      <h3 className="group-title">
        Like {reference.id}
        {fresh && `: other ${fresh.family}`}
      </h3>
      {failed && <p className="quiet">The suggestions could not be loaded.</p>}
      {!fresh && !failed && (
        <div className="space-y-2" aria-hidden="true">
          <div className="skeleton h-11" />
          <div className="skeleton h-11" />
        </div>
      )}
      {fresh && fresh.suggestions.length === 0 && <p className="quiet">Nothing else like it in the Messier list.</p>}
      {fresh && fresh.suggestions.length > 0 && (
        <ul className="divide-y divide-rule border-y border-rule">
          {fresh.suggestions.map((s) => (
            <Row
              key={s.id}
              family={s.family}
              id={s.id}
              title={objectTitle(s)}
              detail={`${s.kind} · mag ${mag(s.mag)}`}
              dim={s.up === false}
              where={
                s.alt_deg === null ? (
                  `${deg(s.separation_deg)} away`
                ) : s.up ? (
                  `${Math.round(s.alt_deg)}° up ${compass(s.az_deg ?? 0)}`
                ) : (
                  <span className="inline-flex items-center gap-1">
                    <Horizon size={16} />
                    below {minAlt}°
                  </span>
                )
              }
            />
          ))}
        </ul>
      )}
      {!site && (
        <p className="quiet">
          <button type="button" className="link" onClick={onSetSite}>
            Set your site
          </button>{" "}
          to see which of these are up now.
        </p>
      )}
    </div>
  );
}

export function DirectionArrow({ degrees, rotation = 0 }: { degrees: number; rotation?: number }) {
  return (
    <svg
      viewBox="-8 -8 16 16"
      width="15"
      height="15"
      role="img"
      aria-label={`${Math.round(degrees)}° from up on the frame`}
    >
      {/* The arrow turns with the chart; its label keeps the frame's own bearing. */}
      <g transform={`rotate(${degrees - rotation})`}>
        <path
          d="M0 6V-5M-3.5 -1.5L0 -5L3.5 -1.5"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.6"
          strokeLinecap="round"
        />
      </g>
    </svg>
  );
}
