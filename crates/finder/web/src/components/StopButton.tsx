import { useEffect, useRef, useState } from "react";
import { postStop, type GoToProgress, type SessionMount, type SessionTrack, type SessionMotion } from "../api";
import { gotoRunning } from "./GoToControl";

export function mountMayMove(
  mount?: SessionMount,
  goto?: GoToProgress,
  track?: SessionTrack,
  motion?: SessionMotion,
): boolean {
  const tracking = track?.state === "armed" || track?.state === "tracking" || track?.state === "keyhole_wait";
  return (
    motion !== undefined ||
    (mount !== undefined && (mount.moving || gotoRunning(goto) || (mount.connected && tracking)))
  );
}

export function StopButton({
  mount,
  goto,
  track,
  motion,
  onChange,
}: {
  mount?: SessionMount;
  goto?: GoToProgress;
  track?: SessionTrack;
  motion?: SessionMotion;
  onChange?: () => void;
}) {
  const [error, setError] = useState<string | null>(null);
  const [waiting, setWaiting] = useState<{ mount: SessionMount | undefined; confirmed: boolean } | null>(null);
  const latestMount = useRef(mount);
  latestMount.current = mount;
  useEffect(() => {
    // Only a session read received after the reply can prove rest.
    if (waiting && mount !== waiting.mount && mount?.connected === true && mount.moving === false) {
      setWaiting(null);
      setError(null);
    }
  }, [mount, waiting]);
  if (!waiting && !mountMayMove(mount, goto, track, motion)) return null;
  const stop = async () => {
    setError(null);
    try {
      const reply = await postStop();
      const confirmed = reply.acknowledged !== false;
      setWaiting({ mount: latestMount.current, confirmed });
      if (!confirmed) {
        setError(
          "The mount did not confirm Stop and may still be moving. Press Stop again. If it keeps moving, switch off the mount's power.",
        );
      }
      onChange?.();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Stop did not reach the mount. Try again.");
      setWaiting({ mount: latestMount.current, confirmed: false });
    }
  };
  const note =
    waiting?.confirmed && mount?.connected !== true
      ? "Stop sent. Waiting for the mount to report that it has stopped."
      : null;
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
      {(error || note) && (
        <p
          role={error ? "alert" : undefined}
          className="absolute right-0 top-full w-64 border border-ink bg-paper p-2 text-sm text-ink"
        >
          {error || note}
        </p>
      )}
    </div>
  );
}
