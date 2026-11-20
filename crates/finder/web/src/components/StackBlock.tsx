// The Imaging page's live stack (035).
import { useLayoutEffect, useState } from "react";
import { deleteImagingStack, postImagingStack, type ImagingView, type StackDropped } from "../api";
import { useColourHold } from "../useColourHold";

const DROPPED: [keyof StackDropped, string][] = [
  ["not_solved", "not solved"],
  ["too_few_stars", "too few stars"],
  ["solver_failed", "solver failed"],
  ["soft", "soft"],
  ["off_field", "off the field"],
  ["solve_differs", "solved differently"],
  ["settings", "settings changed"],
  ["frame_failed", "not received"],
  ["unreadable", "unreadable"],
  ["behind", "behind"],
  ["unprocessed", "unprocessed"],
  ["not_steady", "while the mount was not steady"],
];
const MISSING = { dark: "a master dark", flat: "a master flat", bad_pixels: "a bad-pixel map" };

function exposure(seconds: number): string {
  const whole = Math.floor(seconds);
  return whole < 60 ? `${whole} s` : `${Math.floor(whole / 60)} min ${whole % 60} s`;
}

export function StackBlock({ view, night, onChange }: { view: ImagingView; night: boolean; onChange: () => void }) {
  const stack = view.stack;
  const { held, bind } = useColourHold(night);
  const gains = stack?.options.gains;
  const stretch = stack?.options.stretch ?? 10;
  const red = gains?.[0] ?? 1;
  const green = gains?.[1] ?? 1;
  const blue = gains?.[2] ?? 1;
  const [draft, setDraft] = useState([String(red), String(green), String(blue), String(stretch)]);
  const [pending, setPending] = useState(false);
  const [refusal, setRefusal] = useState<string | null>(null);

  // Only saved colour values seed the fields. A new view object from each poll
  // must leave an observer's unsent edit alone.
  useLayoutEffect(() => {
    setDraft([String(red), String(green), String(blue), String(stretch)]);
  }, [red, green, blue, stretch]);

  const send = async (request: () => Promise<ImagingView>) => {
    setPending(true);
    setRefusal(null);
    try {
      await request();
    } catch (e) {
      setRefusal(e instanceof Error ? e.message : "The stack could not be reached.");
      return;
    } finally {
      setPending(false);
    }
    onChange();
  };
  const apply = () =>
    postImagingStack({ gains: [Number(draft[0]), Number(draft[1]), Number(draft[2])], stretch: Number(draft[3]) });
  const running = stack?.state === "stacking";
  const dropped = stack ? DROPPED.reduce((sum, [key]) => sum + (stack.dropped[key] ?? 0), 0) : 0;
  const parts = stack
    ? DROPPED.filter(([key]) => (stack.dropped[key] ?? 0) > 0)
        .map(([key, words]) => `${stack.dropped[key]} ${words}`)
        .join(", ")
    : "";

  return (
    <section aria-labelledby="stack-title" className="space-y-3">
      <h2 id="stack-title" className="section-title">
        Stack
      </h2>
      {!stack && (
        <p className="quiet">Takes subs one after another and stacks them into one picture. Turn Track on first.</p>
      )}
      {running && stack && (
        <>
          <p role="status">{`Stacking · ${stack.stacked} of ${stack.taken} subs stacked · ${exposure(stack.exposure_s)} of exposure`}</p>
          {dropped > 0 && <p className="quiet">{`Dropped ${dropped}: ${parts}`}</p>}
          {!!stack.missing?.length && (
            <p className="quiet">{`Stacked without ${stack.missing.map((key) => MISSING[key]).join(", ")}.`}</p>
          )}
        </>
      )}
      {stack?.state === "ended" && (
        <>
          <p role="status">{`Stack ${stack.id}: ${stack.stacked} subs stacked, ${dropped} dropped.${stack.files?.fits && stack.files.png ? ` Saved ${stack.files.fits} and stack.png.` : ""}`}</p>
          {stack.ended_reason === "camera" && <p role="alert">{`Stacking stopped: ${stack.error}.`}</p>}
          {stack.save_error && <p role="alert">{`The stack was not saved: ${stack.save_error}.`}</p>}
        </>
      )}
      <div className="space-y-3 border-y border-rule py-4">
        <div className="grid grid-cols-2 gap-3">
          {["Red gain", "Green gain", "Blue gain", "Stretch"].map((name, index) => (
            <label key={name} className="block space-y-1">
              <span className="text-sm font-semibold text-ink-2">{name}</span>
              <input
                type="number"
                inputMode="decimal"
                min={index === 3 ? 1 : 0.1}
                max={index === 3 ? 1000 : 10}
                step="any"
                value={draft[index]}
                onChange={(event) => setDraft(draft.map((value, i) => (i === index ? event.target.value : value)))}
                className="h-11 w-full min-w-0 border border-ink bg-transparent px-3 text-base outline-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink"
              />
            </label>
          ))}
        </div>
        <div className="flex flex-wrap gap-3">
          {running ? (
            <>
              <button type="button" className="button" disabled={pending} onClick={() => void send(deleteImagingStack)}>
                Stop stacking
              </button>
              <button type="button" className="button" disabled={pending} onClick={() => void send(apply)}>
                Apply colour
              </button>
            </>
          ) : (
            <button type="button" className="button button-primary" disabled={pending} onClick={() => void send(apply)}>
              Start stacking
            </button>
          )}
        </div>
        {refusal && <p role="alert">{refusal}</p>}
      </div>
      {stack?.preview && (
        <>
          <div className="border border-ink bg-photo">
            <img
              alt={`Stack of ${stack.stacked} subs`}
              src={stack.preview}
              className={night && held ? "block h-auto w-full" : "photo block h-auto w-full"}
            />
          </div>
          {night && (
            <>
              <button type="button" className="button" aria-pressed={held} {...bind}>
                Hold to see colour
              </button>
              <p className="quiet">
                The picture is red at night. Hold the control to see its colours for up to 10 seconds.
              </p>
            </>
          )}
        </>
      )}
    </section>
  );
}
