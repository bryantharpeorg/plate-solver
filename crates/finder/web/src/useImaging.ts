// The imaging camera's state, read once a second while the Imaging page is open.
import { useCallback, useEffect, useRef, useState } from "react";
import { getImaging, type ImagingView } from "./api";

export function useImaging(): { view: ImagingView | null; refresh: () => Promise<void> } {
  const [view, setView] = useState<ImagingView | null>(null);
  const timer = useRef<ReturnType<typeof setInterval> | undefined>(undefined);
  const read = useCallback(async () => {
    try {
      setView(await getImaging());
    } catch {
      // A failed read changes nothing the page shows (031 US7-S6).
    }
  }, []);
  useEffect(() => {
    void read();
    timer.current = setInterval(() => void read(), 1000);
    return () => clearInterval(timer.current);
  }, [read]);
  return { view, refresh: read };
}
