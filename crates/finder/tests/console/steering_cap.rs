//! US5: the spec's worked sensor geometry (1e-12 radians), exact halvings,
//! scripted limits and call order are the independent oracles for FR-018/019/030.
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use finder::{
    imaging::{
        self,
        solver::{ImagingSolve, Region, SolveFrame},
        steering::{ExposureAsk, GapFuture, Steered, Steering, Stepped, TrackingError},
    },
    link::Transport,
    samples::Samples,
    session::{FixedClock, Session},
    AppState,
};
use scope_link::memory;
use serde_json::{json, Value};
use simulator::{mount::SimMount, SimNode, Simulator, Site, VirtualClock};
use tower::ServiceExt;

use crate::support::{live, Scratch};

const START: f64 = 1_768_528_800.0;
const SMEAR_ERROR: &str = "the corner smear must be a number from 0.1 to 20";
const STEP_ERROR: &str = "the step share must be a number from 0 to 1";
const RATE_ERROR: &str = "the rate share must be a number from 0 to 1";

fn ask(wanted_s: f64) -> ExposureAsk {
    ExposureAsk {
        wanted_s,
        shortest_s: 0.001,
        sensor: (1236, 822),
        roi: Region {
            x: 0,
            y: 0,
            width: 1236,
            height: 822,
        },
        binning: 1,
    }
}
fn worked_frame() -> SolveFrame {
    SolveFrame {
        path: "unused.fits".into(),
        width: 1236,
        height: 822,
        bits: 16,
        bayer: None,
        binning: 1,
        roi: ask(5.0).roi,
        start: START,
        exposure_s: 5.0,
    }
}
fn worked_solve() -> ImagingSolve {
    ImagingSolve::from_cd(
        83.8221,
        -5.3911,
        [
            [-5.196152422706632e-4, -3.0e-4],
            [3.0e-4, -5.196152422706632e-4],
        ],
        1236,
        822,
    )
}
fn near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() <= 1e-12, "{actual} vs {expected}");
}
#[derive(Debug, Clone, PartialEq)]
enum Call {
    Now(f64),
    Ahead(f64),
    Tune(f64, f64),
    Gap,
    Steer(TrackingError),
    Release,
}
struct Scripted {
    limits: Mutex<(Option<f64>, Option<f64>)>,
    log: Mutex<Vec<Call>>,
    subs: AtomicBool,
}
impl Scripted {
    fn new(now: Option<f64>, ahead: Option<f64>) -> Arc<Self> {
        Arc::new(Self {
            limits: Mutex::new((now, ahead)),
            log: Mutex::default(),
            subs: AtomicBool::new(false),
        })
    }
    fn record(&self, call: Call) {
        self.log.lock().unwrap().push(call);
    }
    fn calls(&self) -> Vec<Call> {
        self.log.lock().unwrap().clone()
    }
    fn limits(&self, now: Option<f64>, ahead: Option<f64>) {
        *self.limits.lock().unwrap() = (now, ahead);
    }
    fn now_turn(&self) -> f64 {
        self.calls()
            .into_iter()
            .rev()
            .find_map(|c| if let Call::Now(t) = c { Some(t) } else { None })
            .unwrap()
    }
    fn ahead_turn(&self) -> f64 {
        self.calls()
            .into_iter()
            .rev()
            .find_map(|c| {
                if let Call::Ahead(t) = c {
                    Some(t)
                } else {
                    None
                }
            })
            .unwrap()
    }
}
impl Steered for Scripted {
    fn target(&self) -> Option<(f64, f64)> {
        Some((83.81867, -5.38967))
    }
    fn steer(&self, error: TrackingError) {
        self.record(Call::Steer(error));
        self.subs.store(true, Ordering::SeqCst);
    }
    fn gap(&self) -> GapFuture {
        self.record(Call::Gap);
        Box::pin(async { Stepped::default() })
    }
    fn release(&self) {
        self.record(Call::Release);
        self.subs.store(false, Ordering::SeqCst);
    }
    fn steered_by_subs(&self) -> bool {
        self.subs.load(Ordering::SeqCst)
    }
    fn longest_exposure_s(&self, turn: f64) -> Option<f64> {
        self.record(Call::Now(turn));
        self.limits.lock().unwrap().0
    }
    fn longest_exposure_ahead_s(&self, turn: f64) -> Option<f64> {
        self.record(Call::Ahead(turn));
        self.limits.lock().unwrap().1
    }
    fn tune(&self, step: f64, rate: f64) {
        self.record(Call::Tune(step, rate));
    }
}
type SourceCell = Arc<Mutex<Option<Arc<dyn Steered>>>>;
struct Rig {
    steering: Arc<Steering>,
    tracker: Arc<Scripted>,
    source: SourceCell,
}
impl Rig {
    fn new() -> Self {
        Self::limits(Some(37.15), Some(37.15))
    }
    fn limits(now: Option<f64>, ahead: Option<f64>) -> Self {
        let steering = Steering::new(Arc::new(FixedClock(START)));
        let tracker = Scripted::new(now, ahead);
        let source: SourceCell = Arc::new(Mutex::new(Some(tracker.clone())));
        steering.set_source(Arc::new({
            let source = source.clone();
            move || source.lock().unwrap().clone()
        }));
        Self {
            steering,
            tracker,
            source,
        }
    }
    fn view(&self) -> Value {
        serde_json::to_value(self.steering.view().unwrap()).unwrap()
    }
    fn view_for(&self, ask: ExposureAsk) -> Value {
        serde_json::to_value(self.steering.view_for(Some(ask)).unwrap()).unwrap()
    }
}

#[tokio::test(start_paused = true)]
async fn corner_geometry_and_exact_halving_with_both_floors() {
    // S1: expected radii are the worked examples, independent of production geometry.
    let rig = Rig::new();
    for (wanted, exposure, limited) in [(60.0, 30.0, true), (20.0, 20.0, false)] {
        let go = rig.steering.before_exposure(ask(wanted)).await;
        assert_eq!((go.exposure_s, go.limited), (exposure, limited));
        near(rig.tracker.ahead_turn(), 6.0 / 618f64.hypot(411.0));
        rig.steering.release();
    }
    let mut binned = ask(60.0);
    binned.binning = 2;
    let mut region = ask(60.0);
    region.roi = Region {
        x: 400,
        y: 200,
        width: 400,
        height: 400,
    };
    let mut odd = ask(60.0);
    odd.sensor = (1237, 823);
    odd.roi.width = 1237;
    odd.roi.height = 823;
    for (a, set, turn) in [
        (binned, None, 6.0 / 309f64.hypot(205.5)),
        (region, None, 6.0 / 218f64.hypot(211.0)),
        (ask(60.0), Some((718.0, 361.0)), 6.0 / 718f64.hypot(461.0)),
        (odd, None, 6.0 / 618.5f64.hypot(411.5)),
    ] {
        rig.steering.set_set_point(set);
        rig.steering.before_exposure(a).await;
        near(rig.tracker.ahead_turn(), turn);
        rig.steering.release();
    }
    for (limit, wanted, shortest, exposure, limited) in [
        (Some(6.0), 60.0, 0.001, 3.75, true),
        (Some(8.0), 60.0, 10.0, 15.0, true),
        (None, 60.0, 0.001, 60.0, false),
        (Some(30.0), 60.0, 0.001, 30.0, true),
        (Some(0.7), 5.0, 0.001, 1.25, true),
    ] {
        rig.tracker.limits(limit, limit);
        let go = rig
            .steering
            .before_exposure(ExposureAsk {
                shortest_s: shortest,
                ..ask(wanted)
            })
            .await;
        assert_eq!((go.exposure_s, go.limited), (exposure, limited));
        rig.steering.release();
    }
}

#[tokio::test(start_paused = true)]
async fn held_exposure_and_limited_survive_new_asks_until_release() {
    // S2: strict comparison at the current limit, exact dyadic exposures.
    let rig = Rig::new();
    let first = rig.steering.before_exposure(ask(60.0)).await;
    assert_eq!((first.exposure_s, first.limited), (30.0, true));
    rig.tracker.limits(Some(25.0), Some(25.0));
    for wanted in [60.0, 10.0] {
        assert_eq!(rig.steering.before_exposure(ask(wanted)).await, first);
    }
    let view = rig.view();
    assert_eq!(view["exposure_s"], 30.0);
    assert_eq!(view["limited"], true);
    assert_eq!(view["longest_exposure_s"], 25.0);
    assert_eq!(view["over_limit"], true);
    rig.steering.release();
    let view = rig.view();
    assert!(view.get("exposure_s").is_none());
    assert_eq!(view["limited"], false);
    assert_eq!(view["over_limit"], false);
    let next = rig.steering.before_exposure(ask(60.0)).await;
    assert_eq!((next.exposure_s, next.limited), (15.0, true));
    assert_eq!(rig.view()["over_limit"], false);
    let rig = Rig::new();
    rig.steering.before_exposure(ask(60.0)).await;
    rig.tracker.limits(Some(30.0), Some(30.0));
    rig.steering.before_exposure(ask(60.0)).await;
    assert_eq!(rig.view()["longest_exposure_s"], 30.0);
    assert_eq!(rig.view()["over_limit"], false);
}

#[tokio::test(start_paused = true)]
async fn smear_setting_and_refused_settings_do_not_mark_used() {
    // S3: range and exact refusal words from FR-019, 1e-12 geometry tolerance.
    let rig = Rig::new();
    assert_eq!(rig.steering.set_smear_px(0.05).unwrap_err(), SMEAR_ERROR);
    assert_eq!(rig.steering.set_step_share(1.5).unwrap_err(), STEP_ERROR);
    assert_eq!(rig.steering.set_rate_share(-0.1).unwrap_err(), RATE_ERROR);
    assert!(rig.steering.view().is_none());
    rig.steering.set_smear_px(1.0).unwrap();
    rig.steering.before_exposure(ask(60.0)).await;
    near(rig.tracker.ahead_turn(), 1.0 / 618f64.hypot(411.0));
    for value in [0.05, 20.5, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let before = rig.view();
        assert_eq!(rig.steering.set_smear_px(value).unwrap_err(), SMEAR_ERROR);
        assert_eq!(rig.view(), before);
    }
    for value in [0.1, 20.0] {
        rig.steering.set_smear_px(value).unwrap();
        assert_eq!(rig.view()["smear_px"], value);
    }
}

#[tokio::test(start_paused = true)]
async fn view_shows_defaults_held_exposure_and_only_the_running_trackers_limit() {
    // S4: view reads the live source, not a cached limit.
    let rig = Rig::new();
    rig.steering.before_exposure(ask(60.0)).await;
    let view = rig.view();
    for (key, expected) in [
        ("smear_px", json!(6.0)),
        ("on", json!(true)),
        ("step_share", json!(0.5)),
        ("rate_share", json!(0.3)),
        ("exposure_s", json!(30.0)),
        ("limited", json!(true)),
        ("over_limit", json!(false)),
        ("longest_exposure_s", json!(37.15)),
    ] {
        assert_eq!(view[key], expected, "{key}");
    }
    *rig.source.lock().unwrap() = None;
    assert!(rig.view().get("longest_exposure_s").is_none());
    rig.steering.before_exposure(ask(60.0)).await;
    assert!(rig.view().get("longest_exposure_s").is_none());
    let rig = Rig::limits(None, None);
    rig.steering.before_exposure(ask(60.0)).await;
    assert_eq!(rig.view()["longest_exposure_s"], "no limit");
    assert_eq!(rig.view()["exposure_s"], 60.0);
    assert_eq!(rig.view()["limited"], false);
}

#[tokio::test(start_paused = true)]
async fn prospective_view_does_not_use_steering_and_separates_stack_and_new_asks() {
    // S6: now=37.15, ahead=20; 60 halved twice is 15, while a new 10 fits.
    let rig = Rig::limits(Some(37.15), Some(20.0));
    assert!(rig.steering.view().is_none());
    let view = rig.view_for(ask(60.0));
    assert_eq!(view["source"], "tracker");
    assert_eq!(view["longest_exposure_s"], 37.15);
    assert_eq!(view["would_take_s"], 15.0);
    assert!(view.get("exposure_s").is_none());
    assert_eq!(view["limited"], false);
    assert!(rig.steering.view().is_none());
    rig.tracker.limits(None, None);
    let view = rig.view_for(ask(60.0));
    assert_eq!(view["longest_exposure_s"], "no limit");
    assert_eq!(view["would_take_s"], 60.0);
    *rig.source.lock().unwrap() = None;
    assert!(rig.steering.view_for(Some(ask(60.0))).is_none());
    *rig.source.lock().unwrap() = Some(rig.tracker.clone());
    rig.tracker.limits(Some(37.15), Some(20.0));
    rig.steering.before_exposure(ask(60.0)).await;
    let view = rig.view_for(ask(10.0));
    assert_eq!(view["exposure_s"], 15.0);
    assert_eq!(view["would_take_s"], 10.0);
    let mut binned = ask(10.0);
    binned.binning = 2;
    rig.view_for(binned);
    near(rig.tracker.now_turn(), 6.0 / 618f64.hypot(411.0));
    near(rig.tracker.ahead_turn(), 6.0 / 309f64.hypot(205.5));
}

#[tokio::test(start_paused = true)]
async fn switch_off_keeps_the_cap_without_gap_tune_or_steer() {
    // S7: the switch controls feedback only; exact call-log negative controls.
    let rig = Rig::new();
    rig.steering.set_on(false);
    assert_eq!(rig.view()["on"], false);
    let go = rig.steering.before_exposure(ask(60.0)).await;
    assert_eq!((go.exposure_s, go.limited), (30.0, true));
    rig.steering.accepted(&worked_frame(), &worked_solve());
    assert!(!rig
        .tracker
        .calls()
        .iter()
        .any(|c| matches!(c, Call::Gap | Call::Tune(..) | Call::Steer(_))));
    assert_eq!(rig.view()["source"], "tracker");
    assert_eq!(rig.view()["on"], false);
}

#[tokio::test(start_paused = true)]
async fn shares_are_checked_and_tuned_immediately_before_each_gap() {
    // S8: FR-030's defaults, inclusive bounds and call order are exact.
    let rig = Rig::new();
    rig.steering.before_exposure(ask(5.0)).await;
    rig.steering.set_step_share(0.25).unwrap();
    rig.steering.set_rate_share(0.0).unwrap();
    rig.steering.before_exposure(ask(5.0)).await;
    let calls = rig.tracker.calls();
    let gaps: Vec<_> = calls
        .iter()
        .enumerate()
        .filter_map(|(i, c)| (*c == Call::Gap).then_some(i))
        .collect();
    assert_eq!(gaps.len(), 2);
    assert_eq!(calls[gaps[0] - 1], Call::Tune(0.5, 0.3));
    assert_eq!(calls[gaps[1] - 1], Call::Tune(0.25, 0.0));
    assert_eq!(rig.view()["step_share"], 0.25);
    assert_eq!(rig.view()["rate_share"], 0.0);
    for value in [-0.1, 1.5, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let before = rig.view();
        assert_eq!(rig.steering.set_step_share(value).unwrap_err(), STEP_ERROR);
        assert_eq!(rig.view(), before);
        assert_eq!(rig.steering.set_rate_share(value).unwrap_err(), RATE_ERROR);
        assert_eq!(rig.view(), before);
    }
    for value in [0.0, 1.0] {
        rig.steering.set_step_share(value).unwrap();
        rig.steering.set_rate_share(value).unwrap();
        assert_eq!(rig.view()["step_share"], value);
        assert_eq!(rig.view()["rate_share"], value);
    }
}

#[tokio::test(start_paused = true)]
async fn switch_off_mid_stack_releases_once_and_keeps_the_exposure() {
    // S9: the worked solve is within 180 arcsec, so one error is handed over.
    let rig = Rig::new();
    rig.steering.before_exposure(ask(5.0)).await;
    rig.steering.accepted(&worked_frame(), &worked_solve());
    assert_eq!(rig.view()["source"], "subs");
    assert_eq!(rig.view()["subs"], 1);
    assert_eq!(
        rig.tracker
            .calls()
            .iter()
            .filter(|c| matches!(c, Call::Gap))
            .count(),
        1
    );
    assert_eq!(
        rig.tracker
            .calls()
            .iter()
            .filter(|c| matches!(c, Call::Steer(_)))
            .count(),
        1
    );
    let before = rig.tracker.calls();
    rig.steering.set_on(false);
    let after = rig.tracker.calls();
    assert_eq!(&after[..before.len()], before);
    assert_eq!(&after[before.len()..], [Call::Release]);
    let view = rig.view();
    assert_eq!(view["source"], "tracker");
    assert_eq!(view["subs"], 0);
    assert!(view.get("last").is_none());
    assert_eq!(view["exposure_s"], 5.0);
    assert_eq!(
        rig.steering.before_exposure(ask(60.0)).await.exposure_s,
        5.0
    );
    assert_eq!(
        rig.tracker
            .calls()
            .iter()
            .filter(|c| matches!(c, Call::Gap))
            .count(),
        1
    );
    rig.steering.set_on(true);
    assert_eq!(
        rig.steering.before_exposure(ask(60.0)).await.exposure_s,
        5.0
    );
    assert_eq!(
        rig.tracker
            .calls()
            .iter()
            .filter(|c| matches!(c, Call::Gap))
            .count(),
        2
    );
}

#[tokio::test(start_paused = true)]
async fn decides_from_the_hour_ahead_but_shows_the_limit_now() {
    // S10: 60 -> 30 -> 15 against 20 ahead, not 37.15 now.
    let rig = Rig::limits(Some(37.15), Some(20.0));
    let go = rig.steering.before_exposure(ask(60.0)).await;
    assert_eq!((go.exposure_s, go.limited), (15.0, true));
    let view = rig.view();
    assert_eq!(view["exposure_s"], 15.0);
    assert_eq!(view["longest_exposure_s"], 37.15);
    assert_eq!(view["over_limit"], false);
}

// 032's solving rig: same database/site/clock, mount connected before the push,
// paired session without a target or aim point, and simulated imaging solver.
struct Console {
    app: Router,
    imaging: Option<Arc<imaging::Imaging>>,
}
fn console(root: Option<&Path>) -> Console {
    let sim = Simulator::new(live::DB.clone(), Site::default(), VirtualClock::new(START));
    sim.connect_mount(SimMount::new(sim.clock().clone()));
    sim.push_to(50.0, 120.0);
    let node = Arc::new(
        SimNode::new(sim.clone(), "test-token", Default::default())
            .unwrap()
            .with_frame_interval(Duration::from_millis(500)),
    );
    let session = Session::default();
    session.edit(serde_json::from_value(json!({"node":{"address":"scope.local:1","token":"test-token"},"site":{"lat_deg":40,"lon_deg":-75}})).unwrap(), START).unwrap();
    let mut state = AppState::shared(live::DB.clone(), Samples::default())
        .with_session(session)
        .with_clock(Arc::new(live::ConsoleClock(sim.clock().clone())))
        .with_link(Transport::connector(move || {
            let node = node.clone();
            async move {
                let (client, server) = tokio::io::duplex(64 * 1024);
                tokio::spawn(async move {
                    memory::serve_routes_on(server, node.service().routes())
                        .await
                        .unwrap();
                });
                Ok::<_, std::io::Error>(client)
            }
        }))
        .with_live();
    if let Some(root) = root {
        state = state
            .with_imaging(root)
            .with_imaging_solver(Arc::new(imaging::simulated::SimulatedSolver::new(sim)));
    }
    let imaging = state.imaging();
    Console {
        app: finder::router(state),
        imaging,
    }
}
async fn ready(app: &Router) -> Value {
    for _ in 0..600 {
        let (_, v) = live::request(app, "GET", "/api/imaging", Value::Null).await;
        if v["state"] == "ready" {
            return v;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("camera never ready");
}
async fn put(app: &Router, body: Value) -> (StatusCode, Value) {
    live::request(app, "PUT", "/api/imaging/steering", body).await
}
async fn steering_view(app: &Router) -> Value {
    live::request(app, "GET", "/api/imaging", Value::Null)
        .await
        .1["steering"]
        .clone()
}
async fn raw_put(
    app: &Router,
    body: &str,
    content_type: &str,
    foreign: bool,
) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method("PUT")
        .uri("/api/imaging/steering")
        .header("content-type", content_type)
        .header("host", "localhost");
    if foreign {
        req = req.header("origin", "https://foreign.example");
    }
    let response = app
        .clone()
        .oneshot(req.body(Body::from(body.to_owned())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test(start_paused = true)]
async fn route_edits_four_settings_and_refuses_invalid_requests_atomically() {
    // S5: exact FR-019 status/words; compare unchanged steering after every refusal.
    let scratch = Scratch::new("steering-route");
    let rig = console(Some(&scratch.0));
    ready(&rig.app).await;
    let (status, view) = put(&rig.app, json!({"smear_px":1.5})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(view["steering"]["smear_px"], 1.5);
    assert_eq!(view["steering"]["on"], true);
    let session = live::request(&rig.app, "GET", "/api/session", Value::Null)
        .await
        .1;
    assert_eq!(session["imaging"]["steering"]["smear_px"], 1.5);
    assert_eq!(session["imaging"]["steering"]["on"], true);
    let (status, view) = put(&rig.app, json!({"on":false})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(view["steering"]["on"], false);
    assert_eq!(view["steering"]["smear_px"], 1.5);
    let (status, view) = put(&rig.app, json!({"step_share":0.25,"rate_share":0})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(view["steering"]["step_share"], 0.25);
    assert_eq!(view["steering"]["rate_share"], 0.0);
    let before = steering_view(&rig.app).await;
    let (status, view) = put(&rig.app, json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(view["steering"], before);
    for (body, error) in [
        (json!({"smear_px":0}), Some(SMEAR_ERROR)),
        (json!({"smear_px":"2"}), None),
        (json!({"on":1}), None),
        (json!({"smear":2}), None),
        (json!({"step_share":1.5}), Some(STEP_ERROR)),
        (json!({"rate_share":-0.1}), Some(RATE_ERROR)),
        (json!({"on":true,"smear_px":0}), Some(SMEAR_ERROR)),
        (json!({"on":null}), None),
        (json!({"smear_px":null}), None),
        (json!({"step_share":null}), None),
        (json!({"rate_share":null}), None),
        (
            json!({"rate_share":-0.1,"step_share":1.5,"smear_px":0}),
            Some(SMEAR_ERROR),
        ),
        (
            json!({"rate_share":-0.1,"step_share":1.5}),
            Some(STEP_ERROR),
        ),
        (json!([]), None),
        (Value::Null, None),
        (json!(true), None),
    ] {
        let (status, reply) = put(&rig.app, body.clone()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}: {reply}");
        assert!(reply["error"].is_string());
        if let Some(error) = error {
            assert_eq!(reply["error"], error);
        }
        assert_eq!(steering_view(&rig.app).await, before);
    }
    for (body, content_type, foreign, expected) in [
        (
            "{\"on\":true}",
            "application/json",
            true,
            StatusCode::FORBIDDEN,
        ),
        (
            "{\"on\":true}",
            "text/plain",
            false,
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
        ),
        (
            "not json",
            "application/json",
            false,
            StatusCode::BAD_REQUEST,
        ),
    ] {
        let (status, reply) = raw_put(&rig.app, body, content_type, foreign).await;
        assert_eq!(status, expected);
        assert!(reply["error"].is_string());
        assert_eq!(steering_view(&rig.app).await, before);
    }
    let rig = console(None);
    let (status, reply) = put(&rig.app, json!({"on":true})).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(reply["error"], "this console takes no imaging frames");
}

#[tokio::test(start_paused = true)]
async fn console_prospective_view_uses_its_camera_sensor_settings_and_shortest_exposure() {
    // S6: FR-018 uses the real camera's minimum and dimensions, not the worked rig.
    let scratch = Scratch::new("steering-prospective");
    let rig = console(Some(&scratch.0));
    ready(&rig.app).await;
    let steering = rig.imaging.as_ref().unwrap().steering();
    let tracker = Scripted::new(None, None);
    steering.set_source(Arc::new({
        let tracker = tracker.clone();
        move || Some(tracker.clone())
    }));
    let view = live::request(&rig.app, "GET", "/api/imaging", Value::Null)
        .await
        .1;
    assert_eq!(view["steering"]["longest_exposure_s"], "no limit");
    assert_eq!(
        view["steering"]["would_take_s"],
        view["settings"]["exposure_ms"].as_f64().unwrap() / 1000.0
    );
    assert!(steering.view().is_none());
    let (status, _) = live::request(
        &rig.app,
        "PUT",
        "/api/imaging/settings",
        json!({"exposure_ms":8000}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    tracker.limits(Some(1000.0), Some(0.3));
    let view = live::request(&rig.app, "GET", "/api/imaging", Value::Null)
        .await
        .1;
    assert_eq!(view["steering"]["would_take_s"], 1.0);
    let w = view["camera"]["width"].as_f64().unwrap();
    let h = view["camera"]["height"].as_f64().unwrap();
    near(tracker.now_turn(), 6.0 / (w / 2.0).hypot(h / 2.0));
    near(tracker.ahead_turn(), 6.0 / (w / 2.0).hypot(h / 2.0));
}
