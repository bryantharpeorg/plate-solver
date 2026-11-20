//! The imaging run (037): one procedure from a chosen target to a saved stack, carried
//! through five parts the console hands it. It moves the mount only by asking its mount
//! part for Track, and asks nothing of the mount when it ends, but a Stop when it is
//! stopped while it still slews.

pub mod console;
pub mod script;
pub mod store;

use std::{
    collections::BTreeMap,
    future::Future,
    path::PathBuf,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use tokio::{
    sync::{watch, Mutex as AsyncMutex},
    task::JoinHandle,
    time::{self, Instant},
};

use self::store::{Event, Store};
use super::Refusal;
use crate::session::Clock;

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

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct SubCounts {
    pub stacked: u64,
    pub dropped: BTreeMap<String, u64>,
}

/// `reading` and `record` are skipped while `None`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FocusNote {
    pub verdict: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reading: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record: Option<f64>,
    pub unit: &'static str,
}

impl From<FocusCheck> for FocusNote {
    fn from(check: FocusCheck) -> Self {
        Self {
            verdict: check.verdict.as_str(),
            reading: check.reading,
            record: check.record,
            unit: check.unit.as_str(),
        }
    }
}

impl FocusNote {
    fn sentence(&self) -> String {
        let reading = |value| match self.unit {
            "arcsec" => format!("{value:.1}″"),
            _ => format!("{value:.1} px"),
        };
        match (self.verdict, self.reading, self.record) {
            ("slightly_soft", Some(value), Some(record)) => format!(
                "Focus is slightly soft: stars are {} across against a record of {}.",
                reading(value),
                reading(record)
            ),
            ("slightly_soft", _, _) => "Focus is slightly soft.".into(),
            ("clearly_out", Some(value), Some(record)) => format!(
                "Focus is clearly out: stars are {} across against a record of {}.",
                reading(value),
                reading(record)
            ),
            ("clearly_out", Some(value), None) => {
                format!("Focus is clearly out: stars are {} across.", reading(value))
            }
            ("clearly_out", None, _) => "Focus is clearly out.".into(),
            ("unmeasurable", _, _) => "Focus could not be measured: too few stars.".into(),
            ("no_record", _, _) => "There is no focus record for this camera yet.".into(),
            _ => String::new(),
        }
    }

    fn pause_message(&self, failure: Option<&Failure>) -> String {
        if let Some(failure) = failure.filter(|failure| !matches!(failure, Failure::NotSolved(_))) {
            return format!(
                "The focus check did not finish: {}. Check the imaging camera, then check focus again or go on anyway.",
                failure.words()
            );
        }
        let advice = match self.verdict {
            "unmeasurable" => "Check for cloud or dew, then check focus again or go on anyway.",
            "no_record" => "Focus with the focus aid, then check focus again or go on anyway.",
            _ => "Turn the focuser while watching the focus aid below, then check focus again or go on anyway.",
        };
        format!("{} {advice}", self.sentence())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CentringNote {
    pub tries: u32,
    pub offset_arcsec: f64,
    pub tolerance_arcsec: f64,
}

/// `saved`: the run started a stack and `stack.stop` answered `Ok`. `track_on`:
/// Track was on at the ending and has been ever since.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EndedNote {
    pub reason: &'static str,
    pub at: f64,
    pub message: String,
    pub saved: bool,
    pub track_on: bool,
}

/// A warning a run keeps. Later stories reach the remaining variants.
#[derive(Debug, Clone, PartialEq)]
pub enum Warning {
    SlightlySoft(FocusCheck),
    CheckUnfinished(String),
    WentOnAnyway,
    AidNotOpened(String),
    StackUnclean(String),
    SubsKept,
    RecordUnsaved(String),
    OffCentre { offset_arcsec: f64, measured: bool },
    TrackerNotSteady,
}

impl Warning {
    pub fn words(&self, name: &str) -> String {
        match self {
            Self::SlightlySoft(check) => FocusNote::from(*check).sentence(),
            Self::CheckUnfinished(words) => format!("The focus check did not finish: {words}."),
            Self::WentOnAnyway => {
                "Focus was not checked again: the observer went on anyway.".into()
            }
            Self::AidNotOpened(words) => format!("The focus aid could not be opened: {words}."),
            Self::StackUnclean(words) => format!("The stack did not end cleanly: {words}."),
            Self::SubsKept => "The subs were kept: nothing was stacked.".into(),
            Self::RecordUnsaved(words) => format!("The run's record could not be saved: {words}."),
            Self::OffCentre {
                offset_arcsec,
                measured,
            } => {
                let offset = offset_words(*offset_arcsec);
                let after = if *measured {
                    "after 3 tries"
                } else {
                    "and no new aim point could be measured"
                };
                let steering = if *offset_arcsec <= crate::imaging::steering::FAR_ARCSEC {
                    "the stack's steering will bring it in"
                } else {
                    "it is stacked where it is"
                };
                format!("{name} is {offset} from the sensor's centre {after}; {steering}.")
            }
            Self::TrackerNotSteady => "The tracker was not steady during the focus check.".into(),
        }
    }
}

fn offset_words(offset_arcsec: f64) -> String {
    if offset_arcsec < 600.0 {
        format!("{offset_arcsec:.0}″")
    } else {
        format!("{:.1}′", offset_arcsec / 60.0)
    }
}

/// `run` in the imaging view. Absent values and empty warnings are omitted;
/// `settings.subs` remains null when no number was asked for.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct RunView {
    pub id: String,
    pub state: &'static str,
    pub by: &'static str,
    pub target: RunTarget,
    pub started_at: f64,
    pub elapsed_s: f64,
    pub settings: RunSettings,
    pub subs: SubCounts,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus: Option<FocusNote>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub centring: Option<CentringNote>,
    /// Console Unix seconds at which the stack started.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stacking_since: Option<f64>,
    /// `stacking_since` plus the requested duration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_at: Option<f64>,
    /// Console Unix seconds; only present during a focus pause (041).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pause_ends_at: Option<f64>,
    pub message: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended: Option<EndedNote>,
    /// Relative to the imaging folder.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
}

impl RunView {
    fn state_message(&self, focus_failure: Option<&Failure>, aim_moved: bool) -> String {
        let name = &self.target.name;
        match self.state {
            "slewing" => format!("Slewing to {name}."),
            "tracking" => {
                format!("Tracking {name}. Waiting for the mount to settle; nothing to do.")
            }
            "first_frame" => "Taking a first imaging frame.".into(),
            "focus_check" => "Checking focus.".into(),
            "focus_pause" => self
                .focus
                .as_ref()
                .map_or_else(String::new, |focus| focus.pause_message(focus_failure)),
            "solving" => "Solving an imaging frame.".into(),
            "centring" => self.centring.as_ref().map_or_else(String::new, |centre| {
                let offset = offset_words(centre.offset_arcsec);
                if aim_moved {
                    format!(
                        "Centring {name}: {offset} from the sensor's centre, try {} of 3.",
                        centre.tries
                    )
                } else {
                    format!(
                        "Centring {name}: {offset} from the sensor's centre, no new aim point yet."
                    )
                }
            }),
            "stacking" => format!(
                "Stacking {name}: {} stacked, {} dropped.",
                self.subs.stacked,
                self.subs.dropped.values().sum::<u64>()
            ),
            "paused_by_tracker" => format!(
                "Exposures paused: {} stacked so far. They resume by themselves.",
                self.subs.stacked
            ),
            "ended" => self
                .ended
                .as_ref()
                .map_or_else(String::new, |ended| ended.message.clone()),
            _ => String::new(),
        }
    }
}

/// Only the latest run is shared with its task. Part calls and awaits always
/// happen outside this mutex, since a part may itself read the imaging view.
struct ActiveRun {
    view: RunView,
    started: Instant,
    finished: Option<Instant>,
    stop: watch::Sender<bool>,
    done: Arc<watch::Sender<bool>>,
    focus_failure: Option<Failure>,
    aim_moved: bool,
    events: Vec<Event>,
    store: Option<Store>,
    last_write: Option<(Instant, u64)>,
    tracker_event: Option<Instant>,
}

impl ActiveRun {
    /// Only actual writes reset the refresh clock and the saved count.
    fn write_record(&mut self) -> std::io::Result<()> {
        if let Some(store) = self.store.as_ref().filter(|store| store.folder().is_some()) {
            store.write(&self.view, &self.events)?;
            self.last_write = Some((Instant::now(), self.view.subs.stacked));
        }
        Ok(())
    }
}

/// Carries one imaging run through the five injected parts.
pub struct Runner {
    steps: Steps,
    clock: Arc<dyn Clock>,
    root: Option<PathBuf>,
    unfinished: Mutex<Option<String>>,
    starting: AsyncMutex<()>,
    inner: Arc<Mutex<Option<ActiveRun>>>,
    task: Mutex<Option<JoinHandle<()>>>,
}

impl Drop for Runner {
    fn drop(&mut self) {
        if let Some(task) = self.task.get_mut().unwrap().take() {
            task.abort();
        }
    }
}

impl Runner {
    pub fn new(steps: Steps, clock: Arc<dyn Clock>) -> Arc<Self> {
        Self::build(steps, clock, None)
    }

    /// As `new`, keeping each run's record under the imaging folder.
    pub fn with_folder(steps: Steps, clock: Arc<dyn Clock>, root: PathBuf) -> Arc<Self> {
        Self::build(steps, clock, Some(root))
    }

    fn build(steps: Steps, clock: Arc<dyn Clock>, root: Option<PathBuf>) -> Arc<Self> {
        let unfinished = root.as_deref().and_then(store::unfinished);
        Arc::new(Self {
            steps,
            clock,
            root,
            unfinished: Mutex::new(unfinished),
            starting: AsyncMutex::new(()),
            inner: Arc::new(Mutex::new(None)),
            task: Mutex::new(None),
        })
    }

    /// The newest unfinished record's folder, until the first accepted start.
    pub fn unfinished(&self) -> Option<String> {
        self.unfinished.lock().unwrap().clone()
    }

    /// Check, take the lease, ask Track, and start the run's task.
    pub async fn start(self: &Arc<Self>, request: RunRequest, by: By) -> Result<RunView, Refusal> {
        if request
            .subs
            .is_some_and(|subs| !(1..=MAX_SUBS).contains(&subs))
        {
            return Err(Refusal::bad_request(
                "subs must be a whole number from 1 to 100000",
            ));
        }
        if request.duration_s.is_some_and(|duration| {
            !duration.is_finite() || !(1.0..=MAX_DURATION_S).contains(&duration)
        }) {
            return Err(Refusal::bad_request(
                "duration_s must be a number from 1 to 43200",
            ));
        }

        let _starting = self.starting.lock().await;
        let running = self
            .inner
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|run| run.finished.is_none());
        if running {
            return Err(Refusal::conflict("an imaging run is already running"));
        }
        self.steps.camera.ready().map_err(Refusal::conflict)?;
        self.steps.solver.ready().map_err(Refusal::conflict)?;
        let target = self.steps.mount.target().map_err(Refusal::conflict)?;
        self.steps.camera.hold().map_err(Refusal::conflict)?;
        if let Err(words) = self.steps.mount.point(by).await {
            self.steps.camera.release();
            return Err(Refusal::conflict(words));
        }

        *self.unfinished.lock().unwrap() = None;
        let started_at = self.clock.now();
        let started = Instant::now();
        let mut view = RunView {
            id: run_id(started_at, &target.name),
            state: "slewing",
            by: by.as_str(),
            target,
            started_at,
            elapsed_s: 0.0,
            settings: RunSettings {
                subs: request.subs,
                duration_s: request.duration_s.unwrap_or(DEFAULT_DURATION_S),
                keep_subs: request.keep_subs.unwrap_or(true),
            },
            subs: SubCounts::default(),
            focus: None,
            centring: None,
            stacking_since: None,
            limit_at: None,
            pause_ends_at: None,
            message: String::new(),
            warnings: Vec::new(),
            ended: None,
            folder: None,
        };
        view.message = view.state_message(None, true);
        let (stop, stopped) = watch::channel(false);
        let (done, _) = watch::channel(false);
        *self.inner.lock().unwrap() = Some(ActiveRun {
            view: view.clone(),
            started,
            finished: None,
            stop,
            done: Arc::new(done),
            focus_failure: None,
            aim_moved: true,
            events: vec![Event {
                at: started_at,
                state: "slewing",
            }],
            store: self.root.clone().map(Store::new),
            last_write: None,
            tracker_event: None,
        });
        // The task owns only the inner state and the parts, never the Runner.
        let task = RunTask {
            steps: self.steps.clone(),
            clock: self.clock.clone(),
            inner: self.inner.clone(),
            stopped,
            abandoned_check: false,
            stack_started: false,
        };
        *self.task.lock().unwrap() = Some(tokio::spawn(task.run()));
        Ok(view)
    }

    /// End the run that is on, then answer its latest view.
    pub async fn stop(&self) -> Option<RunView> {
        // A start still asking Track must install its run before we signal it.
        let starting = self.starting.lock().await;
        let signals = self.inner.lock().unwrap().as_ref().and_then(|run| {
            run.finished
                .is_none()
                .then(|| (run.stop.clone(), run.done.subscribe()))
        });
        drop(starting);
        if let Some((stop, mut done)) = signals {
            stop.send_replace(true);
            let _ = done.wait_for(|finished| *finished).await;
        }
        self.view()
    }

    /// Going on from a focus pause is added by 041; until then this refuses.
    pub fn resume(&self, _anyway: bool) -> Result<RunView, Refusal> {
        let running = self
            .inner
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|run| run.finished.is_none());
        Err(Refusal::conflict(if running {
            "the imaging run is not paused"
        } else {
            "no imaging run is running"
        }))
    }

    /// The latest run, with elapsed time measured on Tokio's clock.
    pub fn view(&self) -> Option<RunView> {
        let (mut view, started, finished, identity) = {
            let inner = self.inner.lock().unwrap();
            let run = inner.as_ref()?;
            (
                run.view.clone(),
                run.started,
                run.finished,
                run.done.clone(),
            )
        };
        view.elapsed_s = finished
            .unwrap_or_else(Instant::now)
            .duration_since(started)
            .as_secs_f64();
        if view.ended.as_ref().is_some_and(|ended| ended.track_on) && !self.steps.mount.track_on() {
            view.ended.as_mut().unwrap().track_on = false;
            let mut inner = self.inner.lock().unwrap();
            if let Some(run) = inner
                .as_mut()
                .filter(|run| Arc::ptr_eq(&run.done, &identity))
            {
                run.view.ended.as_mut().unwrap().track_on = false;
            }
        }
        Some(view)
    }
}

struct RunTask {
    steps: Steps,
    clock: Arc<dyn Clock>,
    inner: Arc<Mutex<Option<ActiveRun>>>,
    stopped: watch::Receiver<bool>,
    abandoned_check: bool,
    stack_started: bool,
}

impl RunTask {
    fn update(&self, change: impl FnOnce(&mut ActiveRun)) {
        let mut inner = self.inner.lock().unwrap();
        change(inner.as_mut().unwrap());
    }

    fn set_state(&self, state: &'static str) -> Result<(), Ending> {
        let at = self.clock.now();
        let now = Instant::now();
        let mut inner = self.inner.lock().unwrap();
        let run = inner.as_mut().unwrap();
        let changed = run.view.state != state;
        let tracker_change = matches!(
            (run.view.state, state),
            ("stacking", "paused_by_tracker") | ("paused_by_tracker", "stacking")
        );
        run.view.state = state;
        run.view.message = run
            .view
            .state_message(run.focus_failure.as_ref(), run.aim_moved);
        if changed
            && (!tracker_change
                || run
                    .tracker_event
                    .is_none_or(|last| now.duration_since(last) >= RECORD_EVERY))
        {
            run.events.push(Event { at, state });
            if tracker_change {
                run.tracker_event = Some(now);
            }
            run.write_record()
                .map_err(|error| Ending::SaveFailed(error.to_string()))?;
        }
        Ok(())
    }

    fn refresh_record(&self) -> Result<(), Ending> {
        let mut inner = self.inner.lock().unwrap();
        let run = inner.as_mut().unwrap();
        if run.last_write.is_some_and(|(last, stacked)| {
            run.view.subs.stacked > stacked && last.elapsed() >= RECORD_EVERY
        }) {
            run.write_record()
                .map_err(|error| Ending::SaveFailed(error.to_string()))?;
        }
        Ok(())
    }

    fn waiting_for_tracker(&self, waiting: bool) {
        self.update(|run| {
            run.view.message = run
                .view
                .state_message(run.focus_failure.as_ref(), run.aim_moved);
            if waiting
                && matches!(
                    run.view.state,
                    "first_frame" | "focus_check" | "solving" | "centring"
                )
            {
                run.view
                    .message
                    .push_str(" Waiting for the mount to settle; nothing to do.");
            }
        });
    }

    /// Copy the counts and answer whether the stack waits for the tracker. Read
    /// outside the run's lock: the stack part may read the imaging view.
    fn read_stack_counts(&self) -> bool {
        let progress = self.steps.stack.progress();
        self.update(|run| {
            run.view.subs = SubCounts {
                stacked: progress.stacked,
                dropped: progress.dropped,
            };
        });
        progress.waiting_for_tracker
    }

    fn warn(&self, warning: Warning) {
        self.update(|run| {
            let words = warning.words(&run.view.target.name);
            if !run.view.warnings.contains(&words) {
                run.view.warnings.push(words);
            }
        });
    }

    /// Poll the mount even while a part's future is held. Callers put the part
    /// call inside an async block so stop and motion endings are checked first.
    async fn guarded<T>(&self, step: impl Future<Output = T>) -> Result<T, Ending> {
        let mut stopped = self.stopped.clone();
        let mut poll = time::interval(POLL);
        tokio::pin!(step);
        loop {
            tokio::select! {
                biased;
                _ = stopped.wait_for(|stop| *stop) => return Err(Ending::RunStopped),
                _ = poll.tick() => {
                    if let MountState::Ended(ending) = self.steps.mount.state() {
                        return Err(ending);
                    }
                }
                answer = &mut step => return Ok(answer),
            }
        }
    }

    async fn steady(&self, show: Option<(&'static str, &'static str)>) -> Result<(), Ending> {
        let mut stopped = self.stopped.clone();
        loop {
            if *stopped.borrow() {
                let slewing = self.inner.lock().unwrap().as_ref().unwrap().view.state == "slewing";
                if slewing {
                    self.steps.mount.stop().await;
                    return Err(Ending::Stopped);
                }
                return Err(Ending::RunStopped);
            }
            match self.steps.mount.state() {
                MountState::Ended(ending) => return Err(ending),
                MountState::Tracking { steady: true } => {
                    if show.is_none() {
                        self.waiting_for_tracker(false);
                    }
                    return Ok(());
                }
                MountState::Slewing => {
                    if let Some((slewing, _)) = show {
                        self.set_state(slewing)?;
                    }
                }
                MountState::Tracking { steady: false } => {
                    if let Some((_, tracking)) = show {
                        self.set_state(tracking)?;
                    }
                }
            }
            if show.is_none() {
                self.waiting_for_tracker(true);
            }
            tokio::select! {
                biased;
                _ = stopped.wait_for(|stop| *stop) => {},
                _ = time::sleep(POLL) => {},
            }
        }
    }

    async fn frame(&self) -> Result<FrameRef, Ending> {
        let mut failures = 0;
        loop {
            self.steady(None).await?;
            let mut taking = false;
            let answer = self
                .guarded(async {
                    taking = true;
                    self.steps.camera.take().await
                })
                .await;
            match answer {
                Err(ending) => {
                    if taking {
                        self.steps.camera.abort().await;
                    }
                    return Err(ending);
                }
                Ok(Ok(frame)) => return Ok(frame),
                Ok(Err(Failure::Stopped)) => return Err(Ending::Stopped),
                Ok(Err(Failure::Save(words))) => return Err(Ending::SaveFailed(words)),
                Ok(Err(failure)) => {
                    failures += 1;
                    if failures == FRAME_TRIES {
                        return Err(Ending::CameraFailed(failure.words().into()));
                    }
                    self.guarded(time::sleep(RETRY_WAIT)).await?;
                }
            }
        }
    }

    async fn run(mut self) {
        let ending = self.procedure().await.unwrap_err();
        self.end(ending).await;
    }

    async fn procedure(&mut self) -> Result<(), Ending> {
        self.steady(Some(("slewing", "tracking"))).await?;
        self.set_state("first_frame")?;
        self.frame().await?;

        self.set_state("focus_check")?;
        self.steady(None).await?;
        self.steps.camera.release();
        let mut checking = false;
        let answer = self
            .guarded(async {
                checking = true;
                self.steps.focus.check().await
            })
            .await;
        let check = match answer {
            Err(ending) => {
                self.abandoned_check = checking;
                return Err(ending);
            }
            Ok(Ok(check)) => check,
            Ok(Err(Failure::Stopped)) => return Err(Ending::CheckStopped),
            Ok(Err(Failure::Save(words))) => return Err(Ending::SaveFailed(words)),
            Ok(Err(Failure::SolverUnavailable(words))) => {
                return Err(Ending::SolverUnavailable(words));
            }
            Ok(Err(failure)) => {
                self.warn(Warning::CheckUnfinished(failure.words().into()));
                self.update(|run| run.focus_failure = Some(failure));
                FocusCheck {
                    verdict: Verdict::Unmeasurable,
                    reading: None,
                    record: None,
                    unit: Unit::Px,
                }
            }
        };
        self.update(|run| run.view.focus = Some(check.into()));
        if check.verdict == Verdict::SlightlySoft {
            self.warn(Warning::SlightlySoft(check));
        }
        self.steps.camera.hold().map_err(Ending::CameraFailed)?;

        self.set_state("solving")?;
        let target = self
            .inner
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .view
            .target
            .clone();
        let mut failures = 0;
        let centre = loop {
            let frame = self.frame().await?;
            match self
                .guarded(async { self.steps.solver.solve(&frame, &target).await })
                .await?
            {
                Ok(centre) => break centre,
                Err(Failure::Stopped) => return Err(Ending::Stopped),
                Err(Failure::Save(words)) => return Err(Ending::SaveFailed(words)),
                Err(Failure::SolverUnavailable(words)) => {
                    return Err(Ending::SolverUnavailable(words));
                }
                Err(failure) => {
                    failures += 1;
                    if failures == SOLVE_TRIES {
                        return Err(Ending::SolverFailing(failure.words().into()));
                    }
                }
            }
        };
        self.update(|run| {
            run.aim_moved = centre.aim_moved;
            run.view.centring = Some(CentringNote {
                tries: 0,
                offset_arcsec: centre.offset_arcsec,
                tolerance_arcsec: centre.tolerance_arcsec,
            });
        });

        self.set_state("stacking")?;
        self.steps.camera.release();
        let folder = match self.steps.stack.start(&target).await {
            Ok(folder) => folder,
            Err(Failure::Stopped) => return Err(Ending::Stopped),
            Err(Failure::Camera(words)) => return Err(Ending::CameraFailed(words)),
            Err(Failure::Save(words)) => return Err(Ending::SaveFailed(words)),
            Err(Failure::SolverUnavailable(words)) => {
                return Err(Ending::SolverUnavailable(words));
            }
            Err(Failure::NotSolved(words)) => return Err(Ending::SolverFailing(words)),
            Err(failure) => return Err(Ending::StackFailed(failure.words().into())),
        };
        self.stack_started = true;
        let stacking_since = self.clock.now();
        self.update(|run| {
            if let Some(store) = &mut run.store {
                store.adopt(&folder);
            }
            run.view.folder = Some(folder);
            run.view.stacking_since = Some(stacking_since);
            run.view.limit_at = Some(stacking_since + run.view.settings.duration_s);
        });

        {
            let mut inner = self.inner.lock().unwrap();
            inner
                .as_mut()
                .unwrap()
                .write_record()
                .map_err(|error| Ending::SaveFailed(error.to_string()))?;
        }
        let mut stopped = self.stopped.clone();
        loop {
            let waiting_for_tracker = self.read_stack_counts();
            // Once stacking, the mount's ending takes precedence over a stop
            // observed at the same poll, including after an unguarded start.
            if let MountState::Ended(ending) = self.steps.mount.state() {
                return Err(ending);
            }
            if *stopped.borrow() {
                return Err(Ending::RunStopped);
            }
            self.set_state(if waiting_for_tracker {
                "paused_by_tracker"
            } else {
                "stacking"
            })?;
            self.refresh_record()?;
            tokio::select! {
                biased;
                _ = stopped.wait_for(|stop| *stop) => {},
                _ = time::sleep(POLL) => {},
            }
        }
    }

    async fn end(&self, mut ending: Ending) {
        if self.stack_started {
            self.update(|run| {
                run.view.message = format!("Saving the stack: {} stacked.", run.view.subs.stacked);
            });
        }
        let (target, settings) = {
            let inner = self.inner.lock().unwrap();
            let view = &inner.as_ref().unwrap().view;
            (view.target.clone(), view.settings.clone())
        };
        if self.abandoned_check {
            self.steps.focus.close_aid().await;
        }
        let saved = if self.stack_started {
            let saved = match self.steps.stack.stop(settings.keep_subs).await {
                Ok(()) => true,
                Err(failure) => {
                    self.warn(Warning::StackUnclean(failure.words().into()));
                    false
                }
            };
            // Include subs saved after the last poll, without judging any rule.
            self.read_stack_counts();
            let stacked = {
                let mut inner = self.inner.lock().unwrap();
                let view = &mut inner.as_mut().unwrap().view;
                view.message = format!("Saving the stack: {} stacked.", view.subs.stacked);
                view.subs.stacked
            };
            if !settings.keep_subs && stacked == 0 {
                self.warn(Warning::SubsKept);
            }
            saved
        } else {
            false
        };
        self.steps.camera.release();
        let track_on = self.steps.mount.track_on();
        if ending == Ending::RunStopped && !track_on {
            ending = Ending::Stopped;
        }
        let at = self.clock.now();
        let finished = Instant::now();
        let message = ending.message(&target.name, &settings);
        let done = {
            let mut inner = self.inner.lock().unwrap();
            let run = inner.as_mut().unwrap();
            run.view.elapsed_s = finished.duration_since(run.started).as_secs_f64();
            run.view.ended = Some(EndedNote {
                reason: ending.reason(),
                at,
                message: message.clone(),
                saved,
                track_on,
            });
            run.view.state = "ended";
            run.view.message = message;
            run.events.push(Event { at, state: "ended" });
            let made = if let Some(store) = &mut run.store {
                let result = store.make(&run.view.id);
                run.view.folder = store.folder().map(str::to_owned);
                result
            } else {
                Ok(())
            };
            if let Err(error) = made.and_then(|()| run.write_record()) {
                let warning =
                    Warning::RecordUnsaved(error.to_string()).words(&run.view.target.name);
                if !run.view.warnings.contains(&warning) {
                    run.view.warnings.push(warning);
                }
            }
            // Completion is published only after the final record attempt.
            run.finished = Some(finished);
            run.done.clone()
        };
        done.send_replace(true);
    }
}

/// Imaging frames in a row the camera may fail before the run ends.
pub const FRAME_TRIES: u32 = 3;
/// Solves in a row that may find no solution before the run ends.
pub const SOLVE_TRIES: u32 = 3;
/// How long the run waits after a failed take before it asks again.
pub const RETRY_WAIT: Duration = Duration::from_secs(5);

/// How often, at most, a rising stacked count or a tracker transition is recorded.
pub const RECORD_EVERY: Duration = Duration::from_secs(60);
