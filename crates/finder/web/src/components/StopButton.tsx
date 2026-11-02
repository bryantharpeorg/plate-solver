import { useState } from "react";
import { postMotion, type GoToProgress, type SessionMount } from "../api";
import { gotoRunning } from "./GoToControl";

export function mountMayMove(mount?: SessionMount, goto?: GoToProgress): boolean {
  return mount !== undefined && (mount.moving || gotoRunning(goto));
}

export function StopButton({
  mount,
  goto,
  onChange,
}: {
  mount?: SessionMount;
  goto?: GoToProgress;
  onChange?: () => void;
}) {
  const [error, setError] = useState<string | null>(null);
  if (!mountMayMove(mount, goto)) return null;
  const stop = async () => {
    setError(null);
    try {
      await postMotion("/api/stop");
      onChange?.();
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
