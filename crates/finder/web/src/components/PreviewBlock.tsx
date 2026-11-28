// The Preview frames block (058 US4): one preview frame after another at the settings in
// force, each shown as its crop and never saved. It asks for the next as soon as the last
// is shown, pauses while the page is hidden, and ends on Stop, a refusal, a request the
// console does not answer, or a frame that ends other than shown (FR-013, FR-014).
import { useEffect, useRef, useState } from "react";
import { ApiError, getImaging, postImagingPreview, type ImagingFrame, type ImagingView } from "../api";

/** How often the block reads the console while its frame is in flight (FR-013). */
export const POLL_MS = 250;

const WORDS =
  "Imaging frames one after another at the settings above, shown and never saved, to focus or frame by eye. " +
  "The whole frame shows under Imaging frame, and its brightest part here, one pixel a cell.";

const UNANSWERED = "the console did not answer";

// The block asks for frames (`on`), is stopping while its frame in flight ends, or is off.
type Mode = "off" | "on" | "stopping";

const sleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

export function PreviewBlock() {
  const [mode, setMode] = useState<Mode>("off");
  const [visible, setVisible] = useState(() => document.visibilityState !== "hidden");
  const [shown, setShown] = useState<ImagingFrame | null>(null);
  // Seconds between the last two frames' ends seen by the page; kept across a pause.
  const [every, setEvery] = useState<number | null>(null);
  const [alert, setAlert] = useState<string | null>(null);
  // The last read failed: the console has not answered.
  const [lost, setLost] = useState(false);
  // These outlive a pause and a stop, so the next run carries on with the frame in flight.
  const pending = useRef<number | null>(null); // the block's frame in flight
  const asking = useRef<Promise<void> | null>(null); // the POST being answered
  const refused = useRef<string | null>(null); // a POST that failed, not yet reported

  useEffect(() => {
    const onChange = () => setVisible(document.visibilityState !== "hidden");
    document.addEventListener("visibilitychange", onChange);
    return () => document.removeEventListener("visibilitychange", onChange);
  }, []);

  useEffect(() => {
    if (mode === "off" || !visible) return;
    let stopped = false;

    // Ask for one frame. Its answer lands in `pending`, or in `refused` when it failed.
    const ask = () => {
      asking.current = postImagingPreview().then(
        (view) => {
          if (view.frame) pending.current = view.frame.id;
          else refused.current = UNANSWERED;
        },
        (error: unknown) => {
          refused.current = error instanceof ApiError ? error.message : UNANSWERED;
        },
      );
    };

    const end = (message: string) => {
      setAlert(`Preview frames stopped: ${message}.`);
      setLost(false);
      setMode("off");
    };

    // One run of the block. A run that is stopped (hidden, stopped, unmounted) returns at
    // its next await without touching the state, and the next run takes over `pending`.
    const run = async () => {
      let last: number | null = null; // when this run last showed a frame, by performance.now()
      for (;;) {
        if (asking.current) await asking.current;
        if (stopped) return;
        if (refused.current != null) {
          const message = refused.current;
          refused.current = null;
          end(message);
          return;
        }
        if (pending.current == null) {
          if (mode === "stopping") {
            setMode("off");
            return;
          }
          ask();
          continue;
        }

        // Read the frame in flight, every POLL_MS, until it has ended.
        await sleep(POLL_MS);
        if (stopped) return;
        const id = pending.current;
        let view: ImagingView;
        try {
          view = await getImaging();
        } catch {
          if (stopped) return;
          setLost(true);
          continue;
        }
        if (stopped) return;
        setLost(false);
        const frame = view.frame;
        if (frame != null && frame.id === id && (frame.state === "exposing" || frame.state === "receiving")) continue;

        pending.current = null;
        // Overtaken by another frame, or none in the view: one more POST names the holder.
        if (frame == null || frame.id !== id) continue;
        if (frame.state === "shown") {
          const now = performance.now();
          if (last != null) setEvery((now - last) / 1000);
          last = now;
          setShown(frame);
          continue;
        }
        end(frame.state === "aborted" ? "the exposure was aborted" : (frame.error ?? frame.state));
        return;
      }
    };

    void run();
    return () => {
      stopped = true;
    };
  }, [mode, visible]);

  const press = () => {
    if (mode === "on") {
      setMode("stopping");
      return;
    }
    setAlert(null);
    setShown(null);
    setEvery(null);
    setLost(false);
    setMode("on");
  };

  const statusWords = lost
    ? "Waiting for the console"
    : mode === "stopping"
      ? "Stopping: the last preview frame is still in flight"
      : shown == null
        ? "Waiting for the first preview frame"
        : `Preview frame ${shown.id} · ${every == null ? "" : `every ${every.toFixed(1)} s · `}not saved`;

  return (
    <section aria-labelledby="imaging-preview-title" className="space-y-3">
      <h2 id="imaging-preview-title" className="section-title">
        Preview frames
      </h2>
      <div className="space-y-3 border-y border-rule py-4">
        <p className="quiet">{WORDS}</p>
        <button type="button" className={mode === "on" ? "button" : "button button-primary"} onClick={press}>
          {mode === "on" ? "Stop preview frames" : "Start preview frames"}
        </button>
        {mode !== "off" && <p role="status">{statusWords}</p>}
        {alert && <p role="alert">{alert}</p>}
        {shown?.crop != null && (
          <div className="border border-ink bg-photo">
            <img
              src={shown.crop}
              alt={`Crop of preview frame ${shown.id} around its brightest part`}
              className="photo block h-auto w-full"
            />
          </div>
        )}
      </div>
    </section>
  );
}
