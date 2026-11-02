// One shared observing session, refreshed while this page is in use.
import { useCallback, useEffect, useRef, useState } from "react";
import type { ObservingSession, SessionSite } from "./api";
export type { ObservingSession, SessionSite } from "./api";

export interface SessionEdit {
  site?: SessionSite | null;
  min_alt_deg?: number;
  target?: string | null;
}

const EMPTY: ObservingSession = { site: null, min_alt_deg: undefined, target: null };

async function request(init?: RequestInit): Promise<ObservingSession> {
  const response = await (init ? fetch("/api/session", init) : fetch("/api/session"));
  if (!response.ok) {
    let message = `Session request failed (${response.status}).`;
    try {
      const body = (await response.json()) as { error?: string };
      if (body.error) message = body.error;
    } catch {
      // A non-JSON response still has its HTTP status.
    }
    throw new Error(message);
  }
  const body = (await response.json()) as Partial<ObservingSession> | null;
  // Older page fixtures answer unknown routes with health data, without session fields.
  return {
    site: body?.site ?? null,
    min_alt_deg: body?.min_alt_deg,
    target: body?.target ?? null,
    ...(body?.pointing !== undefined && { pointing: body.pointing }),
    ...(body?.time !== undefined && { time: body.time }),
    ...(body?.guidance != null && { guidance: body.guidance }),
    ...(body?.node != null && { node: body.node }),
    ...(body?.camera_settings != null && { camera_settings: body.camera_settings }),
    ...(body?.aim != null && { aim: body.aim }),
    ...(body?.field != null && { field: body.field }),
    ...(body?.mount != null && { mount: body.mount }),
    ...(body?.model != null && { model: body.model }),
    ...(body?.goto != null && { goto: body.goto }),
  };
}

export function useSession() {
  const [session, setSession] = useState(EMPTY);
  const [loaded, setLoaded] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const mounted = useRef(false);
  const latest = useRef(0);

  const read = useCallback(async () => {
    const ticket = ++latest.current;
    try {
      const next = await request();
      if (mounted.current && ticket === latest.current) {
        setSession(next);
        setLoaded(true);
        setError(null);
      }
    } catch (e) {
      if (mounted.current && ticket === latest.current) {
        setError(e instanceof Error ? e.message : "The observing session could not be loaded.");
      }
      throw e;
    }
  }, []);

  useEffect(() => {
    mounted.current = true;
    let timer: ReturnType<typeof setInterval> | undefined;
    const refresh = () => void read().catch(() => {});
    const visibility = () => {
      clearInterval(timer);
      if (document.visibilityState === "visible") {
        refresh();
        timer = setInterval(() => {
          if (document.visibilityState === "visible") refresh();
        }, 5000);
      }
    };
    visibility();
    document.addEventListener("visibilitychange", visibility);
    return () => {
      mounted.current = false;
      ++latest.current;
      clearInterval(timer);
      document.removeEventListener("visibilitychange", visibility);
    };
  }, [read]);

  const put = useCallback(
    async (edit: SessionEdit) => {
      // A read started before this edit must not replace the edited session.
      ++latest.current;
      try {
        await request({ method: "PUT", headers: { "Content-Type": "application/json" }, body: JSON.stringify(edit) });
        await read();
      } catch (e) {
        if (mounted.current) setError(e instanceof Error ? e.message : "The observing session could not be saved.");
        throw e;
      }
    },
    [read],
  );

  return { session, loaded, error, put };
}
