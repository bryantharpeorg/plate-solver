// The top bar: the frame being charted, the observing site, and night mode.

import { thumbUrl, type GoToProgress, type SessionMount, type SessionTrack, type FrameSource } from "../api";
import { StopButton } from "./StopButton";
import type { Site } from "../settings";
import { Chevron, Pin, Reticle, Torch } from "./Icons";

interface Props {
  mount?: SessionMount;
  goto?: GoToProgress;
  track?: SessionTrack;
  source: FrameSource | null;
  label: string | null;
  site: Site | null;
  night: boolean;
  onFrame: () => void;
  onTarget: () => void;
  onSite: () => void;
  onNight: (on: boolean) => void;
  onSetup: () => void;
  onHome: () => void;
  setup: boolean;
  onChange?: () => void;
}

const siteText = (site: Site) => `${Math.abs(site.lat).toFixed(1)}° ${site.lat >= 0 ? "N" : "S"}`;

export function TopBar({
  mount,
  goto,
  track,
  source,
  label,
  site,
  night,
  onFrame,
  onTarget,
  onSite,
  onNight,
  onSetup,
  onHome,
  setup,
  onChange,
}: Props) {
  return (
    <header className="sticky top-0 z-30 border-b border-rule bg-paper">
      <div
        className={`page-width flex h-14 items-center ${mount ? "gap-1" : "gap-2"} px-3 sm:h-16 sm:gap-3 sm:px-5 lg:px-10`}
      >
        <a
          href="/"
          onClick={(e) => {
            if (e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
            e.preventDefault();
            onHome();
          }}
          className="mr-auto flex items-center gap-2 font-semibold tracking-[0.02em]"
          aria-label="Finder home"
        >
          <Reticle size={22} />
          <span className="hidden text-[1.05rem] sm:inline">Finder</span>
        </a>
        <button
          type="button"
          className="chip min-w-0 max-w-[30vw] sm:max-w-[44vw] pl-1.5"
          onClick={onFrame}
          aria-label={label ?? "Frames"}
          aria-haspopup="dialog"
        >
          {source?.kind === "sample" ? (
            <img src={thumbUrl(source.name)} alt="" className="photo size-7 object-cover" />
          ) : (
            <span className="grid size-7 place-items-center bg-paper-2" aria-hidden="true">
              <Reticle size={14} />
            </span>
          )}
          <span className="hidden truncate sm:inline">{label ?? "Frames"}</span>
          <Chevron size={16} />
        </button>
        <button
          type="button"
          className="chip px-2.5"
          onClick={onTarget}
          aria-label="Choose target"
          aria-haspopup="dialog"
        >
          <Reticle size={17} />
          <span className="hidden sm:inline">Target</span>
        </button>
        <button
          type="button"
          className={`chip px-2.5 ${mount ? "max-[380px]:hidden" : ""}`}
          onClick={onSite}
          aria-label={site ? `Site ${siteText(site)}` : "Set your site"}
        >
          <Pin size={17} />
          <span className="hidden sm:inline">{site ? siteText(site) : "Set site"}</span>
          {site && <span className="sm:hidden">{Math.round(site.lat)}°</span>}
        </button>
        <a
          href="/setup"
          className="chip"
          aria-current={setup ? "page" : undefined}
          onClick={(e) => {
            if (e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
            e.preventDefault();
            onSetup();
          }}
        >
          Setup
        </a>
        <StopButton mount={mount} goto={goto} track={track} onChange={onChange} />
        <label className="flex items-center gap-2 text-sm font-medium">
          <span className="hidden sm:inline">Night</span>
          <span className="sm:hidden" aria-hidden="true">
            <Torch size={18} />
          </span>
          <button
            type="button"
            role="switch"
            aria-checked={night}
            aria-label="Night mode: red and black"
            className="switch"
            onClick={() => onNight(!night)}
          />
        </label>
      </div>
    </header>
  );
}
