// Guidance is the console's session answer. The page formats it, without sky maths.
import type { Guidance } from "../api";
import type { ObservingSession } from "../useSession";
import { TARGET_CLASS_LABEL } from "./TargetSheet";
import { GoToControl } from "./GoToControl";
import { TrackControl } from "./TrackControl";
import { McpNote } from "./McpNote";

export function GuidanceBlock({
  session,
  onSetup,
  stale,
  onChange,
}: {
  session: ObservingSession;
  onSetup?: () => void;
  stale?: boolean;
  onChange?: () => void;
}) {
  const { target, guidance, pointing } = session;
  if (!target) return null;
  const age = pointing && session.time !== undefined ? Math.max(0, Math.floor(session.time - pointing.time)) : null;
  const ageText =
    age === null
      ? null
      : age < 60
        ? `${age} s`
        : age < 3600
          ? `${Math.floor(age / 60)} min`
          : `${Math.floor(age / 3600)} h ${Math.floor((age % 3600) / 60)} min`;
  return (
    <section aria-labelledby="guidance-title" className="space-y-2 border-y border-ink py-3 text-ink">
      <h2 id="guidance-title" className="section-title">
        Target <span className="italic">{target.messier ?? target.id}</span>
        {target.common_names.length > 0 && ` · ${target.common_names.join(", ")}`}
      </h2>
      <p className="text-sm text-ink-2">
        {target.kind}
        {guidance && ` · ${TARGET_CLASS_LABEL[guidance.class]}`}
      </p>
      {stale || guidance?.stale ? (
        <>
          <p className="font-semibold">Pointing stale</p>
          <p className="quiet">Waiting for a live solve.</p>
        </>
      ) : !guidance ? (
        <p className="quiet">Loading guidance…</p>
      ) : guidance.class === "sun_guard" ? (
        <p>The console will not point within 15° of the Sun.</p>
      ) : guidance.class === "no_site" ? (
        <p>
          A site is needed.{" "}
          <a
            href="/setup"
            className="link"
            onClick={(e) => {
              if (!onSetup || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
              e.preventDefault();
              onSetup();
            }}
          >
            Set your site
          </a>{" "}
          in Setup.
        </p>
      ) : guidance.class === "below_horizon" || guidance.class === "too_low" ? (
        <>
          <p>
            {target.messier ?? target.id} is{" "}
            {guidance.class === "below_horizon" ? "below the horizon" : "below the minimum altitude"}.
          </p>
          {guidance.never && <p>It never reaches {session.min_alt_deg ?? 15}° at this site.</p>}
          <EventTimes guidance={guidance} />
        </>
      ) : pointing && guidance.turn_deg !== undefined && guidance.tilt_deg !== undefined ? (
        <>
          <p className="font-semibold">
            turn {guidance.turn_deg >= 0 ? "clockwise" : "anticlockwise"} {Math.round(Math.abs(guidance.turn_deg))}° ·
            tilt {guidance.tilt_deg >= 0 ? "up" : "down"} {Math.round(Math.abs(guidance.tilt_deg))}°
          </p>
          {ageText && <p className="quiet">Pointing {ageText} ago.</p>}
        </>
      ) : (
        <p>Solve a frame to get directions from the latest pointing.</p>
      )}
      <GoToControl session={session} onChange={onChange} />
      <TrackControl session={session} onChange={onChange} />
      <McpNote motion={session.motion} />
    </section>
  );
}

function EventTimes({ guidance }: { guidance: Guidance }) {
  const events = [
    ["Rises", guidance.rise],
    ["Transits", guidance.transit],
    ["Sets", guidance.set],
  ] as const;
  return (
    <p className="flex flex-wrap gap-x-4 gap-y-1 text-sm">
      {events.map(([label, value]) =>
        value === undefined ? null : (
          <span key={label}>
            {label}{" "}
            <time dateTime={new Date(value * 1000).toISOString()}>
              {new Date(value * 1000).toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" })}
            </time>
          </span>
        ),
      )}
    </p>
  );
}
