import { useState } from "react";
import { postRecord, postRecordRaw, type ObservingSession } from "../api";

export function RecordingSection({ session, onChange }: { session: ObservingSession; onChange?: () => void }) {
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  if (!session.node) return null;
  const { recording } = session;
  const raw = recording?.raw;
  const rawPending = raw !== undefined && raw.saved < raw.wanted && raw.error === undefined;
  const write = async (action: () => Promise<ObservingSession>) => {
    setPending(true);
    setError(null);
    try {
      await action();
      onChange?.();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Recording could not be changed. Try again.");
    } finally {
      setPending(false);
    }
  };

  return (
    <section aria-labelledby="recording-title" className="space-y-3">
      <h2 id="recording-title" className="section-title">
        Recording
      </h2>
      {!recording && (
        <p className="quiet">
          Saves each star frame's stars, every solve and the previews to the console's recordings folder, to test with
          later.
        </p>
      )}
      {recording?.on ? (
        <>
          <p role="status">
            <span className="block">{`Recording to ${recording.folder} · ${recording.frames} frames · ${recording.solves} solves`}</span>
            {raw && raw.error === undefined && (
              <span className="block">
                Raw frames: {raw.saved} of {raw.wanted} saved
              </span>
            )}
          </p>
          <div className="flex flex-wrap gap-3">
            <button
              type="button"
              className="button"
              disabled={pending}
              onClick={() => void write(() => postRecord(false))}
            >
              Stop recording
            </button>
            <button
              type="button"
              className="button"
              disabled={pending || rawPending}
              onClick={() => void write(() => postRecordRaw(3))}
            >
              Save 3 raw frames
            </button>
          </div>
          {raw?.error !== undefined && <p role="alert">Raw frames stopped: {raw.error}.</p>}
        </>
      ) : (
        <>
          {recording && (
            <p role="status">
              {`Saved ${recording.folder}: ${recording.frames} frames, ${recording.solves} solves.`}
              {recording.ended_reason === "limit" && " It stopped at its frame limit."}
              {recording.ended_reason === "node_restarted" &&
                " It stopped when the scope node restarted. Start a new recording."}
            </p>
          )}
          {recording?.ended_reason === "write_failed" && (
            <p role="alert">Recording stopped: the console could not write to its folder.</p>
          )}
          <button
            type="button"
            className="button button-primary"
            disabled={pending}
            onClick={() => void write(() => postRecord(true))}
          >
            Start recording
          </button>
        </>
      )}
      {error && <p role="alert">{error}</p>}
    </section>
  );
}
