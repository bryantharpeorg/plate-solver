import { useState } from "react";
import { postMotion, resetModel, type ObservingSession } from "../api";
import { mountMayMove } from "./StopButton";

export function MountSection({ session, onChange }: { session: ObservingSession; onChange?: () => void }) {
  const [pending, setPending] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const { mount, model, node } = session;
  const moving = mountMayMove(mount, session.goto, undefined, session.motion);
  const request = async (action: () => Promise<void>, success: string) => {
    setPending(true);
    setMessage(null);
    setError(null);
    try {
      await action();
      setMessage(success);
      onChange?.();
    } catch (e) {
      setError(e instanceof Error ? e.message : "The model request failed. Try again.");
    } finally {
      setPending(false);
    }
  };
  return (
    <section aria-labelledby="setup-mount-title" className="space-y-3">
      <h2 id="setup-mount-title" className="section-title">
        Mount and pointing model
      </h2>
      <div className="space-y-3 border-y border-rule py-4">
        <p>
          {mount?.connected ? "Connected" : "Disconnected"}
          {node?.address && ` · ${node.address}`}
          {mount && ` · ${mount.board ?? "Board not reported"}`}
          {mount?.firmware && ` · firmware ${mount.firmware}`}
        </p>
        <p>
          {model?.base_tilt
            ? `base tilt ${model.base_tilt.deg.toFixed(1)}° toward ${Math.round(model.base_tilt.toward_deg) % 360}°`
            : "Base tilt needs two syncs at least 90° apart."}
        </p>
        <p>
          {model?.syncs ?? 0} {(model?.syncs ?? 0) === 1 ? "sync" : "syncs"}
        </p>
        {model && <p>RMS residual {model.rms_arcsec.toFixed(1)}″</p>}
        {model && model.needs > 0 && (
          <p className="quiet">Fixed terms need {model.needs} more syncs spread across the sky.</p>
        )}
        <button
          type="button"
          className="button button-primary"
          disabled={pending || moving || !mount?.connected || !model?.syncs}
          onClick={() => void request(() => postMotion("/api/model/second-sync"), "Second sync requested.")}
        >
          Second sync
        </button>
        <div className="flex flex-wrap gap-3">
          <button
            type="button"
            className="button"
            disabled={pending || moving}
            onClick={() => void request(() => resetModel("night"), "Night's model reset.")}
          >
            Reset night's model
          </button>
          <button
            type="button"
            className="button"
            disabled={pending || moving}
            onClick={() => void request(() => resetModel("fixed"), "Fixed terms reset.")}
          >
            Reset fixed terms
          </button>
        </div>
        {message && <p role="status">{message}</p>}
        {error && <p role="alert">{error}</p>}
      </div>
    </section>
  );
}
