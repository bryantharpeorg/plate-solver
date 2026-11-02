import { useState } from "react";
import { postMotion, type GoToProgress, type SessionMount } from "../api";

export function mountMayMove(mount?: SessionMount, goto?: GoToProgress): boolean {
  return (
    mount !== undefined &&
    (mount.moving || goto?.state === "slewing" || goto?.state === "solving" || goto?.state === "correcting")
  );
}

export function StopButton({ mount, goto }: { mount?: SessionMount; goto?: GoToProgress }) {
  const [error, setError] = useState<string | null>(null);
  if (!mountMayMove(mount, goto)) return null;
  const stop = async () => {
    setError(null);
    try {
      await postMotion("/api/stop");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Stop did not reach the mount. Try again.");
    }
  };
  return (
    <div className="relative shrink-0">
      <button
        type="button"
        className="button border-2 font-semibold"
        style={{ minHeight: 44 }}
        onClick={() => void stop()}
      >
        Stop
      </button>
      {error && (
        <p role="alert" className="absolute right-0 top-full w-64 border border-ink bg-paper p-2 text-sm text-ink">
          {error}
        </p>
      )}
    </div>
  );
}
