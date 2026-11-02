// Choose a frame: one of the server's sample frames, or a file from this device.

import { useEffect, useRef, useState } from "react";
import { getSamples, thumbUrl, type FrameSource, type Sample, type SolveSettings } from "../api";
import { Chevron, Upload } from "./Icons";

interface Props {
  open: boolean;
  keepTopBarActive?: boolean;
  current: string | null;
  settings: SolveSettings;
  onSettings: (settings: SolveSettings) => void;
  onClose: () => void;
  onPick: (source: FrameSource) => void;
}

export function FramePicker({ open, keepTopBarActive = false, current, settings, onSettings, onClose, onPick }: Props) {
  const dialog = useRef<HTMLDialogElement>(null);
  const file = useRef<HTMLInputElement>(null);

  const modal = useRef(false);
  useEffect(() => {
    const d = dialog.current;
    if (!d) return;
    // A native modal makes the whole top bar inert, regardless of z-index.
    // With a mount present, keep the sheet below the live Stop control instead.
    if (keepTopBarActive) {
      if (modal.current) d.close();
      modal.current = false;
      d.open = open;
    } else {
      if (open && !d.open) {
        d.showModal();
        modal.current = true;
      }
      if (!open && d.open) d.close();
    }
  }, [open, keepTopBarActive]);

  const pick = (source: FrameSource) => {
    onPick(source);
    onClose();
  };

  return (
    <dialog
      ref={dialog}
      className="sheet"
      style={
        keepTopBarActive
          ? { position: "fixed", inset: "4rem 0 0", zIndex: 20, maxHeight: "calc(100dvh - 4rem)" }
          : undefined
      }
      onKeyDown={(e) => {
        if (e.key === "Escape") onClose();
      }}
      aria-labelledby="picker-title"
      onClose={() => {
        if (!keepTopBarActive || !open) onClose();
      }}
    >
      <header className="flex items-center justify-between border-b border-rule px-4 py-3 sm:px-5">
        <h2 id="picker-title" className="section-title">
          Choose a frame
        </h2>
        <button type="button" className="button button-quiet" onClick={onClose}>
          Close
        </button>
      </header>
      <div className="space-y-4 overflow-y-auto px-4 py-4 sm:px-5" style={{ maxHeight: "calc(88dvh - 4rem)" }}>
        <button type="button" className="button button-primary w-full sm:w-auto" onClick={() => file.current?.click()}>
          <Upload size={18} />
          Open a frame from this device
        </button>
        <input
          ref={file}
          type="file"
          accept="image/jpeg,image/png,image/bmp,.pgm,.ppm,.pnm"
          hidden
          onChange={(e) => {
            const f = e.currentTarget.files?.[0];
            e.currentTarget.value = "";
            if (f) pick({ kind: "file", file: f });
          }}
        />
        <p className="quiet">JPEG, PNG, BMP or PNM, up to 32 MB.</p>
        <SolveOptions settings={settings} onSettings={onSettings} />
        <SampleGrid current={current} onPick={(name) => pick({ kind: "sample", name })} />
      </div>
    </dialog>
  );
}

/** The server's sample frames as tappable previews. */
export function SampleGrid({
  current,
  onPick,
  heading = "h3",
}: {
  current: string | null;
  onPick: (name: string) => void;
  /** The list's heading level where it sits: under the dialog's title, or the page's. */
  heading?: "h2" | "h3";
}) {
  const Heading = heading;
  const [samples, setSamples] = useState<Sample[] | null>(null);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    getSamples()
      .then(setSamples)
      .catch(() => setFailed(true));
  }, []);

  if (failed) return <p className="quiet">The sample frames could not be listed.</p>;
  if (samples && samples.length === 0)
    return <p className="quiet">This server has no sample frames. Start it with --samples to offer some.</p>;
  return (
    <div className="space-y-2">
      <Heading className="group-title">Sample frames</Heading>
      <ul className="grid grid-cols-[repeat(auto-fill,minmax(7.5rem,1fr))] gap-3">
        {(samples ?? Array.from({ length: 6 }, () => null)).map((s, i) =>
          s ? (
            <li key={s.name}>
              <button
                type="button"
                className="group block w-full text-left"
                aria-current={s.name === current ? "true" : undefined}
                onClick={() => onPick(s.name)}
              >
                <img
                  src={thumbUrl(s.name)}
                  alt=""
                  loading="lazy"
                  className="photo aspect-square w-full border border-rule object-cover group-hover:border-ink group-aria-[current=true]:border-2 group-aria-[current=true]:border-ink"
                />
                <span className="mt-1 block truncate text-sm group-aria-[current=true]:font-semibold">{s.label}</span>
              </button>
            </li>
          ) : (
            <li key={i} className="skeleton aspect-square" aria-hidden="true" />
          ),
        )}
      </ul>
    </div>
  );
}

/** Options for frames from other cameras: a field-of-view hint and the detection threshold. */
function SolveOptions({ settings, onSettings }: { settings: SolveSettings; onSettings: (s: SolveSettings) => void }) {
  const number = (value: string) =>
    value.trim() === "" || !Number.isFinite(Number(value)) ? undefined : Number(value);
  return (
    <details className="border-y border-rule py-3">
      <summary className="flex cursor-pointer items-center justify-between gap-2 font-semibold">
        Solve options
        <span className="disclosure text-ink-2">
          <Chevron size={18} />
        </span>
      </summary>
      <div className="mt-3 grid grid-cols-2 gap-3">
        <label className="block space-y-1">
          <span className="text-sm font-semibold text-ink-2">Field of view</span>
          <span className="flex items-center border border-ink">
            <input
              inputMode="decimal"
              defaultValue={settings.fov ?? ""}
              placeholder="unknown"
              onChange={(e) => onSettings({ ...settings, fov: number(e.target.value) })}
              className="h-11 w-full min-w-0 bg-transparent px-3 outline-none"
            />
            <span className="pr-3 text-sm text-ink-2">°</span>
          </span>
        </label>
        <label className="block space-y-1">
          <span className="text-sm font-semibold text-ink-2">Threshold</span>
          <span className="flex items-center border border-ink">
            <input
              inputMode="decimal"
              defaultValue={settings.sigma ?? 8}
              onChange={(e) => onSettings({ ...settings, sigma: number(e.target.value) })}
              className="h-11 w-full min-w-0 bg-transparent px-3 outline-none"
            />
            <span className="pr-3 text-sm text-ink-2">σ</span>
          </span>
        </label>
      </div>
      <p className="quiet mt-2 text-sm">
        Leave the field of view empty unless the database does not match your camera. Lower the threshold to find
        fainter stars.
      </p>
    </details>
  );
}
