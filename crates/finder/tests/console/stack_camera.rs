//! US11: camera failures, an abort and the stack's camera lease. Oracles:
//! scripted exposure outcomes, the stand-in's sample rule, the lease's named
//! refusal and folder rule, and injected paused time. Counts and paths are exact;
//! background matching is checked over every sample within 0.001 counts.

use std::{
    fs,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use axum::{http::StatusCode, Router};
use finder::imaging::stack::service::StartRequest;
use scope_node::{
    clock::NodeClock,
    imaging::{
        synthetic::SyntheticImagingCamera, Exposure, ImagingCamera, ImagingError, ImagingFrame,
        ImagingFuture, ImagingInfo, ImagingSettings, ImagingStatus,
    },
};
use serde_json::Value;
use simulator::VirtualClock;

use crate::{
    stack_loop::{cards, stand_in_rig, wait_stack, Rig, ScriptedSolver},
    support::live as support,
};

/// The scripted camera retains the stand-in's timing, status and abort behaviour.
/// Exposure numbers count from one, including failed or aborted exposures.
pub(crate) struct ScriptedCamera {
    camera: SyntheticImagingCamera,
    failures: Vec<(usize, String)>,
    fail_from: Option<(usize, String)>,
    additions: Vec<(usize, u16)>,
    calls: AtomicUsize,
    /// The gain the nth `expose` answers its imaging frame with, n from one.
    pub(crate) gain_script: Mutex<Vec<(usize, f64)>>,
}

impl ScriptedCamera {
    pub(crate) fn new(clock: Arc<dyn NodeClock>) -> Self {
        Self {
            camera: SyntheticImagingCamera::new(clock),
            failures: Vec::new(),
            fail_from: None,
            additions: Vec::new(),
            calls: AtomicUsize::new(0),
            gain_script: Mutex::new(Vec::new()),
        }
    }

    pub(crate) fn fail_at(mut self, n: usize, text: &str) -> Self {
        self.failures.push((n, text.to_string()));
        self
    }

    pub(crate) fn fail_from(mut self, n: usize, text: &str) -> Self {
        self.fail_from = Some((n, text.to_string()));
        self
    }

    /// Add counts to every sample of the nth exposure, before it is relayed.
    pub(crate) fn add_at(mut self, n: usize, counts: u16) -> Self {
        self.additions.push((n, counts));
        self
    }
}

impl ImagingCamera for ScriptedCamera {
    fn describe(&self) -> ImagingFuture<'_, ImagingInfo> {
        self.camera.describe()
    }

    fn status(&self) -> ImagingFuture<'_, ImagingStatus> {
        self.camera.status()
    }

    fn apply<'a>(&'a self, settings: &'a ImagingSettings) -> ImagingFuture<'a, ImagingSettings> {
        self.camera.apply(settings)
    }

    fn expose<'a>(&'a self, exposure: &'a Exposure) -> ImagingFuture<'a, ImagingFrame> {
        Box::pin(async move {
            let n = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
            // Even a scripted failure takes the exposure's time and can be aborted.
            let mut frame = self.camera.expose(exposure).await?;
            if let Some((_, text)) = self
                .failures
                .iter()
                .find(|(at, _)| *at == n)
                .or_else(|| self.fail_from.as_ref().filter(|(at, _)| n >= *at))
            {
                return Err(ImagingError::Failed(text.clone()));
            }
            if let Some((_, gain)) = self
                .gain_script
                .lock()
                .unwrap()
                .iter()
                .find(|(at, _)| *at == n)
            {
                frame.gain = Some(*gain);
            }
            if let Some((_, counts)) = self.additions.iter().find(|(at, _)| *at == n) {
                for sample in frame.samples.as_chunks_mut::<2>().0 {
                    let value = u16::from_le_bytes(*sample);
                    sample.copy_from_slice(&value.checked_add(*counts).unwrap().to_le_bytes());
                }
            }
            Ok(frame)
        })
    }

    fn abort(&self) -> ImagingFuture<'_, ()> {
        self.camera.abort()
    }
}

struct NodeTime(VirtualClock);
impl NodeClock for NodeTime {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
}

fn camera() -> ScriptedCamera {
    ScriptedCamera::new(Arc::new(NodeTime(VirtualClock::new(support::START))))
}

async fn scripted_rig(name: &str, camera: ScriptedCamera) -> Rig {
    stand_in_rig(name, ScriptedSolver::new(), Some(Box::new(camera))).await
}

// Copied from stack_loop: private helpers stay private to their own module.
async fn wait_get(app: &Router, done: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..3000 {
        let (status, view) = support::request(app, "GET", "/api/imaging", Value::Null).await;
        assert_eq!(status, StatusCode::OK);
        if done(&view) {
            return view;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("imaging did not reach the awaited state");
}

fn balanced(view: &Value) {
    let dropped: u64 = view["dropped"]
        .as_object()
        .unwrap()
        .values()
        .map(|v| v.as_u64().unwrap())
        .sum();
    assert_eq!(
        view["taken"].as_u64().unwrap(),
        view["stacked"].as_u64().unwrap() + dropped,
        "{view}"
    );
}

/// US11-S1: a single failed exposure is counted and the stack keeps taking subs.
#[tokio::test(start_paused = true)]
async fn one_failed_exposure_does_not_end_the_stack() {
    let rig = scripted_rig(
        "stack-camera-one-failure",
        camera().fail_at(2, "the camera fell over"),
    )
    .await;
    let stack = rig.state.stack().unwrap();
    stack.start(StartRequest::default(), None).await.unwrap();
    let view = wait_stack(&rig, |v| v["taken"].as_u64().unwrap_or(0) >= 4).await;
    assert_eq!(view["dropped"]["frame_failed"], 1);
    assert_eq!(view["state"], "stacking");
    let end = serde_json::to_value(stack.stop().await.unwrap()).unwrap();
    balanced(&end);
}

/// US11-S1: exactly three failures in a row end the stack with the camera's
/// words. The camera is free before the ended view is published.
#[tokio::test(start_paused = true)]
async fn three_consecutive_failures_end_the_stack_and_release_the_camera() {
    let rig = scripted_rig(
        "stack-camera-three-failures",
        camera().fail_from(2, "the camera fell over"),
    )
    .await;
    let stack = rig.state.stack().unwrap();
    stack.start(StartRequest::default(), None).await.unwrap();
    let end = wait_stack(&rig, |v| v["state"] == "ended").await;
    assert_eq!(end["ended_reason"], "camera");
    assert_eq!(end["error"], "the exposure failed: the camera fell over");
    assert_eq!(end["taken"], 4);
    assert_eq!(end["dropped"]["frame_failed"], 3);
    assert_eq!(
        end["stacked"].as_u64().unwrap() + end["dropped"]["unprocessed"].as_u64().unwrap(),
        1
    );
    balanced(&end);
    assert_eq!(rig.state.imaging().unwrap().holder(), None);
    let (status, answer) =
        support::request(&rig.app, "POST", "/api/imaging/frame", Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    // This camera still fails every exposure, but another taker can ask for one.
    rig.state
        .imaging()
        .unwrap()
        .ended(answer["frame"]["id"].as_u64().unwrap())
        .await;
    assert_eq!(serde_json::to_value(stack.view()).unwrap(), end);
}

/// US11-S1: a saved sub resets the consecutive-failure count; three failures
/// separated by successful exposures must not end a stack.
#[tokio::test(start_paused = true)]
async fn successful_subs_separate_three_failures() {
    let rig = scripted_rig(
        "stack-camera-separated-failures",
        camera()
            .fail_at(2, "the camera fell over")
            .fail_at(4, "the camera fell over")
            .fail_at(6, "the camera fell over"),
    )
    .await;
    let stack = rig.state.stack().unwrap();
    stack.start(StartRequest::default(), None).await.unwrap();
    let view = wait_stack(&rig, |v| v["taken"].as_u64().unwrap_or(0) >= 8).await;
    assert_eq!(view["state"], "stacking");
    assert_eq!(view["dropped"]["frame_failed"], 3);
    let end = serde_json::to_value(stack.stop().await.unwrap()).unwrap();
    balanced(&end);
}

/// US11-S1: the second sub's uniform +100 counts are removed by background
/// matching. The oracle is the first sub, with no change at any sample.
#[tokio::test(start_paused = true)]
async fn a_brighter_sub_matches_the_reference_background() {
    let rig = scripted_rig("stack-camera-background", camera().add_at(2, 100)).await;
    let stack = rig.state.stack().unwrap();
    stack.start(StartRequest::default(), None).await.unwrap();
    let view = wait_stack(&rig, |v| v["stacked"].as_u64().unwrap_or(0) >= 1).await;
    assert_eq!(view["stacked"], 1);
    let first = stack.mean().unwrap();
    let view = wait_stack(&rig, |v| v["stacked"].as_u64().unwrap_or(0) >= 2).await;
    assert_eq!(view["stacked"], 2);
    let second = stack.mean().unwrap();
    assert_eq!((first.width, first.height), (64, 48));
    assert_eq!((second.width, second.height), (64, 48));
    assert_eq!(first.planes.len(), 3 * 64 * 48);
    assert_eq!(second.planes.len(), first.planes.len());
    for (index, (before, after)) in first.planes.iter().zip(&second.planes).enumerate() {
        assert!(
            (before - after).abs() <= 0.001,
            "sample {index}: {before} != {after}"
        );
    }
    // Independently read the integer FITS data: matching equal means is useful
    // only if the second sub really carries the scripted increase. FITS stores
    // these samples as signed big-endian integers with BZERO = 32768.
    let subs = rig
        .root
        .join("runs")
        .join(view["id"].as_str().unwrap())
        .join("subs");
    for (id, increase) in [(1, 0), (2, 100)] {
        let bytes = fs::read(subs.join(format!("{id:06}.fits"))).unwrap();
        let header = cards(&bytes);
        for (key, value) in [
            ("BITPIX", "16"),
            ("NAXIS1", "64"),
            ("NAXIS2", "48"),
            ("BZERO", "32768"),
            ("BSCALE", "1"),
        ] {
            assert!(header.contains(&(key.into(), value.into())));
        }
        let end = bytes
            .as_chunks::<80>()
            .0
            .iter()
            .position(|card| &card[..8] == b"END     ")
            .unwrap();
        let data = ((end + 1) * 80).div_ceil(2880) * 2880;
        for y in 0..48 {
            for x in 0..64 {
                let at = data + 2 * (y * 64 + x);
                let sample = i32::from(i16::from_be_bytes([bytes[at], bytes[at + 1]])) + 32768;
                assert_eq!(sample, (416 + x + 3 * y + increase) as i32);
            }
        }
    }
    stack.stop().await;
}

/// US11-S1: a lease's folder belongs to its holder; a save cannot recreate a
/// removed subs folder. A write failure counts as a failed exposure too.
#[tokio::test(start_paused = true)]
async fn removing_the_subs_folder_counts_a_failed_frame() {
    let rig = scripted_rig("stack-camera-missing-subs", camera()).await;
    let stack = rig.state.stack().unwrap();
    stack.start(StartRequest::default(), None).await.unwrap();
    let view = wait_stack(&rig, |v| v["stacked"] == 1).await;
    let subs = rig
        .root
        .join("runs")
        .join(view["id"].as_str().unwrap())
        .join("subs");
    fs::remove_dir_all(&subs).unwrap();
    let view = wait_stack(&rig, |v| {
        v["dropped"]["frame_failed"].as_u64().unwrap_or(0) >= 1
    })
    .await;
    assert_eq!(view["dropped"]["frame_failed"], 1);
    assert_eq!(view["state"], "stacking");
    assert!(!subs.exists());
    let end = serde_json::to_value(stack.stop().await.unwrap()).unwrap();
    balanced(&end);
}

/// US11-S2: the page's Abort remains open to everyone, including while the
/// stack holds the camera. It ends the stack once, within five virtual seconds.
#[tokio::test(start_paused = true)]
async fn aborting_a_sub_ends_the_stack_as_stopped() {
    let rig = stand_in_rig("stack-camera-abort", ScriptedSolver::new(), None).await;
    let stack = rig.state.stack().unwrap();
    stack.start(StartRequest::default(), None).await.unwrap();
    wait_stack(&rig, |v| v["stacked"].as_u64().unwrap_or(0) >= 2).await;
    wait_get(&rig.app, |v| v["frame"]["state"] == "exposing").await;
    let before = tokio::time::Instant::now();
    let (status, answer) =
        support::request(&rig.app, "POST", "/api/imaging/abort", Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let end = tokio::time::timeout(
        Duration::from_secs(5),
        wait_stack(&rig, |v| v["state"] == "ended"),
    )
    .await
    .expect("Abort must end the stack within five seconds");
    assert!(before.elapsed() <= Duration::from_secs(5));
    assert_eq!(end["ended_reason"], "stopped");
    balanced(&end);
    assert_eq!(rig.state.imaging().unwrap().holder(), None);
    tokio::time::sleep(Duration::from_secs(5)).await;
    assert_eq!(serde_json::to_value(stack.stop().await).unwrap(), end);
}

/// US11-S3: another taker gets the holder's name, and does not disturb the
/// running stack. After stop, unleased and newly leased frames use frames/.
#[tokio::test(start_paused = true)]
async fn another_taker_is_refused_until_stop_then_saves_in_frames() {
    let rig = stand_in_rig("stack-camera-other-taker", ScriptedSolver::new(), None).await;
    let stack = rig.state.stack().unwrap();
    let imaging = rig.state.imaging().unwrap();
    stack.start(StartRequest::default(), None).await.unwrap();
    wait_stack(&rig, |v| v["stacked"].as_u64().unwrap_or(0) >= 1).await;
    wait_get(&rig.app, |v| v["frame"]["state"] == "exposing").await;
    let taken = stack.view().unwrap().taken;
    assert_eq!(imaging.holder(), Some("the stack"));
    let (status, answer) =
        support::request(&rig.app, "POST", "/api/imaging/frame", Value::Null).await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert!(
        answer["error"].as_str().unwrap().contains("the stack"),
        "{answer}"
    );
    tokio::time::sleep(Duration::from_secs(5)).await;
    let view = stack.view().unwrap();
    assert!(view.taken > taken);
    assert_eq!(view.state, "stacking");
    assert_eq!(view.dropped.frame_failed, 0);
    let end = stack.stop().await.unwrap();
    assert_eq!(end.state, "ended");
    assert_eq!(imaging.holder(), None);

    let (status, answer) =
        support::request(&rig.app, "POST", "/api/imaging/frame", Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let frame = imaging.ended(answer["frame"]["id"].as_u64().unwrap()).await;
    assert_eq!(frame.state, "saved");
    assert_eq!(frame.purpose, "frame");
    let file = frame.file.unwrap();
    assert!(file.starts_with("frames/2026-01-16/"), "{file}");
    assert!(rig.root.join(file).is_file());

    let lease = imaging.lease("the test").unwrap();
    let id = lease.take("frame").await.unwrap().frame.unwrap().id;
    let frame = imaging.ended(id).await;
    assert_eq!(frame.state, "saved");
    assert_eq!(frame.purpose, "frame");
    let file = frame.file.unwrap();
    assert!(file.starts_with("frames/"), "{file}");
    assert!(rig.root.join(file).is_file());
    drop(lease);
    assert_eq!(imaging.holder(), None);
    assert_eq!(stack.view(), Some(end));
}
