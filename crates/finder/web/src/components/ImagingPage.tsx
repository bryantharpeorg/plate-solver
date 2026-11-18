// The imaging camera the scope node relays: its sensor, its settings and one frame (031).
import { useLayoutEffect, useState } from "react";
import {
  postImagingAbort,
  postImagingFrame,
  putImagingSettings,
  type ImagingFrame,
  type ImagingSettings,
  type ImagingStatus,
} from "../api";
import { useColourHold } from "../useColourHold";
import { useImaging } from "../useImaging";

// The words of each state that is not ready (031 US7-S2).
const NOT_READY: Record<string, string> = {
  off: "The scope node is not connected.",
  no_link: "The scope node is not connected.",
  no_service: "This scope node has no imaging service. Rebuild it to match this console.",
  no_camera: "This scope node has no imaging camera.",
};

function sensorLine(status: ImagingStatus | undefined, settings: ImagingSettings): string {
  if (!status || status.temp_c == null) return "Sensor temperature not reported";
  const temp = status.temp_c.toFixed(1);
  if (!settings.cooler_on) return `Sensor ${temp} °C · cooler off`;
  const power = Math.round(status.cooler_power ?? 0);
  if (status.set_temp_c == null) return `Sensor ${temp} °C · cooler on at ${power}%`;
  return `Sensor ${temp} °C · cooler on at ${power}% · set ${status.set_temp_c.toFixed(1)} °C`;
}

function savedLine(frame: ImagingFrame): string {
  let line = `Saved ${frame.file ?? ""}`;
  if (frame.start != null && frame.received_at != null) {
    line += ` · in hand ${(frame.received_at - frame.start - frame.exposure_ms / 1000).toFixed(1)} s after the exposure`;
  }
  if (frame.node_file) line += ` · also at the scope node as ${frame.node_file}`;
  return line;
}

export function ImagingPage({ night, onBack }: { night: boolean; onBack: () => void }) {
  const { view, refresh } = useImaging();
  const { held, bind } = useColourHold(night);
  const camera = view?.state === "ready" ? view.camera : undefined;
  const settings = view?.state === "ready" ? view.settings : undefined;
  const status = view?.state === "ready" ? view.status : undefined;
  const frame = view?.state === "ready" ? view.frame : undefined;

  // Drafts start from the settings in force and follow them only when those
  // settings change, so they survive the one-second reads.
  const [exposure, setExposure] = useState("");
  const [gain, setGain] = useState("");
  const [offset, setOffset] = useState("");
  const [binning, setBinning] = useState(1);
  const [coolerOn, setCoolerOn] = useState(false);
  const [setTemp, setSetTemp] = useState("");
  const [forward, setForward] = useState("");
  const [saveAtNode, setSaveAtNode] = useState(false);
  const [settingsAlert, setSettingsAlert] = useState<string | null>(null);
  const [frameAlert, setFrameAlert] = useState<string | null>(null);
  const seed =
    settings != null
      ? JSON.stringify([
          settings.exposure_ms,
          settings.gain,
          settings.offset,
          settings.binning,
          settings.cooler_on,
          settings.set_temp_c,
          settings.forward_kib_per_s,
          settings.save_at_node,
        ])
      : null;
  // A layout effect, not a passive one: it runs inside the commit that first
  // shows the settings, so the drafts are seeded before any edit can be made
  // and no pending seed can overwrite one.
  useLayoutEffect(() => {
    if (settings == null) return;
    setExposure(String(settings.exposure_ms));
    setGain(settings.gain != null ? String(settings.gain) : "");
    setOffset(settings.offset != null ? String(settings.offset) : "");
    setBinning(settings.binning);
    setCoolerOn(settings.cooler_on);
    setSetTemp(settings.set_temp_c != null ? String(settings.set_temp_c) : "");
    setForward(settings.forward_kib_per_s != null ? String(settings.forward_kib_per_s) : "");
    setSaveAtNode(settings.save_at_node);
    // Only the seed: re-seed when the settings change, not on every read.
  }, [seed]);

  const save = async () => {
    if (camera == null || settings == null) return;
    setSettingsAlert(null);
    // One PUT, its keys in this order, each field's value only when it is shown
    // and, for set_temp_c, not empty (031 US7-S3).
    const edit = {
      exposure_ms: Number(exposure),
      ...(camera.gain ? { gain: Number(gain) } : {}),
      ...(camera.offset ? { offset: Number(offset) } : {}),
      binning,
      ...(camera.cooler ? { cooler_on: coolerOn } : {}),
      ...(camera.cooler && setTemp.trim() ? { set_temp_c: Number(setTemp) } : {}),
      forward_kib_per_s: forward.trim() ? Number(forward) : null,
      save_at_node: saveAtNode,
    };
    try {
      await putImagingSettings(edit);
    } catch (e) {
      setSettingsAlert(e instanceof Error ? e.message : "The imaging settings could not be saved.");
      return;
    }
    await refresh();
  };

  const take = async () => {
    setFrameAlert(null);
    try {
      await postImagingFrame();
    } catch (e) {
      setFrameAlert(e instanceof Error ? e.message : "The imaging frame could not be taken.");
      return;
    }
    await refresh();
  };

  const abort = async () => {
    setFrameAlert(null);
    try {
      await postImagingAbort();
    } catch (e) {
      setFrameAlert(e instanceof Error ? e.message : "The exposure could not be aborted.");
      return;
    }
    await refresh();
  };

  const busy = frame?.state === "exposing" || frame?.state === "receiving";
  return (
    <main className="mx-auto max-w-[32.5rem] space-y-6 px-4 pt-4 pb-16 sm:px-5 sm:pt-6">
      <header className="border border-ink p-[3px]">
        <div className="space-y-2 border border-ink px-4 py-3">
          <h1 className="chart-title text-[1.3rem]">Imaging</h1>
          <a
            href="/"
            className="link"
            onClick={(e) => {
              if (e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
              e.preventDefault();
              onBack();
            }}
          >
            Back to console
          </a>
        </div>
      </header>
      {view != null &&
        view.state !== "ready" &&
        (view.state === "error" ? (
          <p role="alert">{`The imaging camera is not answering: ${view.error ?? ""}.`}</p>
        ) : (
          <p role="status">{NOT_READY[view.state] ?? NOT_READY.off!}</p>
        ))}
      {camera != null && settings != null && (
        <>
          <section aria-labelledby="imaging-camera-title" className="space-y-3">
            <h2 id="imaging-camera-title" className="section-title">
              Camera
            </h2>
            <p className="quiet">{`${camera.model} · ${camera.width} × ${camera.height} px · ${camera.pixel_um} µm · ${camera.bits}-bit · ${camera.bayer ?? "mono"}`}</p>
            <p role="status">{sensorLine(status, settings)}</p>
          </section>

          <section aria-labelledby="imaging-settings-title" className="space-y-3">
            <h2 id="imaging-settings-title" className="section-title">
              Settings
            </h2>
            <form
              className="space-y-3 border-y border-rule py-4"
              onSubmit={(e) => {
                e.preventDefault();
                void save();
              }}
            >
              <div className="grid grid-cols-2 gap-3">
                <Field label="Exposure" unit="ms" value={exposure} onChange={setExposure} placeholder="1000" />
                {camera.gain && <Field label="Gain" unit="" value={gain} onChange={setGain} placeholder="120" />}
                {camera.offset && <Field label="Offset" unit="" value={offset} onChange={setOffset} placeholder="10" />}
                <label className="block space-y-1">
                  <span className="text-sm font-semibold text-ink-2">Binning</span>
                  <select
                    aria-label="Binning"
                    value={binning}
                    onChange={(e) => setBinning(Number(e.target.value))}
                    className="h-11 w-full min-w-0 border border-ink bg-transparent px-3 text-base outline-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink"
                  >
                    {camera.binnings.map((b) => (
                      <option key={b} value={b}>{`${b} × ${b}`}</option>
                    ))}
                  </select>
                </label>
                {camera.cooler && (
                  <>
                    <label className="flex min-h-11 items-center gap-3 font-semibold">
                      Cooler
                      <button
                        type="button"
                        role="switch"
                        aria-label="Cooler"
                        aria-checked={coolerOn}
                        className="switch"
                        onClick={() => setCoolerOn(!coolerOn)}
                      />
                    </label>
                    <Field label="Set temperature" unit="°C" value={setTemp} onChange={setSetTemp} placeholder="-10" />
                  </>
                )}
              </div>
              {camera.binnings.includes(2) && (
                <p className="quiet">2 × 2 is half resolution: a quarter of the bytes for each imaging frame.</p>
              )}
              <Field label="Forwarding limit" unit="KiB/s" value={forward} onChange={setForward} placeholder="" />
              <p className="quiet">Empty for no limit. From 64 to 1048576 KiB a second.</p>
              <label className="flex items-center gap-3">
                <input
                  type="checkbox"
                  aria-label="Also keep each imaging frame at the scope node"
                  checked={saveAtNode}
                  onChange={(e) => setSaveAtNode(e.target.checked)}
                  className="appearance-none size-5 border border-ink checked:bg-ink"
                />
                <span>Also keep each imaging frame at the scope node</span>
              </label>
              {settingsAlert && <p role="alert">{settingsAlert}</p>}
              <button type="submit" className="button button-primary">
                Save imaging settings
              </button>
            </form>
          </section>

          <section aria-labelledby="imaging-frame-title" className="space-y-3">
            <h2 id="imaging-frame-title" className="section-title">
              Imaging frame
            </h2>
            <div className="space-y-3 border-y border-rule py-4">
              <div className="flex flex-wrap gap-3">
                <button type="button" className="button button-primary" disabled={busy} onClick={() => void take()}>
                  Take imaging frame
                </button>
                {busy && (
                  <button type="button" className="button" onClick={() => void abort()}>
                    Abort exposure
                  </button>
                )}
              </div>
              {frame?.state === "exposing" && <p role="status">{`Exposing for ${frame.exposure_ms} ms`}</p>}
              {frame?.state === "receiving" && (
                <p role="status">{`Receiving ${frame.received ?? 0} of ${frame.bytes ?? 0} bytes`}</p>
              )}
              {frame?.state === "saved" && <p role="status">{savedLine(frame)}</p>}
              {frame?.state === "aborted" && <p role="status">Exposure aborted.</p>}
              {frame?.state === "failed" && <p role="alert">{`Imaging frame failed: ${frame.error ?? ""}.`}</p>}
              {frame?.state === "saved" && frame.node_save_error && (
                <p role="alert">{`The scope node kept no copy: ${frame.node_save_error}.`}</p>
              )}
              {frameAlert && <p role="alert">{frameAlert}</p>}
              {frame?.preview && (
                <div className="border border-ink bg-photo">
                  <img
                    src={frame.preview}
                    alt={`Imaging frame ${frame.id}`}
                    className={held ? "block h-auto w-full" : "photo block h-auto w-full"}
                  />
                </div>
              )}
              {night && frame?.preview && (
                <>
                  <button type="button" className="button" aria-pressed={held} {...bind}>
                    Hold to see colour
                  </button>
                  <p className="quiet">
                    The picture is red at night. Hold the control to see its colours for up to 10 seconds.
                  </p>
                </>
              )}
            </div>
          </section>
        </>
      )}
    </main>
  );
}

function Field({
  label,
  unit,
  value,
  onChange,
  placeholder,
}: {
  label: string;
  unit: string;
  value: string;
  onChange: (value: string) => void;
  placeholder: string;
}) {
  return (
    <label className="block space-y-1">
      <span className="text-sm font-semibold text-ink-2">{label}</span>
      <span className="flex items-center border border-ink focus-within:outline-2 focus-within:outline-offset-2 focus-within:outline-ink">
        <input
          aria-label={label}
          inputMode="decimal"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={placeholder}
          className="h-11 w-full min-w-0 bg-transparent px-3 text-base outline-none"
        />
        {unit && (
          <span aria-hidden="true" className="pr-3 text-sm whitespace-nowrap text-ink-2">
            {unit}
          </span>
        )}
      </span>
    </label>
  );
}
