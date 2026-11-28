// The nudge arrows (055): one press moves the telescope one step on the sky, in the view
// the observer picked. The console answers each press and reports its move in the session;
// the words here follow that report and nothing else.
import { useState } from "react";
import { postNudge, type NudgeDirection, type NudgeStep, type NudgeView, type ObservingSession } from "../api";

const VIEW_WORDS: Record<NudgeView, string> = {
  mount: "the mount",
  finder_camera: "the finder camera",
  imaging_camera: "the imaging camera",
};
const STEP_WORDS: Record<NudgeStep, { label: string; way: string }> = {
  fine: { label: "Fine 30″", way: "30″" },
  coarse: { label: "Coarse 4′", way: "4′" },
};
const GLYPHS: Record<NudgeDirection, string> = { up: "↑", right: "→", down: "↓", left: "←" };

/** A press the page made: its way in words, whether Track made it, and the session it was made from. */
interface Press {
  way: string;
  track: boolean;
  over: ObservingSession;
}

/** FR-017's one rule for the status line, read from the session after the press. */
function statusOf(press: Press, session: ObservingSession): string {
  const move = session.axis_move;
  if (session === press.over || (move?.tool === "nudge" && move.state === "moving")) {
    return `Nudging ${press.way}…`;
  }
  if (move?.tool === "nudge" && move.state === "halted") return `Nudge stopped: ${move.reason ?? "halted"}`;
  return `Nudged ${press.way}${press.track ? " · Track holds the new place" : ""}`;
}

export function NudgeBlock({
  session,
  onChange,
  first = "mount",
}: {
  session: ObservingSession;
  onChange?: () => void;
  /** The view the arrows follow first, when the session offers it. */
  first?: NudgeView;
}) {
  const [view, setView] = useState<NudgeView>(first);
  const [step, setStep] = useState<NudgeStep>("fine");
  const [pending, setPending] = useState(false);
  const [press, setPress] = useState<Press | null>(null);
  const [refusal, setRefusal] = useState<string | null>(null);
  if (!session.mount?.connected) return null;

  const offered: NudgeView[] = session.imaging_camera
    ? ["mount", "finder_camera", "imaging_camera"]
    : ["mount", "finder_camera"];
  const chosen = offered.includes(view) ? view : "mount";

  const nudge = async (direction: NudgeDirection) => {
    setPending(true);
    setRefusal(null);
    setPress(null);
    try {
      const answer = await postNudge(direction, step, chosen);
      setPress({ way: `${direction} ${STEP_WORDS[step].way}`, track: answer.moved === "track", over: session });
      onChange?.();
    } catch (e) {
      setRefusal(e instanceof Error ? e.message : "The telescope could not be nudged. Try again.");
    } finally {
      setPending(false);
    }
  };

  const arrow = (direction: NudgeDirection) => (
    <button
      type="button"
      aria-label={`Nudge ${direction}`}
      className="button min-h-11 min-w-11 text-lg"
      disabled={pending}
      onClick={() => void nudge(direction)}
    >
      {GLYPHS[direction]}
    </button>
  );

  return (
    <section aria-label="Nudge" className="space-y-3">
      <label className="block space-y-1">
        <span className="text-sm font-semibold text-ink-2">Arrows follow</span>
        <select
          aria-label="Arrows follow"
          value={chosen}
          onChange={(e) => setView(e.target.value as NudgeView)}
          className="h-11 w-full min-w-0 border border-ink bg-transparent px-3 text-base outline-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink"
        >
          {offered.map((option) => (
            <option key={option} value={option}>
              {VIEW_WORDS[option]}
            </option>
          ))}
        </select>
      </label>
      <div role="group" aria-label="Step" className="flex flex-wrap gap-3">
        {(["fine", "coarse"] as const).map((option) => (
          <button
            key={option}
            type="button"
            aria-pressed={step === option}
            className={step === option ? "button button-primary" : "button"}
            onClick={() => setStep(option)}
          >
            {STEP_WORDS[option].label}
          </button>
        ))}
      </div>
      <div className="grid w-max grid-cols-3 gap-2">
        <span aria-hidden="true" />
        {arrow("up")}
        <span aria-hidden="true" />
        {arrow("left")}
        <span aria-hidden="true" />
        {arrow("right")}
        <span aria-hidden="true" />
        {arrow("down")}
        <span aria-hidden="true" />
      </div>
      {press && <p role="status">{statusOf(press, session)}</p>}
      {refusal && <p role="alert">{refusal}</p>}
    </section>
  );
}
