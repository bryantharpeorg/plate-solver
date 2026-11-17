import type { MountSky } from "../api";

export function MountSkyBanner({ sky }: { sky: MountSky }) {
  return (
    <div role="status" className="page-width border-b border-rule px-4 py-2 text-sm text-ink sm:px-5 lg:px-10">
      <strong className="font-semibold">Drawn sky.</strong> These stars come from the mount's encoders, not seen by the
      camera. No hand pushes: a push moves the tube but not the drawn sky.
      {sky.power_cycled && (
        <> The mount was power-cycled, so the drawn sky has stopped: restart the console with a fresh --tube-alt.</>
      )}
      {sky.night_utc && <> Night clock from {sky.night_utc}. Keep the mirror cover and the finder cap on.</>}
    </div>
  );
}
