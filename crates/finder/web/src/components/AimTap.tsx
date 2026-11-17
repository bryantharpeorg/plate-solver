import { useEffect, useRef, useState } from "react";
import { resetAim, setAim, type LiveReport, type ObservingSession } from "../api";
import { boxToFrame, rotationOf } from "../rotation";
import { rotatedPhoto } from "./LivePreview";

export function AimTap({ session, live }: { session: ObservingSession; live: LiveReport | null }) {
  const rotation = rotationOf(session);
  const [aim, setPoint] = useState(session.aim);
  const [preview, setPreview] = useState<{ id: number; width: number; height: number } | null>(null);
  const [loaded, setLoaded] = useState<number | null>(null);
  const [unavailable, setUnavailable] = useState(false);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);
  const image = useRef<HTMLImageElement>(null);
  useEffect(() => setPoint(session.aim), [session.aim?.x, session.aim?.y]);
  useEffect(() => {
    if (live?.preview_seq == null) return;
    // Keep the id AND full-frame dimensions of the photograph actually displayed.
    // Later reports without photographs must not change a tap's coordinate system.
    setPreview({ id: live.preview_seq, width: live.report.width, height: live.report.height });
    setLoaded(null);
    setUnavailable(false);
  }, [live]);

  const write = async (action: () => Promise<ObservingSession>, success: string) => {
    setBusy(true);
    setFailed(false);
    setMessage(null);
    try {
      const next = await action();
      setPoint(next.aim);
      setMessage(success);
    } catch (error) {
      setFailed(true);
      setMessage(error instanceof Error ? error.message : "The aim point could not be saved.");
    } finally {
      setBusy(false);
    }
  };

  // The turned photo, like the live preview's, or nothing when the rotation is 0.
  const turned = rotation !== 0 && preview ? rotatedPhoto(preview.width, preview.height, rotation) : null;

  return (
    <section aria-labelledby="aim-point-title" className="space-y-3">
      <h2 id="aim-point-title" className="section-title">
        Aim point
      </h2>
      <p className="quiet">
        Centre a star in the eyepiece, then tap that star on the live preview. Reset the aim point if you move the
        finder camera.
      </p>
      <p>{aim ? `Aim point: (${aim.x}, ${aim.y}) px` : "Aim point: frame centre"}</p>
      {preview ? (
        <button
          type="button"
          aria-label="Set aim point on live preview"
          disabled={busy}
          className="block w-full cursor-crosshair border border-ink bg-photo"
          style={turned?.box}
          onClick={(event) => {
            if (busy || loaded !== preview.id || !image.current) return;
            const rect = image.current.getBoundingClientRect();
            if (rect.width <= 0 || rect.height <= 0) return;
            // The frame pixel under the tap, whatever way the photo is turned: pixels sent
            // to the server stay in frame coordinates.
            const [x, y] = boxToFrame(
              preview.width,
              preview.height,
              rotation,
              rect,
              event.clientX - rect.left,
              event.clientY - rect.top,
            );
            if (x < 0 || y < 0 || x >= preview.width || y >= preview.height) return;
            void write(() => setAim(preview.id, x, y), "Aim point set.");
          }}
        >
          <img
            key={preview.id}
            ref={image}
            src={`/api/live/preview.jpg?seq=${preview.id}`}
            alt={`Live preview, frame ${preview.id}`}
            width={preview.width}
            height={preview.height}
            className="photo block h-auto w-full"
            style={turned?.photo}
            onLoad={() => {
              setLoaded(preview.id);
              setUnavailable(false);
            }}
            onError={() => {
              setLoaded(null);
              setUnavailable(true);
            }}
          />
        </button>
      ) : (
        <p className="quiet">Waiting for a live preview.</p>
      )}
      {unavailable && <p className="quiet">Preview unavailable. Wait for the next live preview before tapping.</p>}
      {message && (
        <p role={failed ? "alert" : "status"} className="text-ink">
          {message}
        </p>
      )}
      <button
        type="button"
        className="button"
        disabled={busy || !aim}
        onClick={() => void write(resetAim, "Aim point reset.")}
      >
        Reset aim point
      </button>
    </section>
  );
}
