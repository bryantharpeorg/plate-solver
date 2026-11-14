// The console's JSON API (crates/finder/src/api.rs). Field names and shapes mirror the
// Rust types they come from; keep the two in step.

export type Family =
  "nebula" | "planetary_nebula" | "supernova_remnant" | "open_cluster" | "globular_cluster" | "galaxy" | "star";

export interface Constellation {
  abbr: string;
  name: string;
}

export interface DeepSkyObject {
  id: string;
  designation: string | null;
  name: string | null;
  code: string;
  kind: string;
  family: Family;
  ra_deg: number;
  dec_deg: number;
  constellation: Constellation;
  major_arcmin: number | null;
  minor_arcmin: number | null;
  pa_deg: number | null;
  mag: number | null;
}

/** The OpenNGC catalog is separate from the chart's Messier catalog. */
export interface Target extends Omit<DeepSkyObject, "designation" | "name" | "constellation"> {
  messier: string | null;
  common_names: string[];
  constellation: string;
}

export type TargetClass = "sun_guard" | "below_horizon" | "too_low" | "pointable" | "no_site";
export interface TargetMatch extends Target {
  class: TargetClass;
}

export type TargetPlacement =
  | { on_frame: true; x: number; y: number; axes: [number, number]; angle: number }
  | { on_frame: false; direction_deg: number; distance_deg: number };

export interface Guidance {
  class: TargetClass;
  stale?: true;
  rise?: number;
  transit?: number;
  set?: number;
  never?: boolean;
  turn_deg?: number;
  tilt_deg?: number;
  placement?: TargetPlacement;
}

export interface Pointing {
  ra_deg: number;
  dec_deg: number;
  ra_text: string;
  dec_text: string;
  constellation: Constellation;
  roll_deg: number;
  north_deg: number;
  fov_deg: number;
  fov_vertical_deg: number;
  arcsec_per_px: number;
  distortion: number;
  matched: number;
  rmse_arcsec: number | null;
  p90_arcsec: number | null;
  max_arcsec: number | null;
  false_alarm: number | null;
}

export interface StarMark {
  x: number;
  y: number;
  id: string;
  name: string | null;
  mag: number;
  matched: boolean;
  error_px: number | null;
}

export interface ObjectMark extends DeepSkyObject {
  x: number;
  y: number;
  radius_px: number;
  minor_px: number;
  /** Image direction of the major axis, degrees clockwise from up. */
  angle_deg: number;
  separation_deg: number;
  inside: boolean;
}

export type Edge = "top" | "right" | "bottom" | "left";

export interface GridLine {
  kind: "ra" | "dec";
  value_deg: number;
  label: string;
  paths: [number, number][][];
  ticks: { x: number; y: number; edge: Edge }[];
}

export interface NearbyMark extends DeepSkyObject {
  separation_deg: number;
  direction_deg: number;
}

export type SolveStatus = "match_found" | "no_match" | "timeout" | "cancelled" | "too_few";

export interface SolveReport {
  pointing_seq?: number;
  name: string | null;
  status: SolveStatus;
  hint: string | null;
  width: number;
  height: number;
  timing_ms: { decode: number; detect: number; solve: number };
  detected: [number, number][];
  used: number;
  pointing: Pointing | null;
  stars: StarMark[];
  objects: ObjectMark[];
  nearby: NearbyMark[];
  grid: GridLine[];
}

export interface LiveReport {
  frame_id: number;
  /** Mid-exposure, on the console's clock, in Unix seconds. */
  time: number;
  pointing_seq: number | null;
  preview_seq: number | null;
  stars: number;
  report: SolveReport;
  guidance?: Guidance;
}

export interface SessionSite {
  lat_deg: number;
  lon_deg: number;
}

export interface SessionPointing {
  seq: number;
  time: number;
  ra_deg: number;
  dec_deg: number;
  alt_deg?: number;
  az_deg?: number;
  source?: "upload" | "live";
  frame_id?: number;
}

export interface SessionNode {
  address: string | null;
  token_set: boolean;
  connected: boolean;
  last_frame_age_s: number | null;
  offset_s: number | null;
  camera: {
    model: string;
    width: number;
    height: number;
    exposure_ms: { min: number; max: number };
    gain: { min: number; max: number };
  } | null;
  stars: number | null;
  out_of_range: { setting: "exposure_ms" | "gain"; stored: number; min: number; max: number }[];
}

export interface GoToProgress {
  state: "slewing" | "solving" | "correcting" | "done" | "failed" | "halted";
  correction: number;
  error_arcsec: number | null;
  reason: string | null;
}

export interface SessionMount {
  connected: boolean;
  board: string | null;
  firmware: string | null;
  moving: boolean;
}

export interface ImagingCamera {
  width_mm: number;
  height_mm: number;
  pitch_um: number;
}

export interface SessionTrack {
  wanted: boolean;
  state: "off" | "armed" | "tracking" | "keyhole_wait";
  target?: Target;
  ended_reason?: string;
  readout?: {
    rms_arcsec: number;
    field_rotation_deg_per_min: number;
    longest_exposure_s?: number | "no limit";
  };
}

export interface PointingModel {
  syncs: number;
  epoch: number;
  ia: number | null;
  ie: number | null;
  base_tilt?: { deg: number; toward_deg: number };
  fixed?: { ca: number; npae: number; tf: number; camera: number };
  rms_arcsec: number;
  needs: number;
}

export interface SessionMotion {
  by: "page" | "mcp";
  tool: string;
  since: number;
}

export interface McpStatus {
  enabled: boolean;
  local_only: boolean;
  urls: string[];
  allowed_hosts: string[];
}

export interface ObservingSession {
  motion?: SessionMotion;
  track?: SessionTrack;
  imaging_camera?: ImagingCamera;
  backlash?: { alt_deg?: number; az_deg?: number };
  goto?: GoToProgress;
  /** The planner's refusal, omitted when an idle GoTo would be accepted. */
  goto_refusal?: string;
  model?: PointingModel;
  mount?: SessionMount;
  site: SessionSite | null;
  min_alt_deg?: number;
  target?: Target | null;
  pointing?: SessionPointing | null;
  time?: number;
  guidance?: Guidance;
  node?: SessionNode;
  camera_settings?: { exposure_ms: "auto" | number; gain: "auto" | number; sigma: number };
  aim?: { x: number; y: number };
  field?: { fov_deg: number | null; db_min_fov_deg: number; db_max_fov_deg: number; unsolved_s: number };
}

export interface Observer {
  lat: number;
  lon: number;
  time: number;
}

export interface Suggestion extends DeepSkyObject {
  separation_deg: number;
  alt_deg: number | null;
  az_deg: number | null;
  up: boolean | null;
}

export interface SimilarReport {
  reference: DeepSkyObject;
  family: string;
  observer: Observer | null;
  suggestions: Suggestion[];
}

export interface Sample {
  name: string;
  label: string;
}

export interface Health {
  status: string;
  version: string;
  database: { catalog: string; stars: number; patterns: number; min_fov: number; max_fov: number };
  samples: number;
}

/** Where a frame to solve comes from. */
export type FrameSource = { kind: "sample"; name: string } | { kind: "file"; file: File };

export interface SolveSettings {
  /** Horizontal field of view estimate in degrees, if known. */
  fov?: number;
  /** Detection threshold in standard deviations of the noise (the server's default is 8). */
  sigma?: number;
}

export class ApiError extends Error {}

export async function postTrack(on: boolean): Promise<ObservingSession> {
  return json(
    await fetch("/api/track", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ on }),
    }),
  );
}

async function json<T>(response: Response): Promise<T> {
  if (!response.ok) {
    let message = `${response.status} ${response.statusText}`;
    try {
      const body = (await response.json()) as { error?: string };
      if (body.error) message = body.error;
    } catch {
      // Not JSON: keep the status line.
    }
    throw new ApiError(message);
  }
  return (await response.json()) as T;
}

export const sampleUrl = (name: string) => `/api/samples/${encodeURIComponent(name)}`;
export const thumbUrl = (name: string) => `${sampleUrl(name)}/thumb`;

export async function getHealth(): Promise<Health> {
  return json(await fetch("/api/health"));
}

export interface NodeCameraEdit {
  node?: { address?: string; token?: string } | null;
  camera_settings?: Partial<NonNullable<ObservingSession["camera_settings"]>>;
}

/** Pairing tokens are write-only; an empty token field is omitted by the caller. */
export async function putNodeCamera(edit: NodeCameraEdit): Promise<ObservingSession> {
  return json(
    await fetch("/api/session", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(edit),
    }),
  );
}

export async function setAim(frame_id: number, x: number, y: number): Promise<ObservingSession> {
  return json(
    await fetch("/api/aim", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ frame_id, x, y }),
    }),
  );
}

export async function resetAim(): Promise<ObservingSession> {
  return json(await fetch("/api/aim", { method: "DELETE" }));
}

export async function getSamples(): Promise<Sample[]> {
  return json(await fetch("/api/samples"));
}

export async function getTargets(query: string, signal?: AbortSignal): Promise<TargetMatch[]> {
  return json(await fetch(`/api/targets?${new URLSearchParams({ q: query })}`, { signal }));
}

export async function solve(source: FrameSource, settings: SolveSettings = {}): Promise<SolveReport> {
  const body = new FormData();
  if (source.kind === "sample") body.append("sample", source.name);
  else body.append("image", source.file, source.file.name);
  if (settings.fov !== undefined) body.append("fov", String(settings.fov));
  if (settings.sigma !== undefined) body.append("sigma", String(settings.sigma));
  return json(await fetch("/api/solve", { method: "POST", body }));
}

export async function getSimilar(id: string, observer: Omit<Observer, "time"> | null): Promise<SimilarReport> {
  const query = new URLSearchParams({ id, limit: "6" });
  if (observer) {
    query.set("lat", String(observer.lat));
    query.set("lon", String(observer.lon));
    query.set("time", String(Date.now() / 1000));
  }
  return json(await fetch(`/api/similar?${query}`));
}

/** Motion requests have no body. Stop deliberately carries no bearer token. */
export async function postMotion(route: "/api/goto" | "/api/stop" | "/api/model/second-sync"): Promise<void> {
  await json(await fetch(route, { method: "POST" }));
}

export interface StopReply {
  stopped: boolean;
  acknowledged?: boolean;
}

/** Stop deliberately carries no bearer token. */
export async function postStop(): Promise<StopReply> {
  return json(await fetch("/api/stop", { method: "POST" }));
}

export async function resetModel(what: "night" | "fixed"): Promise<void> {
  await json(
    await fetch("/api/model/reset", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ what }),
    }),
  );
}
