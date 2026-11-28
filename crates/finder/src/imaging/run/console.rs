//! The console's own parts of an imaging run: Track, 031's imaging frames and
//! lease, 032's imaging solve, 033's focus check and aid, and 035's stack. Sky terms only:
//! nothing here knows an axis.

use std::{
    fs,
    io::{self, Write},
    path::{Component, Path},
    sync::{Arc, Mutex},
    time::Duration,
};

use tokio::time::{self, Instant};

use super::{
    By, Centre, Ending, Failure, FocusCheck, FocusSteps, FrameRef, FrameSteps, MountState,
    MountSteps, RunTarget, SolveSteps, StackProgress, StackSteps, StepFuture, Unit, Verdict, POLL,
};
use crate::{
    api,
    imaging::{focus::run::FocusAsk, stack::service::StartRequest, ImagingView, Lease},
    model::separation,
    AppState,
};

/// Share of the imaging frame's shorter side used as the centring tolerance.
pub const CENTRE_SHARE: f64 = 0.05;
/// The least centring tolerance, in arcseconds (owner decision 2).
pub const CENTRE_FLOOR_ARCSEC: f64 = 90.0;
/// Polls a re-centre waits, at most, after its own Track off, for the finder
/// camera's next live pointing: 5 s at `POLL`.
pub const FRESH_POLLS: u32 = 20;
/// How long 038's push may read `moving` before it ends a run.
pub const PUSH_GRACE: Duration = Duration::from_secs(10);
/// Share of the shorter side within which an uncentred target may still be stacked.
pub const CENTRE_NEAR_SHARE: f64 = 0.25;
/// The name a run holds the imaging camera's lease under.
pub const HOLDER: &str = "the imaging run";

/// Bytes per sub pixel in the saved stack: twelve in FITS and three in PNG.
pub const RESERVE_PER_PIXEL: u64 = 15;
/// Room for headers, framing, the stack manifest and the run's record.
pub const RESERVE_MARGIN: u64 = 16 * 1024 * 1024;
/// Bytes written between yields while reserving room for the stack's save.
pub const RESERVE_CHUNK: usize = 4 * 1024 * 1024;

/// Reserve for a whole sensor's binned pixels, rounding each side up.
/// A binning of zero counts as one.
pub fn reserve_bytes(width: u32, height: u32, binning: u32) -> u64 {
    let binning = binning.max(1);
    RESERVE_PER_PIXEL * u64::from(width.div_ceil(binning)) * u64::from(height.div_ceil(binning))
        + RESERVE_MARGIN
}

/// Point through the same Track function as the console's route.
pub fn mount(state: &AppState) -> Arc<dyn MountSteps> {
    Arc::new(ConsoleMount {
        state: state.clone(),
        memory: Mutex::new(Motion {
            target: None,
            generation: None,
            by: By::Page,
            repointing: false,
            moving_since: None,
        }),
    })
}

/// Take imaging frames through 031's one lease of the camera.
pub fn camera(state: &AppState) -> Arc<dyn FrameSteps> {
    Arc::new(ConsoleCamera {
        state: state.clone(),
        lease: Mutex::new(None),
    })
}

/// Solve imaging frames with the console's imaging solver.
pub fn solver(state: &AppState) -> Arc<dyn SolveSteps> {
    Arc::new(ConsoleSolver {
        state: state.clone(),
    })
}

/// Check focus and open or close the aid through 033's own service.
pub fn focus(state: &AppState) -> Arc<dyn FocusSteps> {
    Arc::new(ConsoleFocus {
        state: state.clone(),
    })
}

/// Stack through 035, with 036/040's steering and room reserved for the save.
pub fn stack(state: &AppState) -> Arc<dyn StackSteps> {
    Arc::new(ConsoleStack {
        state: state.clone(),
        memory: Mutex::new(StackMemory::default()),
    })
}

struct Motion {
    target: Option<String>,
    generation: Option<u64>,
    by: By,
    repointing: bool,
    moving_since: Option<Instant>,
}

struct ConsoleMount {
    state: AppState,
    memory: Mutex<Motion>,
}

impl ConsoleMount {
    fn stopped(&self) -> bool {
        let generation = self.memory.lock().unwrap().generation;
        self.state
            .goto()
            .is_some_and(|goto| generation.is_some_and(|kept| *goto.stops().borrow() != kept))
    }
}

impl MountSteps for ConsoleMount {
    fn target(&self) -> Result<RunTarget, String> {
        let target = self
            .state
            .session()
            .snapshot(self.state.clock.now())
            .target
            .ok_or("no target is selected")?;
        self.memory.lock().unwrap().target = Some(target.id.clone());
        Ok(RunTarget {
            name: target.messier.clone().unwrap_or_else(|| target.id.clone()),
            id: target.id.clone(),
            ra_deg: target.ra_deg,
            dec_deg: target.dec_deg,
        })
    }

    fn point(&self, by: By) -> StepFuture<'_, Result<(), String>> {
        Box::pin(async move {
            let goto = self.state.goto().ok_or("no mount is connected")?;
            let target = self
                .state
                .session()
                .snapshot(self.state.clock.now())
                .target
                .ok_or("no target is selected")?;
            if self
                .memory
                .lock()
                .unwrap()
                .target
                .as_ref()
                .is_some_and(|kept| *kept != target.id)
            {
                return Err("the target was changed".into());
            }
            let generation = *goto.stops().borrow();
            // The start route already holds the motion lock; it is not re-entrant.
            api::start_tracking(&self.state)
                .await
                .map_err(|error| error.message)?;
            let mut memory = self.memory.lock().unwrap();
            memory.generation = Some(generation);
            memory.by = by;
            // Keep the id read before Track was asked, even if the session changed.
            memory.target = Some(target.id.clone());
            memory.moving_since = None;
            Ok(())
        })
    }

    fn state(&self) -> MountState {
        if self.stopped() {
            return MountState::Ended(Ending::Stopped);
        }
        let now = self.state.clock.now();
        let view = self.state.session().snapshot(now);
        let mut memory = self.memory.lock().unwrap();
        if memory.target.as_ref().is_some_and(|kept| {
            view.target.as_ref().is_none_or(|target| target.id != *kept)
                || view
                    .track
                    .as_ref()
                    .is_some_and(|track| track.target.id != *kept)
        }) {
            return MountState::Ended(Ending::TargetChanged);
        }
        let Some(track) = view.track else {
            memory.moving_since = None;
            return MountState::Ended(Ending::MountFailed("tracking is off".into()));
        };
        // These are the public words of 038's push view, not a second push detector.
        let push = track
            .push
            .as_ref()
            .map(|push| serde_json::to_value(push).expect("push view"));
        let push = push.as_ref().and_then(|push| push["state"].as_str());
        if push.is_none() {
            memory.moving_since = None;
        }
        if !memory.repointing {
            match push {
                Some("following") => return MountState::Ended(Ending::PushedAway),
                Some("moving") => {
                    let since = memory.moving_since.get_or_insert_with(Instant::now);
                    return if since.elapsed() >= PUSH_GRACE {
                        MountState::Ended(Ending::PushedAway)
                    } else {
                        MountState::Tracking { steady: false }
                    };
                }
                _ => {}
            }
        }
        match track.state {
            "armed" => MountState::Slewing,
            "off" => {
                let reason = track.ended_reason.as_deref();
                let ending = match reason {
                    Some("stopped") if memory.repointing => return MountState::Slewing,
                    Some("stopped" | "the observer stopped it") => Ending::Stopped,
                    Some("link_lost" | "halted by a lost link") => Ending::LinkLost,
                    Some("sun_up" | "halted by the Sun guard") => Ending::SunUp,
                    Some("too_low") => Ending::TooLow,
                    _ => Ending::MountFailed(reason.unwrap_or("tracking ended").into()),
                };
                MountState::Ended(ending)
            }
            _ => {
                drop(memory);
                let reach = self
                    .state
                    .imaging()
                    .and_then(|imaging| imaging.view().settings)
                    .map_or(crate::imaging::relation::FIT_REACH_S, |settings| {
                        crate::imaging::relation::FIT_REACH_S.max(settings.exposure_ms / 1000.0)
                    });
                MountState::Tracking {
                    steady: self.state.session().steered().is_some_and(|tracker| {
                        tracker.steady() && tracker.steady_between(now - reach, now)
                    }),
                }
            }
        }
    }

    fn repoint(&self) -> StepFuture<'_, Result<(), Failure>> {
        Box::pin(async move {
            if self.stopped() {
                return Err(Failure::Stopped);
            }
            self.memory.lock().unwrap().repointing = true;
            self.state.session().stop_tracking().await;
            // A pointing recorded with Track on still uses the old aim point.
            // The first live pointing with Track off applies the held measured
            // aim before Track decides whether another GoTo is needed.
            let seq = self
                .state
                .session()
                .snapshot(self.state.clock.now())
                .pointing
                .map(|pointing| pointing.seq);
            for _ in 0..FRESH_POLLS {
                time::sleep(POLL).await;
                if self.stopped() {
                    self.memory.lock().unwrap().repointing = false;
                    return Err(Failure::Stopped);
                }
                let fresh = self
                    .state
                    .session()
                    .snapshot(self.state.clock.now())
                    .pointing
                    .map(|pointing| pointing.seq);
                if fresh != seq {
                    break;
                }
            }
            let target = self.state.session().snapshot(self.state.clock.now()).target;
            let (changed, by) = {
                let memory = self.memory.lock().unwrap();
                (
                    target
                        .as_ref()
                        .is_none_or(|target| Some(&target.id) != memory.target.as_ref()),
                    memory.by,
                )
            };
            if changed {
                self.memory.lock().unwrap().repointing = false;
                return Ok(());
            }
            let tool = match by {
                By::Page => "imaging_run",
                By::Mcp => "start_imaging_run",
            };
            let asked = api::start_motion(&self.state, by.as_str(), tool, || {
                api::start_tracking(&self.state)
            })
            .await;
            self.memory.lock().unwrap().repointing = false;
            // Stop wins even when Track itself refused because that Stop raced its start.
            if self.stopped() {
                api::stop(&self.state).await;
                return Err(Failure::Stopped);
            }
            asked.map_err(|error| Failure::Mount(error.message))
        })
    }

    fn stop(&self) -> StepFuture<'_, ()> {
        Box::pin(async move {
            api::stop(&self.state).await;
        })
    }

    fn track_on(&self) -> bool {
        self.state
            .session()
            .snapshot(self.state.clock.now())
            .track
            .is_some_and(|track| track.state != "off")
    }

    fn steady_between(&self, start: f64, end: f64) -> bool {
        self.state
            .session()
            .steered()
            .is_some_and(|tracker| tracker.steady_between(start, end))
    }

    fn centre(&self) -> StepFuture<'_, Result<bool, Failure>> {
        Box::pin(async move {
            if self.stopped() {
                return Err(Failure::Stopped);
            }
            let by = self.memory.lock().unwrap().by;
            Ok(crate::centre::centre(
                &self.state,
                by.as_str(),
                &crate::centre::CentreAsk::default(),
            )
            .await
            .is_ok())
        })
    }
}

struct ConsoleCamera {
    state: AppState,
    lease: Mutex<Option<Arc<Lease>>>,
}

/// 031's words for a camera that is not ready, without checking anything else.
fn camera_refusal(view: &ImagingView) -> Option<String> {
    match view.state {
        "ready" => None,
        "no_link" => Some("the scope node is not connected".into()),
        "no_service" => Some("this scope node has no imaging service".into()),
        "no_camera" => Some("this scope node has no imaging camera".into()),
        _ => Some(format!(
            "the imaging camera is not answering: {}",
            view.error.as_deref().unwrap_or_default()
        )),
    }
}

impl FrameSteps for ConsoleCamera {
    fn ready(&self) -> Result<(), String> {
        let view = self.state.imaging().expect("console imaging part").view();
        camera_refusal(&view).map_or(Ok(()), Err)
    }

    fn hold(&self) -> Result<(), String> {
        let mut held = self.lease.lock().unwrap();
        if held.is_some() {
            return Ok(());
        }
        let imaging = self.state.imaging().expect("console imaging part");
        let lease = imaging.lease(HOLDER).map_err(|error| error.message)?;
        // The lease's holder must be named before checking for an in-flight frame.
        let view = imaging.view();
        if view
            .frame
            .as_ref()
            .is_some_and(|frame| matches!(frame.state, "exposing" | "receiving"))
        {
            return Err("an exposure is in progress".into());
        }
        if let (Some(settings), Some(camera)) = (view.settings, view.camera) {
            let roi = settings.roi;
            if roi.x != 0 || roi.y != 0 || roi.width != camera.width || roi.height != camera.height
            {
                // A region the focus aid could not take back is named as that (054).
                return Err(match imaging.focus_left() {
                    Some(words) => format!(
                        "the focus aid could not put the imaging camera's settings back: {words}"
                    ),
                    None => "the imaging camera has a region of interest set: clear it before an imaging run".into(),
                });
            }
        }
        *held = Some(Arc::new(lease));
        Ok(())
    }

    fn release(&self) {
        let lease = self.lease.lock().unwrap().take();
        drop(lease);
    }

    fn take(&self) -> StepFuture<'_, Result<FrameRef, Failure>> {
        Box::pin(async move {
            let lease = self.lease.lock().unwrap().clone().ok_or_else(|| {
                Failure::Camera("the imaging run does not hold the imaging camera".into())
            })?;
            let imaging = self.state.imaging().expect("console imaging part");
            let view = lease
                .take("run")
                .await
                .map_err(|error| Failure::Camera(error.message))?;
            let Some(frame) = view.frame.as_ref() else {
                return Err(Failure::Camera(
                    camera_refusal(&view)
                        .unwrap_or_else(|| "the imaging camera answered no frame".into()),
                ));
            };
            let frame = imaging.ended(frame.id).await;
            match frame.state {
                "saved" => Ok(FrameRef {
                    id: frame.id,
                    file: frame.file.expect("saved frame file"),
                }),
                "aborted" => Err(Failure::Camera("the exposure was aborted".into())),
                _ => {
                    let error = frame.error.unwrap_or_default();
                    Err(if error.starts_with("could not save the imaging frame") {
                        Failure::Save(error)
                    } else {
                        Failure::Camera(error)
                    })
                }
            }
        })
    }

    fn abort(&self) -> StepFuture<'_, ()> {
        Box::pin(async move {
            self.state
                .imaging()
                .expect("console imaging part")
                .abort()
                .await;
        })
    }
}

struct ConsoleSolver {
    state: AppState,
}

impl SolveSteps for ConsoleSolver {
    fn ready(&self) -> Result<(), String> {
        if self
            .state
            .imaging()
            .expect("console imaging part")
            .solver()
            .is_some()
        {
            Ok(())
        } else {
            Err("this console has no imaging solver".into())
        }
    }

    fn solve(
        &self,
        frame: &FrameRef,
        target: &RunTarget,
    ) -> StepFuture<'_, Result<Centre, Failure>> {
        let frame = frame.clone();
        let target = target.clone();
        Box::pin(async move {
            let session = self.state.session();
            let before = session.snapshot(self.state.clock.now()).aim;
            let imaging = self.state.imaging().expect("console imaging part");
            let view = imaging
                .solve()
                .await
                .map_err(|error| Failure::NotSolved(error.message))?;
            if view
                .solve
                .as_ref()
                .is_none_or(|solve| solve.frame_id != frame.id)
            {
                return Err(Failure::NotSolved(
                    "another imaging frame was taken before the solve".into(),
                ));
            }
            let solve = imaging.solved(frame.id).await;
            if solve.state != "solved" {
                let error = solve.error.unwrap_or_default();
                return Err(match solve.reason {
                    Some("failed") if error.starts_with("could not save the imaging solve") => {
                        Failure::Save(error)
                    }
                    Some("no_database" | "failed") => Failure::SolverUnavailable(error),
                    _ => Failure::NotSolved(error),
                });
            }
            let scale = solve.arcsec_per_px.expect("solved scale");
            let shorter = imaging
                .view()
                .frame
                .filter(|latest| latest.id == frame.id)
                .and_then(|latest| Some(f64::from(latest.width?.min(latest.height?))));
            let tolerance_arcsec = shorter.map_or(CENTRE_FLOOR_ARCSEC, |shorter| {
                (CENTRE_SHARE * shorter * scale).max(CENTRE_FLOOR_ARCSEC)
            });
            let near_arcsec = shorter.map_or(tolerance_arcsec, |shorter| {
                CENTRE_NEAR_SHARE * shorter * scale
            });
            let after = session.snapshot(self.state.clock.now()).aim;
            let aim_moved = session.held_aim().is_some()
                || after.is_some_and(|after| {
                    before.is_none_or(|before| before.x != after.x || before.y != after.y)
                });
            Ok(Centre {
                offset_arcsec: separation(
                    solve.ra_deg.expect("solved centre"),
                    solve.dec_deg.expect("solved centre"),
                    target.ra_deg,
                    target.dec_deg,
                ) * 3600.0,
                tolerance_arcsec,
                near_arcsec,
                aim_moved,
            })
        })
    }
}

struct ConsoleFocus {
    state: AppState,
}

impl FocusSteps for ConsoleFocus {
    fn check(&self) -> StepFuture<'_, Result<FocusCheck, Failure>> {
        Box::pin(async move {
            let imaging = self.state.imaging().expect("console imaging part");
            imaging
                .focus_start(FocusAsk {
                    check: true,
                    ..Default::default()
                })
                .await
                .map_err(|error| {
                    if error.message == "this console has no imaging solver" {
                        Failure::SolverUnavailable(error.message)
                    } else {
                        Failure::Other(error.message)
                    }
                })?;
            let view = imaging.focus_ended().await;
            let (Some(view), Some(aid)) =
                (view.as_ref(), view.as_ref().and_then(|v| v.aid.as_ref()))
            else {
                return Err(Failure::Other("the focus check left no reading".into()));
            };
            match aid.ended_reason {
                Some("done") => Ok(FocusCheck {
                    verdict: match aid.verdict {
                        Some("good") => Verdict::Good,
                        Some("slightly_soft") => Verdict::SlightlySoft,
                        Some("clearly_out") => Verdict::ClearlyOut,
                        Some("no_record") => Verdict::NoRecord,
                        _ => Verdict::Unmeasurable,
                    },
                    reading: aid
                        .median
                        .map(|median| median.value)
                        .or_else(|| aid.last.and_then(|last| last.value)),
                    record: aid.reference.map(|reference| reference.value),
                    unit: if view.unit == "arcsec" {
                        Unit::Arcsec
                    } else {
                        Unit::Px
                    },
                }),
                Some("stopped") => Err(Failure::Stopped),
                reason => {
                    let mut words =
                        format!("the focus check ended: {}", reason.unwrap_or("failed"));
                    if let Some(error) = &aid.error {
                        words.push_str(": ");
                        words.push_str(error);
                    }
                    Err(if reason == Some("no_region") {
                        Failure::NotSolved(words)
                    } else {
                        Failure::Other(words)
                    })
                }
            }
        })
    }

    fn open_aid(&self) -> StepFuture<'_, Result<(), Failure>> {
        Box::pin(async move {
            self.state
                .imaging()
                .expect("console imaging part")
                .focus_start(FocusAsk::default())
                .await
                .map(|_| ())
                .map_err(|error| Failure::Other(error.message))
        })
    }

    fn close_aid(&self) -> StepFuture<'_, ()> {
        Box::pin(async move {
            self.state
                .imaging()
                .expect("console imaging part")
                .focus_stop()
                .await;
        })
    }
}

#[derive(Default)]
struct StackMemory {
    id: Option<String>,
    last: StackProgress,
}

struct ConsoleStack {
    state: AppState,
    memory: Mutex<StackMemory>,
}

impl StackSteps for ConsoleStack {
    fn start(&self, target: &RunTarget) -> StepFuture<'_, Result<String, Failure>> {
        let label = super::label(&target.name);
        Box::pin(async move {
            *self.memory.lock().unwrap() = StackMemory::default();
            let imaging = self.state.imaging().expect("console imaging part");
            let stacker = self.state.stack().expect("console stack part");
            imaging.steering().set_set_point(None);
            let asked_at = self.state.clock.now();
            let view = stacker
                .start(StartRequest::default(), Some(&label))
                .await
                .map_err(|error| {
                    if error.message.starts_with("cannot make the stack folder") {
                        Failure::Save(error.message)
                    } else {
                        Failure::Other(error.message)
                    }
                })?;
            if view.started_at < asked_at {
                return Err(Failure::Other("another stack is already running".into()));
            }
            self.memory.lock().unwrap().id = Some(view.id.clone());
            let folder = imaging.root().join("runs").join(&view.id);
            let camera = imaging.view();
            let bytes = camera.camera.map_or(RESERVE_MARGIN, |description| {
                reserve_bytes(
                    description.width,
                    description.height,
                    camera.settings.map_or(1, |settings| settings.binning),
                )
            });
            if let Err(error) = write_reserve(&folder.join(".reserve"), bytes).await {
                let _ = fs::remove_file(folder.join(".reserve"));
                if stacker
                    .view()
                    .is_some_and(|own| own.id == view.id && own.state == "stacking")
                {
                    stacker.stop().await;
                }
                *self.memory.lock().unwrap() = StackMemory::default();
                return Err(Failure::Save(format!(
                    "could not reserve room for the stack's save: {error}"
                )));
            }
            Ok(format!("runs/{}", view.id))
        })
    }

    fn progress(&self) -> StackProgress {
        let stacker = self.state.stack().expect("console stack part");
        let Some(view) = stacker.view() else {
            return StackProgress::default();
        };
        let waiting_for_tracker = self
            .state
            .imaging()
            .expect("console imaging part")
            .steering()
            .waiting();
        let mut memory = self.memory.lock().unwrap();
        if memory.id.as_deref() != Some(view.id.as_str()) {
            return StackProgress {
                running: false,
                ..memory.last.clone()
            };
        }
        let dropped = serde_json::to_value(&view.dropped)
            .expect("serializable stack counts")
            .as_object()
            .expect("stack dropped counts object")
            .iter()
            .filter_map(|(reason, count)| {
                count
                    .as_u64()
                    .filter(|count| *count > 0)
                    .map(|count| (reason.clone(), count))
            })
            .collect();
        let failed = if view.state != "ended" {
            None
        } else if view.ended_reason == Some("camera") {
            Some(Failure::Camera(view.error.unwrap_or_default()))
        } else {
            view.save_error.map(Failure::Save)
        };
        let progress = StackProgress {
            running: view.state == "stacking",
            stacked: u64::from(view.stacked),
            dropped,
            waiting_for_tracker,
            failed,
        };
        memory.last = progress.clone();
        progress
    }

    fn stop(&self, keep_subs: bool) -> StepFuture<'_, Result<(), Failure>> {
        Box::pin(async move {
            let id = self.memory.lock().unwrap().id.clone();
            let Some(id) = id else {
                return Ok(());
            };
            let imaging = self.state.imaging().expect("console imaging part");
            let folder = imaging.root().join("runs").join(&id);
            // Free the room before asking 035 to write its final files.
            let _ = fs::remove_file(folder.join(".reserve"));
            let stacker = self.state.stack().expect("console stack part");
            let Some(view) = stacker.view().filter(|view| view.id == id) else {
                return Ok(());
            };
            let stopped_here = view.state == "stacking";
            let view = if stopped_here {
                stacker.stop().await
            } else {
                Some(view)
            };
            let Some(view) = view.filter(|view| view.id == id) else {
                return Ok(());
            };
            if let Some(error) = view.save_error {
                return Err(Failure::Save(error));
            }
            if !keep_subs
                && stopped_here
                && view.stacked > 0
                && view.ended_reason == Some("stopped")
            {
                remove_subs(&folder)?;
            }
            Ok(())
        })
    }
}

async fn write_reserve(path: &Path, mut remaining: u64) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    let zeros = vec![0; RESERVE_CHUNK];
    while remaining > 0 {
        let size = remaining.min(RESERVE_CHUNK as u64) as usize;
        file.write_all(&zeros[..size])?;
        remaining -= size as u64;
        if remaining > 0 {
            tokio::task::yield_now().await;
        }
    }
    Ok(())
}

fn remove_subs(folder: &Path) -> Result<(), Failure> {
    let manifest: serde_json::Value = fs::read(folder.join("stack.json"))
        .and_then(|bytes| serde_json::from_slice(&bytes).map_err(io::Error::other))
        .map_err(|error| Failure::Other(format!("could not read stack.json: {error}")))?;
    let subs = manifest["subs"]
        .as_array()
        .ok_or_else(|| Failure::Other("could not read stack.json: missing subs array".into()))?;
    let mut first_error = None;
    for file in subs.iter().filter_map(|sub| sub["file"].as_str()) {
        let relative = Path::new(file);
        if !relative
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
        {
            first_error.get_or_insert_with(|| {
                Failure::Other(format!(
                    "could not remove {file}: path is outside the stack folder"
                ))
            });
            continue;
        }
        let sub = folder.join(relative);
        for path in [sub.clone(), sub.with_extension("solve.json")] {
            if let Err(error) = fs::remove_file(&path) {
                if error.kind() != io::ErrorKind::NotFound {
                    first_error.get_or_insert_with(|| {
                        Failure::Other(format!("could not remove {}: {error}", path.display()))
                    });
                }
            }
        }
    }
    first_error.map_or(Ok(()), Err)
}
