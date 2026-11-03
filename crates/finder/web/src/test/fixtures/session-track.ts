import type { ObservingSession } from "../../api";
import packed from "./session-track.json";

// Lossless captured-session compression; expectations never come from page code.
export default Object.fromEntries(
  Object.entries(packed.sessions).map(([name, delta]) => [name, { ...packed.base, ...delta }]),
) as unknown as Record<keyof typeof packed.sessions, ObservingSession>;
