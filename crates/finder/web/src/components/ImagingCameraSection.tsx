import { useEffect, useState } from "react";
import type { ObservingSession, SessionEdit } from "../useSession";

const RANGE =
  "Sensor width and height must be greater than 0 and at most 100 mm; pixel pitch must be greater than 0 and at most 50 µm.";

export function ImagingCameraSection({
  session,
  put,
}: {
  session: ObservingSession;
  put: (edit: SessionEdit) => Promise<void>;
}) {
  const camera = session.imaging_camera;
  const [width, setWidth] = useState("");
  const [height, setHeight] = useState("");
  const [pitch, setPitch] = useState("");
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    setWidth(camera ? String(camera.width_mm) : "");
    setHeight(camera ? String(camera.height_mm) : "");
    setPitch(camera ? String(camera.pitch_um) : "");
  }, [camera?.width_mm, camera?.height_mm, camera?.pitch_um]);

  const save = async (imaging_camera: SessionEdit["imaging_camera"]) => {
    setSaving(true);
    setMessage("Saving…");
    setError(null);
    try {
      await put({ imaging_camera });
      setMessage(imaging_camera ? "Imaging camera saved." : "Imaging camera forgotten.");
    } catch (e) {
      setMessage(null);
      setError(e instanceof Error ? e.message : "The imaging camera could not be saved. Try again.");
    } finally {
      setSaving(false);
    }
  };
  const submit = () => {
    const values = [width, height, pitch];
    if (
      values.some(
        (value, i) =>
          !value.trim() ||
          !Number.isFinite(Number(value)) ||
          Number(value) <= 0 ||
          Number(value) > (i === 2 ? 50 : 100),
      )
    ) {
      setMessage(null);
      setError(RANGE);
      return;
    }
    void save({ width_mm: Number(width), height_mm: Number(height), pitch_um: Number(pitch) });
  };
  return (
    <section aria-labelledby="imaging-camera-title" className="space-y-3">
      <h2 id="imaging-camera-title" className="section-title">
        Imaging camera
      </h2>
      <p className="quiet">
        Set the sensor size and pixel pitch to read the longest exposure for 1 px of field-rotation smear at the
        sensor's corner.
      </p>
      <form
        className="space-y-3 border-y border-rule py-4"
        onSubmit={(e) => {
          e.preventDefault();
          submit();
        }}
      >
        <div className="grid grid-cols-2 gap-3">
          {(
            [
              ["Sensor width", "mm", width, setWidth],
              ["Sensor height", "mm", height, setHeight],
              ["Pixel pitch", "µm", pitch, setPitch],
            ] as const
          ).map(([label, unit, value, change]) => (
            <label key={label} className="block space-y-1">
              <span className="text-sm font-semibold text-ink-2">{label}</span>
              <span className="flex items-center border border-ink focus-within:outline-2 focus-within:outline-offset-2 focus-within:outline-ink">
                <input
                  aria-label={label}
                  inputMode="decimal"
                  value={value}
                  onChange={(e) => change(e.target.value)}
                  aria-describedby={`imaging-camera-range${error ? " imaging-camera-error" : ""}`}
                  aria-invalid={error ? true : undefined}
                  className="h-11 w-full min-w-0 bg-transparent px-3 text-base outline-none"
                />
                <span aria-hidden="true" className="pr-3 text-sm text-ink-2">
                  {unit}
                </span>
              </span>
            </label>
          ))}
        </div>
        <p id="imaging-camera-range" className="quiet">
          {RANGE}
        </p>
        {message && <p role="status">{message}</p>}
        {error && (
          <p id="imaging-camera-error" role="alert">
            {error}
          </p>
        )}
        <button type="submit" className="button button-primary" disabled={saving}>
          Save imaging camera
        </button>
        {camera && (
          <button type="button" className="link ml-3" disabled={saving} onClick={() => void save(null)}>
            Forget imaging camera
          </button>
        )}
      </form>
    </section>
  );
}
