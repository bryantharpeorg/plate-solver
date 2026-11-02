import { useEffect, useId, useState } from "react";
import { postMotion, type GoToProgress, type ObservingSession } from "../api";

export function gotoRunning(goto?: GoToProgress): boolean {
  return goto?.state === "slewing" || goto?.state === "solving" || goto?.state === "correcting";
}

function progressText(goto: GoToProgress): string {
  const correction = goto.correction > 0 ? `Correction ${goto.correction} · ` : "";
  const error = goto.error_arcsec === null ? "" : ` · ${goto.error_arcsec.toFixed(1)}″`;
  const phase = {
    slewing: "Slewing…",
    solving: "Solving…",
    correcting: "Planning correction…",
    done: "On target",
    failed: "GoTo failed",
    halted: "Halted",
  }[goto.state];
  return `${correction}${phase}${error}${goto.reason ? ` · ${goto.reason}` : ""}`;
}

export function GoToControl({ session, onChange }: { session: ObservingSession; onChange?: () => void }) {
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const reasonId = useId();
  useEffect(
    () => setError(null),
    [session.target?.id, session.goto_refusal, session.goto?.state, session.goto?.correction],
  );
  if (!session.mount) return null;
  const running = gotoRunning(session.goto);
  const refusal = session.goto_refusal ?? (!session.mount.connected ? "No mount is connected." : null);
  const start = async () => {
    setPending(true);
    setError(null);
    try {
      await postMotion("/api/goto");
      onChange?.();
    } catch (e) {
      setError(e instanceof Error ? e.message : "GoTo could not be started. Try again.");
    } finally {
      setPending(false);
    }
  };
  return (
    <div className="space-y-2">
      <div className="flex flex-wrap items-center gap-x-3 gap-y-2">
        <button
          type="button"
          className="button button-primary"
          disabled={pending || running || refusal !== null}
          aria-describedby={refusal ? reasonId : undefined}
          onClick={() => void start()}
        >
          GoTo
        </button>
        {session.goto && <p role="status">{progressText(session.goto)}</p>}
      </div>
      {refusal && (
        <p id={reasonId} className="text-sm text-ink-2">
          {refusal}
        </p>
      )}
      {error && <p role="alert">{error}</p>}
    </div>
  );
}
