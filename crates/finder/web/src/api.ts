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
  separation_deg?: number;
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
  /** A detected star sits where the solution puts this one (server, spec 020 US1). */
  detected?: boolean;
  /** How the page draws it, after the live hold. Page-only; never sent by the server. */
  seen?: boolean;
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
  up_deg?: number;
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
  up_deg?: number;
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

/** The push by hand Track holds: moving, or the place it follows. */
export type SessionTrackPush =
  { state: "moving" } | { state: "following"; ra_deg: number; dec_deg: number; at: number };

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
  /** The push by hand Track is holding, absent unless it is moving or followed. */
  push?: SessionTrackPush;
}

export interface SessionRecording {
  on: boolean;
  folder: string;
  since: number;
  frames: number;
  solves: number;
  previews: number;
  ended_reason?: "stopped" | "limit" | "write_failed" | "node_restarted";
  raw?: { wanted: number; saved: number; error?: string };
}

// The imaging camera, as GET /api/imaging relays it (031). Field names and shapes
// mirror the Rust types they come from; keep the two in step.
export interface ImagingRange {
  min: number;
  max: number;
}

export interface ImagingCameraInfo {
  model: string;
  serial: string | null;
  width: number;
  height: number;
  pixel_um: number;
  bits: number;
  bayer: string | null;
  exposure_ms: ImagingRange;
  gain: ImagingRange | null;
  offset: ImagingRange | null;
  binnings: number[];
  cooler: boolean;
}

export interface ImagingStatus {
  temp_c: number | null;
  cooler_power: number | null;
  set_temp_c: number | null;
  exposing: boolean;
}

export interface ImagingSettings {
  exposure_ms: number;
  gain: number | null;
  offset: number | null;
  binning: number;
  roi: { x: number; y: number; width: number; height: number };
  cooler_on: boolean;
  set_temp_c: number | null;
  forward_kib_per_s: number | null;
  save_at_node: boolean;
}

export interface ImagingFrame {
  id: number;
  state: "exposing" | "receiving" | "saved" | "failed" | "aborted";
  purpose: string;
  exposure_ms: number;
  start?: number;
  bytes?: number;
  received?: number;
  received_at?: number;
  file?: string;
  preview?: string;
  node_file?: string;
  node_save_error?: string;
  error?: string;
}

export interface ImagingView {
  state: "off" | "no_link" | "no_service" | "no_camera" | "error" | "ready";
  error?: string;
  camera?: ImagingCameraInfo;
  status?: ImagingStatus;
  settings?: ImagingSettings;
  frame?: ImagingFrame;
  solve?: ImagingSolve;
  relation?: ImagingRelation;
  focus?: ImagingFocus;
  calibration?: ImagingCalibration;
  stack?: StackView;
  steering?: ImagingSteering;
  run?: ImagingRun;
  unfinished_run?: string;
  align?: ImagingAlign;
}

// The finder alignment (050): the imaging centre in the finder camera's frame.
export interface ImagingAlignCentre {
  x: number;
  y: number;
  finder_width: number;
  finder_height: number;
  right_deg: number;
  up_deg: number;
  separation_deg: number;
  on_frame: boolean;
  middle: boolean;
}

export interface ImagingAlign {
  state: "running" | "ended";
  since: number;
  exposure_ms: number;
  frames: number;
  last?: {
    frame_id: number;
    at: number;
    outcome: "measured" | "not_solved" | "no_live_solve";
    error?: string;
    centre?: ImagingAlignCentre;
    unsolved?: number;
    aim?: string;
    done: boolean;
  };
  ended?: "stopped" | "failed";
  error?: string;
}

export async function postImagingAlign(): Promise<ImagingView> {
  return json(
    await fetch("/api/imaging/align", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: "{}",
    }),
  );
}

export async function deleteImagingAlign(): Promise<ImagingView> {
  return json(await fetch("/api/imaging/align", { method: "DELETE" }));
}

// The console's imaging run (037 and 041), including its latest ending.
export interface ImagingRun {
  id: string;
  state:
    | "slewing"
    | "tracking"
    | "first_frame"
    | "focus_check"
    | "focus_pause"
    | "solving"
    | "centring"
    | "stacking"
    | "paused_by_tracker"
    | "ended";
  by: "page" | "mcp";
  target: { id: string; name: string; ra_deg: number; dec_deg: number };
  started_at: number;
  elapsed_s: number;
  settings: { subs: number | null; duration_s: number; keep_subs: boolean };
  subs: { stacked: number; dropped: Record<string, number> };
  focus?: {
    verdict: "good" | "slightly_soft" | "clearly_out" | "unmeasurable" | "no_record";
    reading?: number;
    record?: number;
    unit: "arcsec" | "px";
  };
  centring?: { tries: number; offset_arcsec: number; tolerance_arcsec: number };
  stacking_since?: number;
  limit_at?: number;
  pause_ends_at?: number;
  message: string;
  warnings?: string[];
  ended?: { reason: string; at: number; message: string; saved: boolean; track_on: boolean };
  folder?: string;
}

export interface ImagingRunRequest {
  subs?: number;
  duration_s?: number;
  keep_subs?: boolean;
}

async function imagingRunCall(url: string, init: RequestInit): Promise<ImagingView> {
  let response: Response;
  try {
    response = await fetch(url, init);
  } catch {
    throw new Error("The console did not answer. The run itself is not affected; try again.");
  }
  return json(response);
}

export async function postImagingRun(request: ImagingRunRequest): Promise<ImagingView> {
  return imagingRunCall("/api/imaging/run", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      ...(request.subs !== undefined ? { subs: request.subs } : {}),
      ...(request.duration_s !== undefined ? { duration_s: request.duration_s } : {}),
      ...(request.keep_subs !== undefined ? { keep_subs: request.keep_subs } : {}),
    }),
  });
}

export async function deleteImagingRun(): Promise<ImagingView> {
  return imagingRunCall("/api/imaging/run", { method: "DELETE" });
}

export async function postImagingRunContinue(anyway: boolean): Promise<ImagingView> {
  return imagingRunCall("/api/imaging/run/continue", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ anyway }),
  });
}

// The calibration library and sets of darks or flats in the imaging view (034).
export interface CalibrationMaster {
  id: string;
  kind: "dark" | "flat";
  made_at: number;
  frames: number;
  checked: boolean;
  gain: number | null;
  offset: number | null;
  binning: number;
  set_temp_c: number | null;
  temp_c: number | null;
  exposure_ms: number;
  bad_pixels: number | null;
  label: string | null;
  imaging_angle_deg: number | null;
}

export interface CalibrationMatch {
  frame?: number;
  dark: string | null;
  flat: string | null;
  bad_pixels: number | null;
  missing: string[];
  flat_turned_deg?: number;
}

export interface CalibrationSet {
  id: string;
  kind: "dark" | "flat";
  state: "offset" | "frames" | "search" | "needs_darks" | "flat_darks" | "done" | "failed" | "stopped";
  wanted: number;
  taken: number;
  exposure_ms: number;
  label?: string;
  offset_level?: number;
  level?: number;
  level_share?: number;
  darks_taken?: number;
  not_dark?: boolean;
  error?: string;
}

export interface ImagingCalibration {
  masters: CalibrationMaster[];
  error?: string;
  match?: CalibrationMatch;
  set?: CalibrationSet;
}

// The live stack carried by the imaging view (035).
export interface StackDropped {
  not_solved: number;
  too_few_stars: number;
  solver_failed: number;
  soft: number;
  off_field: number;
  solve_differs: number;
  settings: number;
  frame_failed: number;
  unreadable: number;
  behind: number;
  unprocessed: number;
  not_steady?: number;
}

export interface StackView {
  state: "stacking" | "ended";
  id: string;
  started_at: number;
  options: { reject: boolean; focus_gate: boolean; gains: [number, number, number]; stretch: number };
  taken: number;
  stacked: number;
  queued: number;
  dropped: StackDropped;
  exposure_s: number;
  missing?: ("dark" | "flat" | "bad_pixels")[];
  focus_median_arcsec?: number;
  preview?: string;
  ended_reason?: "stopped" | "camera";
  error?: string;
  files?: { fits?: string; png?: string; json: string };
  save_error?: string;
}

export interface StackStart {
  reject?: boolean;
  focus_gate?: boolean;
  gains?: [number, number, number];
  stretch?: number;
}

// Tracking steered by the subs, carried by the imaging and session views (040).
export interface ImagingSteering {
  smear_px: number;
  on: boolean;
  step_share: number;
  rate_share: number;
  source: "subs" | "tracker" | "none";
  holding: boolean;
  steady: boolean;
  waiting: boolean;
  pause?: string;
  subs: number;
  dropped_not_steady: number;
  far_arcsec?: number;
  last?: { time: number; east_arcsec: number; north_arcsec: number };
  exposure_s?: number;
  limited: boolean;
  over_limit: boolean;
  longest_exposure_s?: number | "no limit";
  would_take_s?: number;
  error_rms_arcsec?: number;
  error_rms_px?: number;
  drift_arcsec?: number;
  drift_px?: number;
}

// An imaging solve and what the console keeps between the two cameras, as
// GET /api/imaging relays them (032). Field names and shapes mirror the Rust
// types they come from; keep the two in step.
export interface ImagingSolveHint {
  source: "pointing" | "last_solve" | "target" | "none";
  ra_deg?: number;
  dec_deg?: number;
  radius_deg: number;
  fov_deg?: number;
}

export interface ImagingSolve {
  frame_id: number;
  state: "solving" | "solved" | "failed";
  requested_at: number;
  solver: string;
  hint: ImagingSolveHint;
  ra_deg?: number;
  dec_deg?: number;
  up_east_of_north_deg?: number;
  arcsec_per_px?: number;
  mirrored?: boolean;
  cd_deg?: number[][];
  stars?: number;
  warning?: string;
  took_ms?: number;
  file?: string;
  reason?: string;
  error?: string;
  detail?: string;
}

export interface ImagingRelation {
  scale?: { arcsec_per_px: number; frame_id: number; at?: number };
  focal_length_mm?: number;
  aim?: { x: number; y: number };
  angle_deg?: number;
  mirrored?: boolean;
  readings?: number;
  set_aside?: number;
  held?: boolean;
  last?: {
    frame_id: number;
    outcome: "measured" | "replaced" | "set_aside" | "no_live_pointing" | "moving" | "off_frame";
  };
  field?: { width_arcsec: number; height_arcsec: number };
}

// The focus reading, the focus region, the focus record and the focus aid, as
// GET /api/imaging relays them (033). Field names and shapes mirror the Rust
// types they come from; keep the two in step.
export interface ImagingFocusSized {
  value: number;
  at: number;
  // The altitude the size was read at, when known (054).
  alt_deg?: number;
}

export interface ImagingFocusAid {
  state: "starting" | "finding" | "running" | "ended";
  check: boolean;
  since: number;
  exposure_ms: number;
  readings: number;
  cadence_ms?: number;
  last?: {
    frame_id: number;
    at: number;
    state: "reading" | "settling" | "too_few_stars" | "far_out";
    value?: number;
    stars?: number;
    // The altitude the frame was read at, when known (054).
    alt_deg?: number;
  };
  median?: { value: number; readings: number };
  best?: number;
  trend?: "better" | "worse" | "same";
  found: boolean;
  verdict?: "good" | "slightly_soft" | "clearly_out" | "unmeasurable" | "no_record";
  // The record the verdict is compared with; `carried` when it was carried to the
  // reading's altitude (054).
  reference?: {
    value: number;
    of: "tonight" | "earlier";
    carried?: { from: number; from_alt_deg: number; alt_deg: number };
  };
  history: (number | null)[];
  ended_reason?: "stopped" | "done" | "failed" | "no_region";
  error?: string;
  restore_error?: string;
}

export interface ImagingFocus {
  unit: "arcsec" | "px";
  arcsec_per_px?: number;
  region?: {
    col: number;
    row: number;
    x: number;
    y: number;
    width: number;
    height: number;
    source: "found" | "fixed" | "provisional";
  };
  record?: { tonight?: ImagingFocusSized; earlier?: ImagingFocusSized };
  aid?: ImagingFocusAid;
}

export interface ImagingFocusAsk {
  exposure_ms?: number;
  find?: boolean;
  tile?: { col: number; row: number };
  check?: boolean;
}

export type ImagingSettingsEdit = Partial<Omit<ImagingSettings, "roi">>;

// The finder camera's focus aid (051): sizes in the finder camera's pixels.
export interface FinderFocus {
  state: "running" | "ended";
  since: number;
  readings: number;
  cadence_ms?: number;
  last?: {
    frame_id: number;
    state: "reading" | "settling" | "too_few_stars" | "far_out";
    value?: number;
    stars: number;
    detected: number;
  };
  median?: { value: number; readings: number };
  best?: number;
  trend?: "better" | "worse" | "same";
  found: boolean;
  history: (number | null)[];
  ended?: "stopped" | "failed";
  error?: string;
}

export async function getFinderFocus(): Promise<{ focus: FinderFocus | null }> {
  return json(await fetch("/api/finder-camera/focus"));
}

export async function postFinderFocus(): Promise<{ focus: FinderFocus | null }> {
  return json(
    await fetch("/api/finder-camera/focus", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: "{}",
    }),
  );
}

export async function deleteFinderFocus(): Promise<{ focus: FinderFocus | null }> {
  return json(await fetch("/api/finder-camera/focus", { method: "DELETE" }));
}

export async function getImaging(): Promise<ImagingView> {
  return json(await fetch("/api/imaging"));
}

export async function postImagingStack(body: StackStart): Promise<ImagingView> {
  return json(
    await fetch("/api/imaging/stack", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    }),
  );
}

export async function deleteImagingStack(): Promise<ImagingView> {
  return json(await fetch("/api/imaging/stack", { method: "DELETE" }));
}

export async function postCalibrationDarks(count: number, anyway?: boolean): Promise<ImagingCalibration> {
  return json(
    await fetch("/api/imaging/calibration/darks", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(anyway ? { count, anyway: true } : { count }),
    }),
  );
}

export async function postCalibrationFlats(count: number, label: string): Promise<ImagingCalibration> {
  return json(
    await fetch("/api/imaging/calibration/flats", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ count, label }),
    }),
  );
}

export async function postCalibrationFlatDarks(anyway?: boolean): Promise<ImagingCalibration> {
  return json(
    await fetch("/api/imaging/calibration/flats", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(anyway ? { darks: true, anyway: true } : { darks: true }),
    }),
  );
}

export async function deleteCalibration(id: string): Promise<ImagingCalibration> {
  return json(await fetch(`/api/imaging/calibration/${encodeURIComponent(id)}`, { method: "DELETE" }));
}

export async function putImagingSettings(edit: ImagingSettingsEdit): Promise<ImagingView> {
  return json(
    await fetch("/api/imaging/settings", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(edit),
    }),
  );
}

export async function putImagingSteering(edit: {
  on: boolean;
  smear_px: number;
  step_share: number;
  rate_share: number;
}): Promise<ImagingView> {
  return json(
    await fetch("/api/imaging/steering", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        on: edit.on,
        smear_px: edit.smear_px,
        step_share: edit.step_share,
        rate_share: edit.rate_share,
      }),
    }),
  );
}

export async function postImagingFrame(): Promise<ImagingView> {
  return json(await fetch("/api/imaging/frame", { method: "POST" }));
}

export async function postImagingAbort(): Promise<ImagingView> {
  return json(await fetch("/api/imaging/abort", { method: "POST" }));
}

export async function postImagingSolve(): Promise<ImagingView> {
  return json(await fetch("/api/imaging/solve", { method: "POST" }));
}

export async function postImagingFocus(ask: ImagingFocusAsk): Promise<ImagingView> {
  return json(
    await fetch("/api/imaging/focus", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(ask),
    }),
  );
}

export async function deleteImagingFocus(): Promise<ImagingView> {
  return json(await fetch("/api/imaging/focus", { method: "DELETE" }));
}

/** The settings edit leaves `roi` out, so the whole sensor is a call of its own. */
export async function putImagingWholeSensor(): Promise<ImagingView> {
  return json(
    await fetch("/api/imaging/settings", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ roi: null }),
    }),
  );
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

/** A move the console makes on its axes, as the session reports it (055). */
export interface SessionAxisMove {
  tool: string;
  state: string;
  reason?: string;
}

export interface McpStatus {
  enabled: boolean;
  local_only: boolean;
  urls: string[];
  allowed_hosts: string[];
}

export interface MountSky {
  tube_alt_deg: number;
  zero_counts: [number, number] | null;
  power_cycled?: boolean;
  night_utc?: string;
}

export interface DatabaseView {
  name: string;
  min_fov_deg: number;
  max_fov_deg: number;
}

export interface ObservingSession {
  motion?: SessionMotion;
  track?: SessionTrack;
  recording?: SessionRecording;
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
  aim?: { x: number; y: number; source?: "measured" };
  /** The imaging camera's state with its solve and relation (032). */
  imaging?: ImagingView;
  /** The last axis move the console made: a nudge's or a guard's, and its state (055). */
  axis_move?: SessionAxisMove;
  /** How far the live views are turned, degrees counter-clockwise; omitted until set. */
  chart_rotation_deg?: number;
  field?: {
    fov_deg: number | null;
    db_min_fov_deg: number;
    db_max_fov_deg: number;
    unsolved_s: number;
    database?: DatabaseView & { reason: "only" | "first" | "solved" | "field" };
    databases?: DatabaseView[];
  };
  /** The drawn sky of `--mount-sky`, present only in that mode. */
  mount_sky?: MountSky;
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
  databases?: {
    name: string;
    catalog: string;
    stars: number;
    patterns: number;
    min_fov: number;
    max_fov: number;
  }[];
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

export type NudgeDirection = "up" | "right" | "down" | "left";
export type NudgeStep = "fine" | "coarse";
/** Whose view the arrows follow: the mount's, the finder camera's or the imaging camera's (055). */
export type NudgeView = "mount" | "finder_camera" | "imaging_camera";

/** The step a press made: the offset in the mount's view, and whether Track made it (055). */
export interface Nudged {
  up_arcsec: number;
  right_arcsec: number;
  moved: "axes" | "track";
}

export async function postNudge(direction: NudgeDirection, step: NudgeStep, view: NudgeView): Promise<Nudged> {
  return json(
    await fetch("/api/mount/nudge", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ direction, step, view }),
    }),
  );
}

export async function postRecord(on: boolean): Promise<ObservingSession> {
  return json(
    await fetch("/api/record", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ on }),
    }),
  );
}

export async function postRecordRaw(frames: number): Promise<ObservingSession> {
  return json(
    await fetch("/api/record/raw", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ frames }),
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
