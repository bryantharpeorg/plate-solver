//! The imaging run (037): one procedure from a chosen target to a saved stack, carried
//! through five parts the console hands it. It moves the mount only by asking its mount
//! part for Track, and asks nothing of the mount when it ends, but a Stop when it is
//! stopped while it still slews.

pub mod script;

use std::{collections::BTreeMap, future::Future, pin::Pin, sync::Arc, time::Duration};

use serde::{Deserialize, Serialize};

/// How often a run reads the mount and the stack.
pub const POLL: Duration = Duration::from_millis(250);
/// The time limit of a run whose request gives none, in seconds of stacking.
pub const DEFAULT_DURATION_S: f64 = 7200.0;
pub const MAX_DURATION_S: f64 = 43_200.0;
pub const MAX_SUBS: u32 = 100_000;

/// The state words, in the order a run can pass through them.
pub const STATES: [&str; 10] = [
    "slewing",
    "tracking",
    "first_frame",
    "focus_check",
    "focus_pause",
    "solving",
    "centring",
    "stacking",
    "paused_by_tracker",
    "ended",
];
/// The ending reasons.
pub const ENDINGS: [&str; 17] = [
    "stopped",
    "count_limit",
    "duration_limit",
    "too_low",
    "sun_up",
    "link_lost",
    "mount_failed",
    "target_changed",
    "pushed_away",
    "camera_failed",
    "solver_unavailable",
    "solver_failing",
    "save_failed",
    "focus_timeout",
    "not_centred",
    "subs_failing",
    "stack_failed",
];

pub type StepFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum By {
    Page,
    Mcp,
}

impl By {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Page => "page",
            Self::Mcp => "mcp",
        }
    }
}

/// Why a run is over. The whole enum, its reasons and its messages are here from US12;
/// a later story adds only the code that reaches a variant.
#[derive(Debug, Clone, PartialEq)]
pub enum Ending {
    /// The mount was stopped: the console's Stop, or Track turned off.
    Stopped,
    /// The run's own stop, with the mount left as it was. Its reason is `stopped` too.
    RunStopped,
    /// The stack was stopped from its own route, or ended with no failure. `stopped`.
    StackStopped,
    /// The run's focus check was stopped from 033's own route. `stopped`.
    CheckStopped,
    CountLimit,
    DurationLimit,
    TooLow,
    SunUp,
    LinkLost,
    /// With the words Track or GoTo gave.
    MountFailed(String),
    TargetChanged,
    /// 038's Track view shows a push by hand that settled or lasted.
    PushedAway,
    CameraFailed(String),
    SolverUnavailable(String),
    SolverFailing(String),
    SaveFailed(String),
    FocusTimeout,
    /// `measured` is false when the run ended on solves that measured no new aim point.
    NotCentred {
        offset_arcsec: f64,
        measured: bool,
    },
    /// With the reasons that rose, as `not solved 7, too few stars 3`.
    SubsFailing(String),
    StackFailed(String),
}

impl Ending {
    /// Its word in `ENDINGS`. Four variants answer `stopped`.
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Stopped | Self::RunStopped | Self::StackStopped | Self::CheckStopped => "stopped",
            Self::CountLimit => "count_limit",
            Self::DurationLimit => "duration_limit",
            Self::TooLow => "too_low",
            Self::SunUp => "sun_up",
            Self::LinkLost => "link_lost",
            Self::MountFailed(_) => "mount_failed",
            Self::TargetChanged => "target_changed",
            Self::PushedAway => "pushed_away",
            Self::CameraFailed(_) => "camera_failed",
            Self::SolverUnavailable(_) => "solver_unavailable",
            Self::SolverFailing(_) => "solver_failing",
            Self::SaveFailed(_) => "save_failed",
            Self::FocusTimeout => "focus_timeout",
            Self::NotCentred { .. } => "not_centred",
            Self::SubsFailing(_) => "subs_failing",
            Self::StackFailed(_) => "stack_failed",
        }
    }

    /// The sentence of the run's words. `name` is the target's.
    pub fn message(&self, name: &str, settings: &RunSettings) -> String {
        match self {
            Self::Stopped => "Stopped: the mount was stopped.".into(),
            Self::RunStopped => "Imaging run stopped. Track is still on.".into(),
            Self::StackStopped => "Ended: the stack was stopped.".into(),
            Self::CheckStopped => "Ended: the focus check was stopped.".into(),
            Self::CountLimit => match settings.subs {
                Some(1) => "Finished: the 1 sub asked for is stacked.".into(),
                n => format!("Finished: the {} subs asked for are stacked.", n.unwrap_or(0)),
            },
            Self::DurationLimit => {
                let minutes = format!("{:.1}", (settings.duration_s / 60.0).max(0.1));
                let minutes = minutes.strip_suffix(".0").unwrap_or(&minutes);
                format!("Finished: the {minutes}-minute limit was reached.")
            }
            Self::TooLow => format!("Ended: {name} is below the minimum altitude."),
            Self::SunUp => "Ended: the Sun is up.".into(),
            Self::LinkLost => "Ended: the mount or the scope link stopped answering. Check the scope node and the mount's cable, then start again.".into(),
            Self::MountFailed(words) => format!("Ended: Track ended and the run with it: {words}. Check the mount, then start again."),
            Self::TargetChanged => "Ended: the target was changed. Start a new imaging run for the new target.".into(),
            Self::PushedAway => format!("Ended: the telescope was pushed away from {name}. Track follows where it is left. Start a new imaging run to go back to {name}."),
            Self::CameraFailed(words) => format!("Ended: the imaging camera failed: {words}."),
            Self::SolverUnavailable(words) => format!("Ended: the imaging solver cannot run: {words}."),
            Self::SolverFailing(words) => format!("Ended: 3 imaging frames in a row could not be solved: {words}. Check for cloud, dew and focus."),
            Self::SaveFailed(words) => format!("Ended: the console could not save to its imaging folder: {words}. Check the disk."),
            Self::FocusTimeout => "Ended: focus was left paused for 15 minutes. Start again when ready.".into(),
            Self::NotCentred { offset_arcsec, measured } => {
                let offset = if *offset_arcsec < 600.0 {
                    format!("{offset_arcsec:.0}″")
                } else {
                    format!("{:.1}′", offset_arcsec / 60.0)
                };
                if *measured {
                    format!("Ended: {name} is still {offset} from the sensor's centre after 3 tries. Check the aim point on the console page, then start again.")
                } else {
                    format!("Ended: {name} is {offset} from the sensor's centre and no new aim point could be measured.")
                }
            }
            Self::SubsFailing(words) => format!("Ended: no sub was stacked for 10 minutes ({words}). Cloud, dew or a tree is the usual cause; check the sky and start again."),
            Self::StackFailed(words) => format!("Ended: the stack stopped: {words}."),
        }
    }
}

/// Why a part did not do what it was asked.
#[derive(Debug, Clone, PartialEq)]
pub enum Failure {
    /// The part was stopped from outside the run: the console's Stop for the mount,
    /// 033's own stop for a focus check.
    Stopped,
    Camera(String),
    Save(String),
    /// The imaging solver cannot run at all.
    SolverUnavailable(String),
    /// The imaging solver ran and found no solution; or a focus check found no stars
    /// to measure.
    NotSolved(String),
    Mount(String),
    Other(String),
}

impl Failure {
    pub fn words(&self) -> &str {
        match self {
            Self::Stopped => "stopped",
            Self::Camera(words)
            | Self::Save(words)
            | Self::SolverUnavailable(words)
            | Self::NotSolved(words)
            | Self::Mount(words)
            | Self::Other(words) => words,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RunTarget {
    pub id: String,
    pub name: String,
    pub ra_deg: f64,
    pub dec_deg: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MountState {
    /// The mount is being brought to the target.
    Slewing,
    /// `steady` is false while the tracker says it is not steady enough to expose.
    Tracking { steady: bool },
    /// The motion is over, and why.
    Ended(Ending),
}

pub trait MountSteps: Send + Sync {
    /// The target a run would go to, or why there is none.
    fn target(&self) -> Result<RunTarget, String>;
    /// Ask Track for that target. A refusal's words, unchanged.
    fn point(&self, by: By) -> StepFuture<'_, Result<(), String>>;
    fn state(&self) -> MountState;
    /// Ask Track for the target again, so that it comes to the aim point in force.
    fn repoint(&self) -> StepFuture<'_, Result<(), Failure>>;
    /// Carry out the console's Stop. The run asks only when it is stopped while it
    /// still slews.
    fn stop(&self) -> StepFuture<'_, ()>;
    /// Whether Track is on now, in any state but off.
    fn track_on(&self) -> bool;
    /// Whether the tracker was steady for all of a window of console Unix seconds.
    fn steady_between(&self, start: f64, end: f64) -> bool;
}

#[derive(Debug, Clone, PartialEq)]
pub struct FrameRef {
    pub id: u64,
    pub file: String,
}

pub trait FrameSteps: Send + Sync {
    /// Whether the imaging camera can be used at all, or why not.
    fn ready(&self) -> Result<(), String>;
    /// Take the one lease of the imaging camera for this run. A refusal's words,
    /// unchanged.
    fn hold(&self) -> Result<(), String>;
    /// Give the lease back. Safe when it is not held.
    fn release(&self);
    /// Take one imaging frame and wait until it is saved.
    fn take(&self) -> StepFuture<'_, Result<FrameRef, Failure>>;
    /// Give up the exposure in flight.
    fn abort(&self) -> StepFuture<'_, ()>;
}

/// 033's five verdicts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Good,
    SlightlySoft,
    ClearlyOut,
    Unmeasurable,
    NoRecord,
}

impl Verdict {
    /// `good`, `slightly_soft`, `clearly_out`, `unmeasurable` or `no_record`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Good => "good",
            Self::SlightlySoft => "slightly_soft",
            Self::ClearlyOut => "clearly_out",
            Self::Unmeasurable => "unmeasurable",
            Self::NoRecord => "no_record",
        }
    }
}

/// The unit a focus reading and a focus record are in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Arcsec,
    Px,
}

impl Unit {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Arcsec => "arcsec",
            Self::Px => "px",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FocusCheck {
    pub verdict: Verdict,
    pub reading: Option<f64>,
    pub record: Option<f64>,
    pub unit: Unit,
}

pub trait FocusSteps: Send + Sync {
    /// Check focus and wait for the verdict.
    fn check(&self) -> StepFuture<'_, Result<FocusCheck, Failure>>;
    fn open_aid(&self) -> StepFuture<'_, Result<(), Failure>>;
    /// Close the focus aid, or stop a check that is running. Safe when neither runs.
    fn close_aid(&self) -> StepFuture<'_, ()>;
}

/// How far the target is from the set point, by one imaging solve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Centre {
    pub offset_arcsec: f64,
    pub tolerance_arcsec: f64,
    /// A quarter of the imaging frame's shorter side on the sky: a target this near
    /// after the last try is stacked with a warning.
    pub near_arcsec: f64,
    /// Whether the solve moved the aim point, or left a measured one held back until
    /// Track is off: a re-centre is asked for only then.
    pub aim_moved: bool,
}

pub trait SolveSteps: Send + Sync {
    fn ready(&self) -> Result<(), String>;
    fn solve(
        &self,
        frame: &FrameRef,
        target: &RunTarget,
    ) -> StepFuture<'_, Result<Centre, Failure>>;
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct StackProgress {
    pub running: bool,
    pub stacked: u64,
    /// By the stack's own reason words; only reasons whose count is above 0.
    pub dropped: BTreeMap<String, u64>,
    pub waiting_for_tracker: bool,
    /// Why the stack ended by itself, when it did.
    pub failed: Option<Failure>,
}

pub trait StackSteps: Send + Sync {
    /// Start a stack on the target. Answers the stack's folder, relative to the
    /// imaging folder.
    fn start(&self, target: &RunTarget) -> StepFuture<'_, Result<String, Failure>>;
    fn progress(&self) -> StackProgress;
    /// Stop taking subs and save the stack; with `keep_subs` false, then remove the subs.
    fn stop(&self, keep_subs: bool) -> StepFuture<'_, Result<(), Failure>>;
}

/// The five parts. The five traits and this struct are complete from US12: no later
/// story or spec adds a method or a field, because the script implements them all.
#[derive(Clone)]
pub struct Steps {
    pub mount: Arc<dyn MountSteps>,
    pub camera: Arc<dyn FrameSteps>,
    pub focus: Arc<dyn FocusSteps>,
    pub solver: Arc<dyn SolveSteps>,
    pub stack: Arc<dyn StackSteps>,
}

/// The body of `POST /api/imaging/run`.
#[derive(Debug, Default, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunRequest {
    pub subs: Option<u32>,
    pub duration_s: Option<f64>,
    pub keep_subs: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RunSettings {
    pub subs: Option<u32>,
    pub duration_s: f64,
    pub keep_subs: bool,
}

/// A target's name as a folder name takes it: every character that is not an ASCII
/// letter or digit made `-`. Calls 035's `crate::imaging::stack::service::label`.
pub fn label(name: &str) -> String {
    crate::imaging::stack::service::label(name)
}

/// `<UTC start as YYYY-MM-DDTHH-MM-SSZ>-<label(name)>`.
pub fn run_id(started_at: f64, name: &str) -> String {
    let (_, utc) = crate::imaging::fits::utc(started_at);
    format!("{}Z-{}", utc[..19].replace(':', "-"), label(name))
}
