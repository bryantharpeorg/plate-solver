//! The console's own parts of an imaging run: Track, 031's imaging frames and
//! lease, and 032's imaging solve. Sky terms only: nothing here knows an axis.

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use tokio::time::{self, Instant};

use super::{
    By, Centre, Ending, Failure, FrameRef, FrameSteps, MountState, MountSteps, RunTarget,
    SolveSteps, StepFuture, POLL,
};
use crate::{
    api,
    imaging::{ImagingView, Lease},
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
                return Err("the imaging camera has a region of interest set: clear it before an imaging run".into());
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
