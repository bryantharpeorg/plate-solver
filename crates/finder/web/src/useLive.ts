import { useEffect, useRef, useState } from "react";
import type { LiveReport, ObservingSession, SessionPointing } from "./api";

/** Follow the node only while connected; freshness always uses the console clock. */
export function useLive(session: ObservingSession) {
  const [live, setLive] = useState<LiveReport | null>(null);
  const [solved, setSolved] = useState<LiveReport | null>(null);
  const [previewSeq, setPreviewSeq] = useState<number | null>(null);
  const offset = useRef(0);
  const [now, setNow] = useState(() => Date.now() / 1000);
  const connected = session.node?.connected === true;

  useEffect(() => {
    if (session.time !== undefined) {
      offset.current = session.time - Date.now() / 1000;
      setNow(session.time);
    }
  }, [session.time]);

  useEffect(() => {
    if (!connected) return;
    const stream = new EventSource("/api/live");
    const receive = (event: MessageEvent<string>) => {
      let next: LiveReport;
      try {
        next = JSON.parse(event.data) as LiveReport;
      } catch {
        return;
      }
      offset.current = next.time - Date.now() / 1000;
      setNow(next.time);
      setLive(next);
      if (next.pointing_seq !== null && next.report.pointing) setSolved(next);
      if (next.preview_seq !== null) setPreviewSeq(next.preview_seq);
    };
    stream.addEventListener("live", receive);
    // EventSource handles transient transport errors and reconnects itself.
    return () => {
      stream.removeEventListener("live", receive);
      stream.close();
    };
  }, [connected]);

  const useStreamPointing =
    solved !== null && solved.pointing_seq !== null && solved.pointing_seq > (session.pointing?.seq ?? -1);
  const pointing: SessionPointing | null | undefined = useStreamPointing
    ? {
        seq: solved.pointing_seq!,
        time: solved.time,
        ra_deg: solved.report.pointing!.ra_deg,
        dec_deg: solved.report.pointing!.dec_deg,
        source: "live",
        frame_id: solved.frame_id,
        ...(solved.up_deg !== undefined && { up_deg: solved.up_deg }),
      }
    : session.pointing;
  const guidance = useStreamPointing ? live?.guidance : session.guidance;
  const livePointing = pointing?.source === "live";

  useEffect(() => {
    if (!livePointing) return;
    const timer = setInterval(() => setNow(Date.now() / 1000 + offset.current), 250);
    return () => clearInterval(timer);
  }, [livePointing]);

  const stale = livePointing && (now - pointing.time > 5 || guidance?.stale === true);
  // Strip directions even if a poll still carries the previously fresh guidance.
  const { turn_deg, tilt_deg, ...withoutDirections } = guidance ?? {};
  const current: ObservingSession = {
    ...session,
    ...(pointing !== undefined && { pointing }),
    ...(livePointing && { time: now }),
    ...(guidance && { guidance: stale ? { ...withoutDirections, class: guidance.class, stale: true } : guidance }),
  };
  return { live, pointingSeq: solved?.pointing_seq, previewSeq, stale, session: current };
}
