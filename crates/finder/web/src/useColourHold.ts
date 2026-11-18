// The hold-to-see-colour control (031), built once for every picture from the
// imaging camera: this page's preview and, later, the stack's picture.
import { useEffect, useRef, useState, type KeyboardEvent } from "react";

/** The longest one hold shows colour. */
export const COLOUR_HOLD_MS = 10_000;

export interface ColourHoldBind {
  onPointerDown: () => void;
  onPointerUp: () => void;
  onPointerLeave: () => void;
  onPointerCancel: () => void;
  onBlur: () => void;
  onKeyDown: (event: KeyboardEvent<HTMLButtonElement>) => void;
  onKeyUp: (event: KeyboardEvent<HTMLButtonElement>) => void;
}

/** Colour while held, at night only, for at most COLOUR_HOLD_MS a hold. */
export function useColourHold(night: boolean): { held: boolean; bind: ColourHoldBind } {
  // A hold is under way while its 10 s timer runs; `held` also needs the night.
  const [underway, setUnderway] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const beganKey = useRef<string | null>(null);
  const nightRef = useRef(night);
  nightRef.current = night;

  const end = () => {
    if (timer.current !== null) {
      clearTimeout(timer.current);
      timer.current = null;
    }
    beganKey.current = null;
    setUnderway(false);
  };

  const begin = (key: string | null) => {
    if (!nightRef.current || timer.current !== null) return; // one hold at a time
    beganKey.current = key;
    timer.current = setTimeout(() => {
      timer.current = null;
      beganKey.current = null;
      setUnderway(false);
    }, COLOUR_HOLD_MS);
    setUnderway(true);
  };

  // Night turning false ends a hold; unmount clears its timer.
  useEffect(() => {
    if (!night) end();
  }, [night]);
  useEffect(
    () => () => {
      if (timer.current !== null) clearTimeout(timer.current);
    },
    [],
  );

  const bind: ColourHoldBind = {
    onPointerDown: () => begin(null),
    onPointerUp: end,
    onPointerLeave: end,
    onPointerCancel: end,
    onBlur: end,
    onKeyDown: (event) => {
      // A key the browser repeats must not start the limit again.
      if (event.repeat) return;
      if (event.key === " " || event.key === "Enter") begin(event.key);
    },
    onKeyUp: (event) => {
      if (beganKey.current === event.key) end();
    },
  };
  return { held: underway && night, bind };
}
