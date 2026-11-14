import { useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { getHealth, type FrameSource, type Health, type SolveSettings } from "./api";
import { useCalm } from "./calm";
import { Chart, DEFAULT_LAYERS, type Layers } from "./components/Chart";
import { ChartHeader, ChartHeaderPlaceholder } from "./components/ChartHeader";
import { FieldList, FieldRoster } from "./components/FieldList";
import { FramePicker, SampleGrid } from "./components/FramePicker";
import { GuidanceBlock } from "./components/GuidanceBlock";
import { LivePreview } from "./components/LivePreview";
import { Upload } from "./components/Icons";
import { NextTargets } from "./components/NextTargets";
import { SetupPage } from "./components/SetupPage";
import { SurveyView } from "./components/SurveyView";
import { TopBar } from "./components/TopBar";
import { TargetSheet } from "./components/TargetSheet";
import { useLastSample, useNight, useSite } from "./settings";
import { useSolve } from "./useSolve";
import { useSession } from "./useSession";
import { useLive } from "./useLive";

const FIRST_SAMPLE = "orion2.jpg";
// A transparent pixel keeps the chart's existing image layer empty until a preview
// is available below it, without making a request for a full camera photograph.
const LIVE_IMAGE = "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7";

export function App() {
  const [night, setNight] = useNight();
  const [browserSite, setBrowserSite] = useSite();
  const { session, loaded, error: sessionError, put, refresh } = useSession();
  const changed = () => void refresh().catch(() => {});
  const { live, pointingSeq: liveSeq, previewSeq, stale, session: liveSession } = useLive(session);
  const site = useMemo(
    () => (session.site ? { lat: session.site.lat_deg, lon: session.site.lon_deg } : null),
    [session.site?.lat_deg, session.site?.lon_deg],
  );
  const adopted = useRef(false);
  useEffect(() => {
    if (!loaded || adopted.current) return;
    adopted.current = true;
    if (!browserSite) return;
    if (session.site) {
      setBrowserSite(null);
      return;
    }
    // Clear the old browser copy only after the session accepts it. The ref also
    // prevents duplicate adoption when React checks effects twice in StrictMode.
    void put({ site: { lat_deg: browserSite.lat, lon_deg: browserSite.lon } })
      .then(() => setBrowserSite(null))
      .catch(() => {});
  }, [loaded, browserSite, session.site, put, setBrowserSite]);
  const [lastSample, setLastSample] = useLastSample();
  const { state, run } = useSolve();
  const [picker, setPicker] = useState(false);
  const [targetSheet, setTargetSheet] = useState(false);
  const [path, setPath] = useState(() => window.location.pathname);
  const [mcp, setMcp] = useState<unknown>();
  useEffect(() => {
    let current = true;
    setMcp(undefined);
    if (path === "/setup") {
      void (async () => {
        try {
          const response = await fetch("/api/mcp");
          if (!response.ok) return;
          const status: unknown = await response.json();
          if (current) setMcp(status);
        } catch {
          // Setup remains usable if this optional read fails.
        }
      })();
    }
    return () => {
      current = false;
    };
  }, [path]);
  useEffect(() => {
    const popped = () => setPath(window.location.pathname);
    window.addEventListener("popstate", popped);
    return () => window.removeEventListener("popstate", popped);
  }, []);
  const navigate = (next: string) => {
    window.history.pushState(null, "", next);
    setPath(next);
  };
  const [layers, setLayers] = useState<Layers>(DEFAULT_LAYERS);
  const [selected, setSelected] = useState<string | null>(null);
  const [health, setHealth] = useState<Health | null>(null);
  const [settings, setSettings] = useState<SolveSettings>({});

  useEffect(() => {
    getHealth()
      .then(setHealth)
      .catch(() => setHealth(null));
  }, []);

  // Come back to the last sample charted.
  useEffect(() => {
    if (lastSample) run({ kind: "sample", name: lastSample }, settings);
    // Only on first load: `run` and `lastSample` are read once, deliberately.
  }, []);

  const open = (source: FrameSource, solveWith: SolveSettings = settings) => {
    setSelected(null);
    if (source.kind === "sample") setLastSample(source.name);
    run(source, solveWith);
  };

  const uploadedReport = state.phase === "done" ? state.report : null;
  // Sequence numbers order solved uploads and live pointings on the console.
  // Arrival order also covers no-match reports, which carry no pointing sequence.
  const [preferLive, setPreferLive] = useState(false);
  useEffect(() => {
    if (live) setPreferLive(true);
  }, [live]);
  useEffect(() => {
    if (state.phase === "done" || state.phase === "error") setPreferLive(false);
  }, [state]);
  const showLive =
    live !== null &&
    (uploadedReport?.pointing_seq !== undefined && liveSeq != null
      ? liveSeq > uploadedReport.pointing_seq
      : preferLive);
  const liveReport = useMemo(
    () =>
      live && {
        ...live.report,
        ...(live.pointing_seq !== null && { pointing_seq: live.pointing_seq }),
      },
    [live],
  );
  const report = useCalm(showLive ? liveReport : uploadedReport, live?.time ?? null, showLive);
  const reference = useMemo(() => {
    const objects = report?.objects ?? [];
    return objects.find((o) => o.id === selected) ?? objects.find((o) => o.inside) ?? objects[0] ?? null;
  }, [report, selected]);

  const source = showLive || state.phase === "idle" ? null : state.source;
  const label = showLive ? `Live frame ${live.frame_id}` : state.phase === "idle" ? null : state.label;
  const imageUrl = showLive ? LIVE_IMAGE : state.phase === "idle" ? "" : state.imageUrl;
  const busy = !showLive && state.phase === "solving";
  const hasFrame = showLive || state.phase !== "idle";
  const chartSession = showLive ? liveSession : session;

  // The frame's shape sets how wide the page is on a big screen (see .page-width); keep
  // the last one while the next frame solves.
  const [aspect, setAspect] = useState(4 / 3);
  useEffect(() => {
    if (report) setAspect(report.width / report.height);
  }, [report]);

  return (
    <div className="contents" style={{ "--chart-aspect": aspect } as CSSProperties}>
      <TopBar
        onChange={changed}
        mount={session.mount}
        goto={session.goto}
        track={session.track}
        motion={session.motion}
        source={source}
        label={label}
        site={site}
        night={night}
        onFrame={() => setPicker(true)}
        onTarget={() => setTargetSheet(true)}
        onSite={() => navigate("/setup")}
        onSetup={() => navigate("/setup")}
        onHome={() => navigate("/")}
        setup={path === "/setup"}
        onNight={setNight}
      />
      {sessionError && (
        <p role="alert" className="page-width px-4 pt-3 text-ink">
          {sessionError}
        </p>
      )}
      {path === "/setup" ? (
        <SetupPage session={session} mcp={mcp} put={put} onBack={() => navigate("/")} onChange={changed} />
      ) : (
        <>
          {/*
        With a target, guidance and the title lead, followed by the chart, then what
        is in the field. Without one, keep the field roster under the title.
      */}
          <main
            className={`page-width grid gap-x-10 gap-y-4 px-4 pt-3 pb-16 sm:gap-y-5 sm:px-5 sm:pt-4 lg:grid-cols-[minmax(22rem,30rem)_minmax(0,1fr)] ${session.target ? "max-sm:gap-y-1 lg:grid-rows-[auto_auto_auto_1fr_auto]" : "lg:grid-rows-[auto_auto_1fr_auto]"} lg:gap-y-8 lg:px-10 lg:pt-8`}
          >
            {session.target && (
              <div className="lg:col-span-2 lg:row-start-1">
                <GuidanceBlock
                  session={liveSession}
                  stale={stale}
                  onSetup={() => navigate("/setup")}
                  onChange={changed}
                />
              </div>
            )}
            {hasFrame && (
              <div
                className={`space-y-3 sm:space-y-4 lg:col-start-1 ${session.target ? "lg:row-start-2" : "lg:row-start-1"}`}
              >
                {report ? (
                  <ChartHeader
                    report={report}
                    label={label ?? ""}
                    sigma={settings.sigma ?? 8}
                    stale={showLive && stale}
                    onRetry={(sigma) => {
                      const next = { ...settings, sigma };
                      setSettings(next);
                      if (source) open(source, next);
                    }}
                  />
                ) : (
                  <ChartHeaderPlaceholder busy={busy} label={label} />
                )}
                {report?.pointing && !session.target && <FieldRoster report={report} />}
                {!showLive && state.phase === "error" && (
                  <ErrorNote message={state.message} onRetry={() => run(state.source, settings)} />
                )}
              </div>
            )}

            {!hasFrame && (
              <div className={`lg:col-span-2 ${session.target ? "lg:row-start-2" : "lg:row-start-1"}`}>
                <FirstRun onPick={(name) => open({ kind: "sample", name })} onUpload={() => setPicker(true)} />
              </div>
            )}

            {hasFrame && (
              <div className={`lg:col-start-2 lg:row-span-3 ${session.target ? "lg:row-start-2" : "lg:row-start-1"}`}>
                <div className="lg:sticky lg:top-24">
                  <Chart
                    report={report}
                    live={showLive}
                    imageUrl={imageUrl}
                    label={label ?? ""}
                    busy={busy}
                    layers={layers}
                    onLayers={setLayers}
                    session={chartSession}
                  />
                  {showLive && <LivePreview seq={previewSeq} />}
                </div>
              </div>
            )}

            {report?.pointing && (
              <div
                className={`space-y-3 pt-3 lg:col-start-1 ${session.target ? "lg:row-start-3" : "lg:row-start-2"} lg:pt-0`}
              >
                {session.target && <FieldRoster report={report} />}
                <FieldList report={report} live={showLive} selected={reference?.id ?? null} onSelect={setSelected} />
              </div>
            )}

            {report?.pointing && (
              <div
                className={`space-y-10 pt-5 lg:col-start-1 ${session.target ? "lg:row-start-4" : "lg:row-start-3"} lg:pt-0`}
              >
                <NextTargets
                  reference={reference}
                  nearby={report.nearby}
                  site={site}
                  minAlt={session.min_alt_deg}
                  onSetSite={() => navigate("/setup")}
                />
                <SurveyView pointing={report.pointing} />
              </div>
            )}

            <footer
              className={`pt-5 text-sm text-ink-2 lg:col-span-2 ${session.target ? "lg:row-start-5" : "lg:row-start-4"} lg:pt-0`}
            >
              {health
                ? `Database: ${health.database.catalog}, ${health.database.stars.toLocaleString("en-US")} stars, fields ${health.database.min_fov}–${health.database.max_fov}° · finder ${health.version}`
                : "Finder"}
            </footer>
          </main>
        </>
      )}
      <FramePicker
        open={picker}
        keepTopBarActive={session.mount !== undefined}
        current={source?.kind === "sample" ? source.name : null}
        settings={settings}
        onSettings={setSettings}
        onClose={() => setPicker(false)}
        onPick={(source) => {
          if (path === "/setup") navigate("/");
          open(source);
        }}
      />
      <TargetSheet
        keepTopBarActive={session.mount !== undefined}
        open={targetSheet}
        target={session.target ?? null}
        put={put}
        onClose={() => setTargetSheet(false)}
      />
    </div>
  );
}

function FirstRun({ onPick, onUpload }: { onPick: (name: string) => void; onUpload: () => void }) {
  return (
    <section aria-labelledby="first-title" className="space-y-5 border border-ink p-[3px]">
      <div className="space-y-5 border border-ink p-4 sm:p-6">
        <h1 id="first-title" className="chart-title text-[1.3rem]">
          Chart a frame
        </h1>
        <p className="max-w-[60ch] text-ink-2">
          Finder finds the stars in a frame, works out where it points with no hint, and draws the sky's grid, the
          matched catalog stars and the Messier objects over it. Start with one of the sample frames, or your own.
        </p>
        <div className="flex flex-wrap gap-3">
          <button type="button" className="button button-primary" onClick={() => onPick(FIRST_SAMPLE)}>
            Chart the Orion sample
          </button>
          <button type="button" className="button" onClick={onUpload}>
            <Upload size={18} />
            Open your own frame
          </button>
        </div>
        <SampleGrid current={null} onPick={onPick} heading="h2" />
      </div>
    </section>
  );
}

function ErrorNote({ message, onRetry }: { message: string; onRetry: () => void }) {
  return (
    <div role="alert" className="space-y-3 border-t-2 border-alert pt-3">
      <p className="font-semibold text-alert">That frame did not go through.</p>
      <p className="text-ink-2">{message}</p>
      <button type="button" className="button" onClick={onRetry}>
        Try again
      </button>
    </div>
  );
}
