//! Scripted stand-ins for the five parts of an imaging run, as `FixedClock` stands in
//! for a clock: tests queue the answers and read back the calls.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Arc, Mutex},
};

use tokio::sync::Notify;

use super::{
    By, Centre, Ending, Failure, FocusCheck, FocusSteps, FrameRef, FrameSteps, MountState,
    MountSteps, RunTarget, SolveSteps, StackProgress, StackSteps, StepFuture, Steps, Unit, Verdict,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Part {
    Camera,
    Solver,
    Target,
    Hold,
    Point,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Step {
    Take,
    Check,
    Solve,
    Repoint,
    StackStart,
    StackStop,
    CloseAid,
}

/// The folder the script's stack answers.
pub const STACK_FOLDER: &str = "runs/2026-01-16T02-00-10Z-M42";

struct Inner {
    mount: MountState,
    refusals: HashMap<Part, String>,
    frames: VecDeque<Result<FrameRef, Failure>>,
    focus: VecDeque<Result<FocusCheck, Failure>>,
    solves: VecDeque<Result<Centre, Failure>>,
    next_frame: u64,
    unsteady: u32,
    repoint_failure: Option<Failure>,
    open_aid_failure: Option<Failure>,
    stack_start_failure: Option<Failure>,
    stack_stop_failure: Option<Failure>,
    stack: StackProgress,
    holds: HashSet<Step>,
    calls: Vec<String>,
}

/// Queued answers and calls under one lock, with a notification for held steps.
pub struct Script {
    inner: Mutex<Inner>,
    released: Notify,
}

impl Script {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(Inner {
                mount: MountState::Tracking { steady: true },
                refusals: HashMap::new(),
                frames: VecDeque::new(),
                focus: VecDeque::new(),
                solves: VecDeque::new(),
                next_frame: 1,
                unsteady: 0,
                repoint_failure: None,
                open_aid_failure: None,
                stack_start_failure: None,
                stack_stop_failure: None,
                stack: StackProgress::default(),
                holds: HashSet::new(),
                calls: Vec::new(),
            }),
            released: Notify::new(),
        })
    }

    /// All five parts are this script.
    pub fn steps(self: &Arc<Self>) -> Steps {
        Steps {
            mount: self.clone(),
            camera: self.clone(),
            focus: self.clone(),
            solver: self.clone(),
            stack: self.clone(),
        }
    }

    /// What `MountSteps::state` answers from now on.
    pub fn set_mount(&self, state: MountState) {
        self.inner.lock().unwrap().mount = state;
    }

    /// Make a part refuse with these words on every later call.
    pub fn refuse(&self, part: Part, words: &str) {
        self.inner
            .lock()
            .unwrap()
            .refusals
            .insert(part, words.into());
    }

    /// Queue the next take's answer, using no default frame number.
    pub fn push_frame(&self, answer: Result<FrameRef, Failure>) {
        self.inner.lock().unwrap().frames.push_back(answer);
    }

    /// Queue the next focus check's answer.
    pub fn push_focus(&self, answer: Result<FocusCheck, Failure>) {
        self.inner.lock().unwrap().focus.push_back(answer);
    }

    /// Queue the next solve's answer.
    pub fn push_solve(&self, answer: Result<Centre, Failure>) {
        self.inner.lock().unwrap().solves.push_back(answer);
    }

    /// Make the next n calls of `steady_between` answer false.
    pub fn unsteady(&self, n: u32) {
        self.inner.lock().unwrap().unsteady = n;
    }

    /// Make the next re-point fail once.
    pub fn fail_repoint(&self, failure: Failure) {
        self.inner.lock().unwrap().repoint_failure = Some(failure);
    }

    /// Make the next opening of the focus aid fail once.
    pub fn fail_open_aid(&self, failure: Failure) {
        self.inner.lock().unwrap().open_aid_failure = Some(failure);
    }

    /// Make the next stack start fail once.
    pub fn fail_stack_start(&self, failure: Failure) {
        self.inner.lock().unwrap().stack_start_failure = Some(failure);
    }

    /// Make the next stack stop fail once.
    pub fn fail_stack_stop(&self, failure: Failure) {
        self.inner.lock().unwrap().stack_stop_failure = Some(failure);
    }

    /// Set counts and the tracker's pause, leaving `running` and `failed` alone.
    /// A dropped count of zero is left out.
    pub fn set_stack(&self, stacked: u64, dropped: &[(&str, u64)], waiting_for_tracker: bool) {
        let mut inner = self.inner.lock().unwrap();
        inner.stack.stacked = stacked;
        inner.stack.dropped = dropped
            .iter()
            .filter(|(_, count)| *count > 0)
            .map(|(reason, count)| ((*reason).into(), *count))
            .collect();
        inner.stack.waiting_for_tracker = waiting_for_tracker;
    }

    /// Make the stack end by itself: it no longer runs, and `failed` is as given.
    pub fn end_stack(&self, failed: Option<Failure>) {
        let mut inner = self.inner.lock().unwrap();
        inner.stack.running = false;
        inner.stack.failed = failed;
    }

    /// Make every call of a step wait, after it is listed, until released.
    pub fn hold(&self, step: Step) {
        self.inner.lock().unwrap().holds.insert(step);
    }

    /// Let the calls of a step that wait go on, and end its hold.
    pub fn release(&self, step: Step) {
        self.inner.lock().unwrap().holds.remove(&step);
        self.released.notify_waiters();
    }

    /// The calls the run made, in order.
    pub fn calls(&self) -> Vec<String> {
        self.inner.lock().unwrap().calls.clone()
    }

    fn called(&self, call: impl Into<String>) {
        self.inner.lock().unwrap().calls.push(call.into());
    }

    fn refusal(&self, part: Part, call: impl Into<String>) -> Result<(), String> {
        let mut inner = self.inner.lock().unwrap();
        inner.calls.push(call.into());
        match inner.refusals.get(&part) {
            Some(words) => Err(words.clone()),
            None => Ok(()),
        }
    }

    async fn wait(&self, step: Step) {
        loop {
            // Register before reading the hold so release cannot be missed between
            // that read and awaiting the notification. No lock crosses the await.
            let released = self.released.notified();
            tokio::pin!(released);
            released.as_mut().enable();
            if !self.inner.lock().unwrap().holds.contains(&step) {
                return;
            }
            released.await;
        }
    }
}

impl MountSteps for Script {
    fn target(&self) -> Result<RunTarget, String> {
        self.refusal(Part::Target, "mount.target")?;
        Ok(RunTarget {
            id: "NGC 1976".into(),
            name: "M42".into(),
            ra_deg: 83.82,
            dec_deg: -5.39,
        })
    }

    fn point(&self, by: By) -> StepFuture<'_, Result<(), String>> {
        let answer = self.refusal(Part::Point, format!("mount.point {}", by.as_str()));
        Box::pin(async move { answer })
    }

    fn state(&self) -> MountState {
        self.inner.lock().unwrap().mount.clone()
    }

    fn repoint(&self) -> StepFuture<'_, Result<(), Failure>> {
        self.called("mount.repoint");
        Box::pin(async move {
            self.wait(Step::Repoint).await;
            match self.inner.lock().unwrap().repoint_failure.take() {
                Some(failure) => Err(failure),
                None => Ok(()),
            }
        })
    }

    fn stop(&self) -> StepFuture<'_, ()> {
        self.called("mount.stop");
        Box::pin(async {})
    }

    fn track_on(&self) -> bool {
        !matches!(
            self.inner.lock().unwrap().mount,
            MountState::Ended(
                Ending::Stopped
                    | Ending::TooLow
                    | Ending::SunUp
                    | Ending::LinkLost
                    | Ending::MountFailed(_)
            )
        )
    }

    fn steady_between(&self, _start: f64, _end: f64) -> bool {
        let mut inner = self.inner.lock().unwrap();
        if inner.unsteady == 0 {
            true
        } else {
            inner.unsteady -= 1;
            false
        }
    }
}

impl FrameSteps for Script {
    fn ready(&self) -> Result<(), String> {
        self.refusal(Part::Camera, "camera.ready")
    }

    fn hold(&self) -> Result<(), String> {
        self.refusal(Part::Hold, "camera.hold")
    }

    fn release(&self) {
        self.called("camera.release");
    }

    fn take(&self) -> StepFuture<'_, Result<FrameRef, Failure>> {
        self.called("camera.take");
        Box::pin(async move {
            self.wait(Step::Take).await;
            let mut inner = self.inner.lock().unwrap();
            if let Some(answer) = inner.frames.pop_front() {
                return answer;
            }
            let n = inner.next_frame;
            inner.next_frame += 1;
            Ok(FrameRef {
                id: n,
                file: format!("frames/2026-01-16/{n:06}.fits"),
            })
        })
    }

    fn abort(&self) -> StepFuture<'_, ()> {
        self.called("camera.abort");
        Box::pin(async {})
    }
}

impl FocusSteps for Script {
    fn check(&self) -> StepFuture<'_, Result<FocusCheck, Failure>> {
        self.called("focus.check");
        Box::pin(async move {
            self.wait(Step::Check).await;
            self.inner
                .lock()
                .unwrap()
                .focus
                .pop_front()
                .unwrap_or(Ok(FocusCheck {
                    verdict: Verdict::Good,
                    reading: Some(3.0),
                    record: Some(3.0),
                    unit: Unit::Arcsec,
                }))
        })
    }

    fn open_aid(&self) -> StepFuture<'_, Result<(), Failure>> {
        self.called("focus.open_aid");
        let failure = self.inner.lock().unwrap().open_aid_failure.take();
        Box::pin(async move { failure.map_or(Ok(()), Err) })
    }

    fn close_aid(&self) -> StepFuture<'_, ()> {
        self.called("focus.close_aid");
        Box::pin(async move { self.wait(Step::CloseAid).await })
    }
}

impl SolveSteps for Script {
    fn ready(&self) -> Result<(), String> {
        self.refusal(Part::Solver, "solver.ready")
    }

    fn solve(
        &self,
        frame: &FrameRef,
        _target: &RunTarget,
    ) -> StepFuture<'_, Result<Centre, Failure>> {
        self.called(format!("solver.solve {}", frame.id));
        Box::pin(async move {
            self.wait(Step::Solve).await;
            self.inner
                .lock()
                .unwrap()
                .solves
                .pop_front()
                .unwrap_or(Ok(Centre {
                    offset_arcsec: 0.0,
                    tolerance_arcsec: 90.0,
                    near_arcsec: 450.0,
                    aim_moved: true,
                }))
        })
    }
}

impl StackSteps for Script {
    fn start(&self, target: &RunTarget) -> StepFuture<'_, Result<String, Failure>> {
        self.called(format!("stack.start {}", target.name));
        Box::pin(async move {
            self.wait(Step::StackStart).await;
            let mut inner = self.inner.lock().unwrap();
            match inner.stack_start_failure.take() {
                Some(failure) => Err(failure),
                None => {
                    inner.stack.running = true;
                    Ok(STACK_FOLDER.into())
                }
            }
        })
    }

    fn progress(&self) -> StackProgress {
        self.inner.lock().unwrap().stack.clone()
    }

    fn stop(&self, keep_subs: bool) -> StepFuture<'_, Result<(), Failure>> {
        {
            let mut inner = self.inner.lock().unwrap();
            inner.calls.push(
                if keep_subs {
                    "stack.stop keep"
                } else {
                    "stack.stop discard"
                }
                .into(),
            );
            inner.stack.running = false;
        }
        Box::pin(async move {
            self.wait(Step::StackStop).await;
            match self.inner.lock().unwrap().stack_stop_failure.take() {
                Some(failure) => Err(failure),
                None => Ok(()),
            }
        })
    }
}
