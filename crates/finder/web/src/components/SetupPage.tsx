// The atlas's front matter: the site and minimum altitude shared by every device.
import { useEffect, useState } from "react";
import { isValidSite } from "../settings";
import type { ObservingSession, SessionEdit } from "../useSession";
import { useLive } from "../useLive";
import { AimTap } from "./AimTap";
import { FieldWarning } from "./FieldWarning";
import { ScopeNodeSection } from "./ScopeNodeSection";
import { MountSection } from "./MountSection";

interface Props {
  session: ObservingSession;
  put: (edit: SessionEdit) => Promise<void>;
  onBack: () => void;
  onChange?: () => void;
}

export function SetupPage({ session, put, onBack, onChange }: Props) {
  const { live } = useLive(session);
  const [lat, setLat] = useState("");
  const [lon, setLon] = useState("");
  const [minAlt, setMinAlt] = useState("15");
  const [siteMessage, setSiteMessage] = useState<string | null>(null);
  const [altMessage, setAltMessage] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    setLat(session.site ? String(session.site.lat_deg) : "");
    setLon(session.site ? String(session.site.lon_deg) : "");
  }, [session.site?.lat_deg, session.site?.lon_deg]);
  useEffect(() => setMinAlt(String(session.min_alt_deg ?? 15)), [session.min_alt_deg]);

  const save = async (edit: SessionEdit, message: string, tell: (message: string) => void) => {
    setSaving(true);
    tell("Saving…");
    try {
      await put(edit);
      tell(message);
    } catch (e) {
      tell(e instanceof Error ? e.message : "The observing session could not be saved. Try again.");
    } finally {
      setSaving(false);
    }
  };

  const saveSite = () => {
    if (!lat.trim() || !lon.trim() || !isValidSite(Number(lat), Number(lon))) {
      setSiteMessage("Latitude goes from −90 to 90 and longitude from −180 to 180, east positive.");
      return;
    }
    void save({ site: { lat_deg: Number(lat), lon_deg: Number(lon) } }, "Site saved.", setSiteMessage);
  };
  const saveAltitude = () => {
    const value = Number(minAlt);
    if (!minAlt.trim() || !Number.isFinite(value) || value < 0 || value > 80) {
      setAltMessage("Minimum altitude must be 0–80°.");
      return;
    }
    void save({ min_alt_deg: value }, "Minimum altitude saved.", setAltMessage);
  };
  const locate = () => {
    if (!window.isSecureContext || !navigator.geolocation) {
      setSiteMessage(
        "This browser only shares its location with pages served over HTTPS or from the same device. Enter it by hand.",
      );
      return;
    }
    setSiteMessage("Asking for this device's location…");
    navigator.geolocation.getCurrentPosition(
      (p) => {
        setLat(p.coords.latitude.toFixed(3));
        setLon(p.coords.longitude.toFixed(3));
        setSiteMessage(null);
      },
      (e) => setSiteMessage(`No location: ${e.message}. Enter it by hand.`),
      { timeout: 15000 },
    );
  };

  return (
    <main className="mx-auto max-w-[32.5rem] space-y-6 px-4 pt-4 pb-16 sm:px-5 sm:pt-6">
      <header className="border border-ink p-[3px]">
        <div className="space-y-2 border border-ink px-4 py-3">
          <h1 className="chart-title text-[1.3rem]">Setup</h1>
          <p className="quiet">One observing session, shared by every device at the telescope.</p>
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

      <section aria-labelledby="setup-site-title" className="space-y-3">
        <h2 id="setup-site-title" className="section-title">
          Site
        </h2>
        <p className="quiet">
          {session.site
            ? "Used to work out what is above your horizon."
            : "Site not set. Enter where the telescope stands."}
        </p>
        <form
          className="space-y-3 border-y border-rule py-4"
          onSubmit={(e) => {
            e.preventDefault();
            saveSite();
          }}
        >
          <div className="grid grid-cols-2 gap-3">
            <Field label="Latitude" unit="° N" value={lat} onChange={setLat} placeholder="40.0" />
            <Field label="Longitude" unit="° E" value={lon} onChange={setLon} placeholder="-75.0" />
          </div>
          <p className="quiet">Longitude is east positive.</p>
          {siteMessage && (
            <p role="status" className="text-[0.9375rem] text-ink">
              {siteMessage}
            </p>
          )}
          <div className="flex flex-wrap gap-3">
            <button type="button" className="button" onClick={locate} disabled={saving}>
              Use this device's location
            </button>
            <button type="submit" className="button button-primary" disabled={saving}>
              Save site
            </button>
          </div>
          {session.site && (
            <button
              type="button"
              className="link"
              disabled={saving}
              onClick={() => void save({ site: null }, "Site forgotten.", setSiteMessage)}
            >
              Forget the site
            </button>
          )}
        </form>
      </section>

      <section aria-labelledby="setup-alt-title" className="space-y-3">
        <h2 id="setup-alt-title" className="section-title">
          Minimum altitude
        </h2>
        <form
          className="space-y-3 border-y border-rule py-4"
          onSubmit={(e) => {
            e.preventDefault();
            saveAltitude();
          }}
        >
          <div className="max-w-[12rem]">
            <Field label="Minimum altitude" unit="°" value={minAlt} onChange={setMinAlt} placeholder="15" />
          </div>
          <p className="quiet">Targets below this are too low; below 0° they are below the horizon. Allowed: 0–80°.</p>
          {altMessage && (
            <p role="status" className="text-[0.9375rem] text-ink">
              {altMessage}
            </p>
          )}
          <button type="submit" className="button button-primary" disabled={saving}>
            Save minimum altitude
          </button>
        </form>
      </section>
      <ScopeNodeSection session={session} live={live} />
      <MountSection session={session} onChange={onChange} />
      <FieldWarning field={session.field} />
      <AimTap session={session} live={live} />
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
        <span aria-hidden="true" className="pr-3 text-sm whitespace-nowrap text-ink-2">
          {unit}
        </span>
      </span>
    </label>
  );
}
