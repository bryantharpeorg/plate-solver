//! The focus reading (033 US1): `focus::read` over the simulated solver
//! adapter and a scripted solver, the adapter's seeded noise and sharpest
//! point, and the simulate route of the simulator's focuser. Oracles: the
//! simulator's star size H(e) = 2·√(2·ln 2)·√(1.2² + e²) worked from 030's
//! point-spread constants, the published SplitMix64 stream, the worked
//! examples 3.10 × 0.548 = 1.6988 and 1.55 × 2 = 3.10, the stated rule of five
//! stars and 20 frame pixels, and, for the route, the scenario's own words,
//! statuses and keys.

use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
};

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use finder::{
    imaging::{
        self,
        focus::{self, Unmeasurable},
        solver::{
            ImagingSolver, MeasureFuture, Region, SolveError, SolveFrame, SolveFuture, SolveHint,
            StarMeasure,
        },
    },
    samples::Samples,
    simulate::SimulationTransport,
    AppState,
};
use serde_json::{json, Value};
use simulator::{Simulator, Site, VirtualClock};
use tower::ServiceExt;

use crate::support::live as support;

/// The focus rig's simulator (the shared setup): the test database at 40° N
/// 75° W on the virtual clock from Unix 1768528800, pointed at the celestial
/// pole, where the imaging field moves by under a pixel in three seconds. No
/// mount is connected.
fn sim() -> Simulator {
    let sim = Simulator::new(
        support::DB.clone(),
        Site::default(),
        VirtualClock::new(support::START),
    );
    sim.push_to(40.0, 0.0);
    sim
}

/// One imaging frame of the test's own description (the shared setup): no file
/// is read, the exposure starts at Unix 1768528810 and lasts 1 s.
fn frame(width: u32, height: u32, binning: u32, roi: Region) -> SolveFrame {
    SolveFrame {
        path: PathBuf::new(),
        width,
        height,
        bits: 16,
        bayer: None,
        binning,
        roi,
        start: 1_768_528_810.0,
        exposure_s: 1.0,
    }
}

/// The whole sensor's region, 1236 × 822.
fn whole() -> Region {
    Region {
        x: 0,
        y: 0,
        width: 1236,
        height: 822,
    }
}

/// The region of interest 824, 0, 412, 274: a third of the sensor at its
/// top-right.
fn corner() -> Region {
    Region {
        x: 824,
        y: 0,
        width: 412,
        height: 274,
    }
}

/// One entry of the scripted solver's list (the shared setup): a star measure
/// of so many frame pixels and stars, or the error it answers.
enum Entry {
    Stars(f64, u32),
    Error(SolveError),
}

/// What one `measure` call was given (the shared setup): the frame's region of
/// interest, binning and size, and the region asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Call {
    roi: Region,
    binning: u32,
    width: u32,
    height: u32,
    region: Option<Region>,
}

/// The scripted solver (the shared setup): `measure` keeps a note of each call
/// and answers the next entry of its list at once, repeating the last once the
/// list is used up; `solve` answers `Failed("not used")`.
struct Scripted {
    entries: Vec<Entry>,
    next: AtomicUsize,
    calls: Mutex<Vec<Call>>,
}

impl Scripted {
    fn new(entries: Vec<Entry>) -> Self {
        Self {
            entries,
            next: AtomicUsize::new(0),
            calls: Mutex::new(Vec::new()),
        }
    }

    /// The `measure` calls the solver was given, in order.
    fn calls(&self) -> Vec<Call> {
        self.calls.lock().unwrap().clone()
    }

    /// The next entry's answer, the last one repeating once the list is used
    /// up.
    fn outcome(&self) -> Result<StarMeasure, SolveError> {
        let n = self.next.fetch_add(1, Ordering::SeqCst);
        let last = self.entries.len() - 1;
        match &self.entries[n.min(last)] {
            Entry::Stars(px, stars) => Ok(StarMeasure {
                half_flux_px: *px,
                stars: *stars,
            }),
            Entry::Error(error) => Err(error.clone()),
        }
    }
}

impl ImagingSolver for Scripted {
    fn name(&self) -> &'static str {
        "scripted"
    }

    fn solve(&self, _frame: SolveFrame, _hint: SolveHint) -> SolveFuture<'_> {
        Box::pin(async { Err(SolveError::Failed("not used".to_string())) })
    }

    fn measure(&self, frame: SolveFrame, region: Option<Region>) -> MeasureFuture<'_> {
        self.calls.lock().unwrap().push(Call {
            roi: frame.roi,
            binning: frame.binning,
            width: frame.width,
            height: frame.height,
            region,
        });
        let answer = self.outcome();
        Box::pin(async move { answer })
    }
}

/// One request with a foreign `Origin`, as a browser on another site sends it.
async fn foreign_origin(app: &Router, method: &str, path: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("host", "localhost:8080")
                .header("content-type", "application/json")
                .header("origin", "http://elsewhere.example")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    (status, value)
}

/// US1-S1: the simulated solver's reading is the star measure's size in sensor
/// pixels, the scale's arcseconds when one was given, whatever the sign of the
/// focus error and at either binning, with the frame id, the measured region
/// and the middle of the exposure (oracle: the simulator's star size H(0) and
/// H(2), and the scale 2.0″ a sensor pixel).
#[tokio::test(start_paused = true)]
async fn the_simulated_solver_s_reading_over_the_focus_rig() {
    let sim = sim();
    let solver = imaging::simulated::SimulatedSolver::new(sim.clone());

    // A whole frame at binning 1: H(0) sensor pixels.
    let reading = focus::read(&solver, frame(1236, 822, 1, whole()), 7, None, Some(2.0))
        .await
        .unwrap()
        .unwrap();
    assert!((reading.px - 2.825784).abs() < 1e-6, "{reading:?}");
    assert!(
        (reading.arcsec.expect("the scale was given") - 5.651568).abs() < 1e-5,
        "{reading:?}"
    );
    assert!(reading.stars >= 30, "{reading:?}");
    assert_eq!(reading.frame_id, 7);
    assert_eq!(reading.region, whole());
    assert_eq!(reading.at, 1_768_528_810.5);

    // The focuser turns: H(2) either way, at binning 1 and binning 2.
    sim.set_focus_error(2.0);
    assert_eq!(sim.focus_error(), 2.0);
    assert_eq!(sim.imaging_optics().focus_error, 2.0);
    let reading = focus::read(&solver, frame(1236, 822, 1, whole()), 7, None, Some(2.0))
        .await
        .unwrap()
        .unwrap();
    assert!((reading.px - 5.492337).abs() < 1e-4, "{reading:?}");
    sim.set_focus_error(-2.0);
    assert_eq!(sim.focus_error(), -2.0);
    let reading = focus::read(&solver, frame(1236, 822, 1, whole()), 7, None, Some(2.0))
        .await
        .unwrap()
        .unwrap();
    assert!((reading.px - 5.492337).abs() < 1e-4, "{reading:?}");
    let reading = focus::read(&solver, frame(618, 411, 2, whole()), 7, None, Some(2.0))
        .await
        .unwrap()
        .unwrap();
    assert!((reading.px - 5.492337).abs() < 1e-4, "{reading:?}");
    assert_eq!(reading.region, whole());

    // Without a scale the reading carries no arcseconds.
    let reading = focus::read(&solver, frame(1236, 822, 1, whole()), 7, None, None)
        .await
        .unwrap()
        .unwrap();
    assert!(reading.arcsec.is_none(), "{reading:?}");
}

/// US1-S2: a scripted star measure becomes sensor pixels by the frame's
/// binning and arcseconds by the scale given, the measured region is in sensor
/// pixels, and the solver is called with exactly the frame and region given
/// (oracle: the worked examples 3.10 × 0.548 = 1.6988 and 1.55 × 2 = 3.10).
#[tokio::test(start_paused = true)]
async fn the_scripted_solver_s_reading_of_binning_and_region() {
    let solver = Scripted::new(vec![Entry::Stars(3.10, 12)]);
    let reading = focus::read(&solver, frame(412, 274, 1, corner()), 7, None, Some(0.548))
        .await
        .unwrap()
        .unwrap();
    assert!((reading.px - 3.10).abs() < 1e-9, "{reading:?}");
    assert!(
        (reading.arcsec.expect("the scale was given") - 1.6988).abs() < 1e-9,
        "{reading:?}"
    );
    assert_eq!(reading.stars, 12);
    assert_eq!(reading.region, corner());
    assert_eq!(
        solver.calls(),
        vec![Call {
            roi: corner(),
            binning: 1,
            width: 412,
            height: 274,
            region: None,
        }]
    );

    let solver = Scripted::new(vec![Entry::Stars(1.55, 12)]);
    let tile = Region {
        x: 100,
        y: 50,
        width: 200,
        height: 100,
    };
    let reading = focus::read(&solver, frame(618, 411, 2, whole()), 7, Some(tile), None)
        .await
        .unwrap()
        .unwrap();
    assert!((reading.px - 3.10).abs() < 1e-9, "{reading:?}");
    assert_eq!(
        reading.region,
        Region {
            x: 200,
            y: 100,
            width: 400,
            height: 200,
        }
    );
    assert_eq!(
        solver.calls(),
        vec![Call {
            roi: whole(),
            binning: 2,
            width: 618,
            height: 411,
            region: Some(tile),
        }]
    );
}

/// US1-S3: fewer than five stars, or the solver's own TooFewStars, is
/// unmeasurable with the star count; the five-star rule comes before the
/// 20-frame-pixel rule, which is read in the frame's own pixels; and any other
/// error of the solver is `read`'s own, unchanged (oracle: the stated rule of
/// five stars and 20 frame pixels).
#[tokio::test(start_paused = true)]
async fn the_reading_of_too_few_stars_far_out_and_solver_errors() {
    let solver = Scripted::new(vec![
        Entry::Stars(3.0, 4),
        Entry::Stars(3.0, 5),
        Entry::Error(SolveError::TooFewStars { detail: None }),
        Entry::Stars(25.0, 3),
        Entry::Stars(20.0, 9),
        Entry::Stars(20.5, 9),
        Entry::Stars(15.0, 9),
        Entry::Error(SolveError::TimedOut { seconds: 30 }),
        Entry::Error(SolveError::NoDatabase { detail: None }),
        Entry::Error(SolveError::Failed("x".to_string())),
    ]);
    let whole_b1 = || frame(1236, 822, 1, whole());
    let whole_b2 = || frame(618, 411, 2, whole());

    // Four stars: unmeasurable, whatever the size.
    let answer = focus::read(&solver, whole_b1(), 7, None, None)
        .await
        .unwrap();
    assert_eq!(answer, Err(Unmeasurable::TooFewStars { stars: 4 }));

    // Five stars is a reading.
    let reading = focus::read(&solver, whole_b1(), 7, None, None)
        .await
        .unwrap()
        .unwrap();
    assert!((reading.px - 3.0).abs() < 1e-9, "{reading:?}");

    // The solver's own TooFewStars: unmeasurable with none measured.
    let answer = focus::read(&solver, whole_b1(), 7, None, None)
        .await
        .unwrap();
    assert_eq!(answer, Err(Unmeasurable::TooFewStars { stars: 0 }));

    // Twenty-five frame pixels with three stars: the five-star rule first.
    let answer = focus::read(&solver, whole_b1(), 7, None, None)
        .await
        .unwrap();
    assert_eq!(answer, Err(Unmeasurable::TooFewStars { stars: 3 }));

    // Twenty frame pixels is a reading of 20 sensor pixels at binning 1.
    let reading = focus::read(&solver, whole_b1(), 7, None, None)
        .await
        .unwrap()
        .unwrap();
    assert!((reading.px - 20.0).abs() < 1e-9, "{reading:?}");

    // Twenty and a half at binning 2 is far out at 41.0 sensor pixels, and
    // fifteen is a reading of 30.0: the limit is 20 frame pixels, not sensor
    // pixels.
    let answer = focus::read(&solver, whole_b2(), 7, None, None)
        .await
        .unwrap();
    assert_eq!(answer, Err(Unmeasurable::FarOut { px: 41.0, stars: 9 }));
    let reading = focus::read(&solver, whole_b2(), 7, None, None)
        .await
        .unwrap()
        .unwrap();
    assert!((reading.px - 30.0).abs() < 1e-9, "{reading:?}");

    // Any other error of the solver is `read`'s own, unchanged.
    assert_eq!(
        focus::read(&solver, whole_b1(), 7, None, None).await,
        Err(SolveError::TimedOut { seconds: 30 })
    );
    assert_eq!(
        focus::read(&solver, whole_b1(), 7, None, None).await,
        Err(SolveError::NoDatabase { detail: None })
    );
    assert_eq!(
        focus::read(&solver, whole_b1(), 7, None, None).await,
        Err(SolveError::Failed("x".to_string()))
    );
}

/// US1-S4: the seeded noise multiplies each star measure by a bounded factor of
/// a published stream, and the sharpest point grows the star size with the
/// distance of the measured region's centre from it; without either builder
/// the adapter answers what 032 specifies (oracle: the published SplitMix64
/// stream for the seed 0, whose first three values are 0xe220a8397b1dcdaf,
/// 0x6e789e6aa1b965f4 and 0x06c45d188009454f, and the worked sizes
/// 2·√(2·ln 2)·√(1.44 + (3.0·d)²) of each region's centre).
#[tokio::test(start_paused = true)]
async fn the_adapter_s_seeded_noise_and_sharpest_point() {
    let sim = sim();
    let whole_b1 = || frame(1236, 822, 1, whole());

    // The seed 0's first three answers, then fifty more within 2% of H(0).
    let noisy = imaging::simulated::SimulatedSolver::new(sim.clone()).with_measure_noise(0, 0.02);
    for expected in [2.869110, 2.818045, 2.772256] {
        let measure = noisy.measure(whole_b1(), None).await.unwrap();
        assert!(
            (measure.half_flux_px - expected).abs() < 1e-5,
            "{measure:?}"
        );
    }
    for _ in 0..50 {
        let measure = noisy.measure(whole_b1(), None).await.unwrap();
        assert!(
            (2.769268..=2.882300).contains(&measure.half_flux_px),
            "{measure:?}"
        );
    }

    // A second solver built the same way answers the same three.
    let noisy_again =
        imaging::simulated::SimulatedSolver::new(sim.clone()).with_measure_noise(0, 0.02);
    for expected in [2.869110, 2.818045, 2.772256] {
        let measure = noisy_again.measure(whole_b1(), None).await.unwrap();
        assert!(
            (measure.half_flux_px - expected).abs() < 1e-5,
            "{measure:?}"
        );
    }

    // Stars smallest at (0.8, 0.2) of the sensor, growing by 3.0 sensor
    // pixels of width at half the diagonal away.
    let sharp =
        imaging::simulated::SimulatedSolver::new(sim.clone()).with_sharpest_at(0.8, 0.2, 3.0);
    let measure = sharp.measure(whole_b1(), None).await.unwrap();
    assert!(
        (measure.half_flux_px - 5.094255).abs() < 1e-5,
        "{measure:?}"
    );
    let measure = sharp
        .measure(frame(618, 411, 2, whole()), None)
        .await
        .unwrap();
    assert!(
        (measure.half_flux_px - 2.547127).abs() < 1e-5,
        "{measure:?}"
    );
    let top = Region {
        x: 0,
        y: 0,
        width: 1236,
        height: 412,
    };
    let measure = sharp.measure(whole_b1(), Some(top)).await.unwrap();
    assert!(
        (measure.half_flux_px - 4.538578).abs() < 1e-5,
        "{measure:?}"
    );
    let measure = sharp
        .measure(frame(412, 274, 1, corner()), None)
        .await
        .unwrap();
    assert!(
        (measure.half_flux_px - 2.864762).abs() < 1e-5,
        "{measure:?}"
    );

    // Without either builder the adapter answers what 032 specifies.
    let plain = imaging::simulated::SimulatedSolver::new(sim.clone());
    let measure = plain.measure(whole_b1(), None).await.unwrap();
    assert!(
        (measure.half_flux_px - 2.825784).abs() < 1e-6,
        "{measure:?}"
    );
    let measure = plain.measure(whole_b1(), Some(top)).await.unwrap();
    assert!(
        (measure.half_flux_px - 2.825784).abs() < 1e-6,
        "{measure:?}"
    );
    let measure = plain
        .measure(frame(618, 411, 2, whole()), None)
        .await
        .unwrap();
    assert!(
        (measure.half_flux_px - 1.412892).abs() < 1e-6,
        "{measure:?}"
    );
}

/// US1-S5: `GET` and `POST /api/sim/focus` read and set the simulator's focus
/// error, a value outside the range, a missing key and an unknown key each
/// answer 400 with a JSON `error` and change nothing, a foreign `Origin` is
/// refused, and without a simulation both answer 404 (oracle: the stated
/// bodies, statuses and words).
#[tokio::test(start_paused = true)]
async fn the_simulate_route_reads_and_sets_the_focus_error() {
    let state = AppState::shared(support::DB.clone(), Samples::default())
        .with_simulation(support::START, SimulationTransport::Memory)
        .await
        .unwrap()
        .with_live();
    let sim = state.simulator().expect("a simulated console");
    let app = finder::router(state);

    let (status, answer) = support::request(&app, "GET", "/api/sim/focus", json!(null)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(answer, json!({"focus_error": 0.0}));

    let (status, answer) =
        support::request(&app, "POST", "/api/sim/focus", json!({"focus_error": 1.5})).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(answer, json!({"focus_error": 1.5}));
    assert_eq!(sim.focus_error(), 1.5);
    assert_eq!(sim.imaging_optics().focus_error, 1.5);
    let (status, answer) = support::request(&app, "GET", "/api/sim/focus", json!(null)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(answer, json!({"focus_error": 1.5}));

    // Three refused bodies, each changing nothing.
    let refused = [
        (
            json!({"focus_error": 101}),
            Some("focus_error must be a number from -100 to 100"),
        ),
        (json!({}), None),
        (json!({"focus_error": 1, "x": 2}), None),
    ];
    for (body, words) in refused {
        let (status, answer) = support::request(&app, "POST", "/api/sim/focus", body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{answer}");
        assert!(answer.get("error").is_some(), "{answer}");
        if let Some(words) = words {
            assert_eq!(answer["error"], json!(words), "{answer}");
        }
        assert_eq!(sim.focus_error(), 1.5);
    }
    let (status, answer) = support::request(&app, "GET", "/api/sim/focus", json!(null)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(answer, json!({"focus_error": 1.5}));

    // A foreign `Origin` is refused.
    let (status, answer) = foreign_origin(&app, "POST", "/api/sim/focus").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{answer}");
    assert_eq!(sim.focus_error(), 1.5);

    // A console built without a simulation answers 404 on both.
    let bare = finder::router(AppState::shared(support::DB.clone(), Samples::default()));
    let (status, _) = support::request(&bare, "GET", "/api/sim/focus", json!(null)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) =
        support::request(&bare, "POST", "/api/sim/focus", json!({"focus_error": 1.5})).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
