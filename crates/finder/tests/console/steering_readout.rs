//! US8-S1/S2: the spec's worked errors, steps and last-ten arithmetic are the
//! oracle. All sky and pixel figures have an absolute tolerance of 0.0001.
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use finder::{
    imaging::{
        solver::{ImagingSolve, Region, SolveFrame},
        steering::{ExposureAsk, GapFuture, Steered, Steering, Stepped, TrackingError},
    },
    session::Clock,
};
use serde_json::Value;

const START: f64 = 1_768_528_800.0;
const TARGET: (f64, f64) = (83.81867, -5.38967);
const CD: [[f64; 2]; 2] = [
    [-5.196152422706632e-4, -3.0e-4],
    [3.0e-4, -5.196152422706632e-4],
];
const KEYS: [&str; 4] = [
    "error_rms_arcsec",
    "error_rms_px",
    "drift_arcsec",
    "drift_px",
];

#[derive(Default)]
struct SetClock(Mutex<f64>);
impl Clock for SetClock {
    fn now(&self) -> f64 {
        *self.0.lock().unwrap()
    }
}
#[derive(Default)]
struct Script {
    gaps: VecDeque<(Stepped, bool)>,
    unsteady_once: bool,
    limit: Option<f64>,
    errors: Vec<TrackingError>,
}
#[derive(Default)]
struct Scripted(Mutex<Script>);
impl Steered for Scripted {
    fn target(&self) -> Option<(f64, f64)> {
        Some(TARGET)
    }
    fn steer(&self, error: TrackingError) {
        self.0.lock().unwrap().errors.push(error);
    }
    fn gap(&self) -> GapFuture {
        let mut script = self.0.lock().unwrap();
        let (step, unsteady) = script.gaps.pop_front().unwrap_or_default();
        script.unsteady_once = unsteady;
        Box::pin(async move { step })
    }
    fn release(&self) {}
    fn steered_by_subs(&self) -> bool {
        true
    }
    fn steady(&self) -> bool {
        !std::mem::take(&mut self.0.lock().unwrap().unsteady_once)
    }
    fn longest_exposure_s(&self, _: f64) -> Option<f64> {
        self.0.lock().unwrap().limit
    }
}
fn ask() -> ExposureAsk {
    ExposureAsk {
        wanted_s: 5.0,
        shortest_s: 0.001,
        sensor: (1236, 822),
        binning: 1,
        roi: Region {
            x: 0,
            y: 0,
            width: 1236,
            height: 822,
        },
    }
}
fn frame(index: u32) -> SolveFrame {
    SolveFrame {
        path: "unused.fits".into(),
        width: 1236,
        height: 822,
        bits: 16,
        bayer: None,
        binning: 1,
        roi: ask().roi,
        start: START + 6.5 * f64::from(index),
        exposure_s: 5.0,
    }
}
fn solve(east: f64, north: f64, scale_share: f64) -> ImagingSolve {
    ImagingSolve::from_cd(
        TARGET.0 + east / (3600.0 * TARGET.1.to_radians().cos()),
        TARGET.1 + north / 3600.0,
        CD.map(|row| row.map(|value| value * scale_share)),
        1236,
        822,
    )
}
fn near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() <= 1e-4, "{actual} vs {expected}");
}
struct Rig {
    steering: Arc<Steering>,
    clock: Arc<SetClock>,
    tracker: Arc<Scripted>,
}
impl Rig {
    fn new() -> Self {
        let clock = Arc::new(SetClock::default());
        let steering = Steering::new(clock.clone());
        let tracker = Arc::new(Scripted::default());
        steering.set_source(Arc::new({
            let tracker = tracker.clone();
            move || Some(tracker.clone())
        }));
        Self {
            steering,
            clock,
            tracker,
        }
    }
    async fn gap(&self, index: u32, east: f64, north: f64) {
        *self.clock.0.lock().unwrap() = frame(index).start;
        self.tracker.0.lock().unwrap().gaps.push_back((
            Stepped {
                east_arcsec: east,
                north_arcsec: north,
            },
            false,
        ));
        self.steering.before_exposure(ask()).await;
    }
    fn accept(&self, index: u32, east: f64, north: f64, scale: f64) {
        self.steering
            .accepted(&frame(index), &solve(east, north, scale));
        let error = *self.tracker.0.lock().unwrap().errors.last().unwrap();
        near(error.east_arcsec, east);
        near(error.north_arcsec, north);
        assert_eq!(error.time, frame(index).start + 2.5);
    }
    fn view(&self) -> Value {
        serde_json::to_value(self.steering.view().unwrap()).unwrap()
    }
    fn figure(&self, key: &str, expected: f64) {
        near(self.view()[key].as_f64().unwrap(), expected);
    }
    fn absent(&self) {
        for key in KEYS {
            assert!(self.view().get(key).is_none(), "{key}");
        }
    }
}

#[tokio::test(start_paused = true)]
async fn worked_errors_and_step_give_rms_in_sky_and_latest_pixels() {
    let rig = Rig::new();
    rig.gap(0, 0.0, 0.0).await;
    rig.accept(0, 0.0, 0.0, 1.0);
    rig.absent();
    rig.gap(1, 0.0, 0.0).await;
    rig.accept(1, 0.6, 0.8, 1.0);
    rig.figure("drift_arcsec", 0.76923);
    rig.figure("error_rms_arcsec", std::f64::consts::FRAC_1_SQRT_2);
    rig.gap(2, -0.5, 0.0).await;
    rig.accept(2, 0.1, 1.2, 1.0);
    rig.figure("drift_arcsec", 0.58583);
    rig.figure("drift_px", 0.27122);
    rig.figure("error_rms_arcsec", 0.90370);
    rig.figure("error_rms_px", 0.41838);
}

#[tokio::test(start_paused = true)]
async fn latest_scale_and_step_before_first_error() {
    let rig = Rig::new();
    rig.gap(0, 0.3, 0.0).await;
    rig.accept(0, 0.0, 0.0, 1.0);
    rig.gap(1, 0.0, 0.0).await;
    rig.accept(1, 0.6, 0.8, 0.5);
    rig.figure("drift_arcsec", 0.76923);
    rig.figure(
        "error_rms_px",
        rig.view()["error_rms_arcsec"].as_f64().unwrap() / 1.08,
    );
    rig.figure("drift_px", 0.76923 / 1.08);
}

#[tokio::test(start_paused = true)]
async fn only_steps_between_mid_exposures_count_even_for_late_errors_or_retried_gaps() {
    // The same 0.4 arcsec residual, with a future gap, two answers in one call,
    // or a north step. Each must give the worked RMS of 0.58583 arcsec.
    for variant in 0..3 {
        let rig = Rig::new();
        rig.gap(0, 0.0, 0.0).await;
        rig.accept(0, 0.0, 0.0, 1.0);
        rig.gap(1, 0.0, 0.0).await;
        let north = variant == 2;
        rig.accept(
            1,
            if north { 0.8 } else { 0.6 },
            if north { 0.6 } else { 0.8 },
            1.0,
        );
        if variant == 1 {
            rig.tracker.0.lock().unwrap().gaps.push_back((
                Stepped {
                    east_arcsec: 0.3,
                    north_arcsec: 0.0,
                },
                true,
            ));
            rig.gap(2, -0.8, 0.0).await;
            assert!(rig.tracker.0.lock().unwrap().gaps.is_empty());
        } else if north {
            rig.gap(2, 0.0, -0.5).await;
        } else {
            rig.gap(2, -0.5, 0.0).await;
            rig.gap(3, 0.4, 0.0).await;
        }
        rig.accept(
            2,
            if north { 1.2 } else { 0.1 },
            if north { 0.1 } else { 1.2 },
            1.0,
        );
        rig.figure("drift_arcsec", 0.58583);
    }
}

#[tokio::test(start_paused = true)]
async fn last_ten_errors_exclude_first_drift_and_release_empties_window() {
    let rig = Rig::new();
    for index in 0..12 {
        rig.gap(index, 0.0, 0.0).await;
        rig.accept(
            index,
            match index {
                0 | 1 => 1.0,
                2 => 2.0,
                _ => 0.0,
            },
            0.0,
            1.0,
        );
    }
    // sqrt(4/10); nine drifts, only one nonzero, 2 * 5/6.5 / sqrt(9).
    rig.figure("error_rms_arcsec", 0.63246);
    rig.figure("drift_arcsec", 0.51282);
    rig.steering.release();
    rig.gap(12, 0.0, 0.0).await;
    rig.absent();
}

#[tokio::test(start_paused = true)]
async fn releasing_for_limit_empties_readout_while_waiting() {
    let rig = Rig::new();
    for index in 0..3 {
        rig.gap(index, 0.0, 0.0).await;
        rig.accept(index, f64::from(index), 0.0, 1.0);
    }
    rig.figure("error_rms_arcsec", (5.0_f64 / 3.0).sqrt());
    rig.tracker.0.lock().unwrap().limit = Some(2.0);
    let steering = rig.steering.clone();
    let waiting = tokio::spawn(async move { steering.before_exposure(ask()).await });
    tokio::task::yield_now().await;
    assert!(rig.steering.waiting());
    assert_eq!(rig.view()["waiting"], true);
    rig.absent();
    waiting.abort();
    assert!(waiting.await.unwrap_err().is_cancelled());
}

#[tokio::test(start_paused = true)]
async fn stored_drifts_survive_eviction_of_exposure_records() {
    let rig = Rig::new();
    // One in five subs is accepted: ten errors span more than 32 records.
    for index in 0..50 {
        rig.gap(index, 0.0, 0.0).await;
        if index % 5 == 0 {
            rig.accept(index, f64::from(index / 5), 0.0, 1.0);
        }
    }
    rig.figure("drift_arcsec", 5.0 / 32.5);
    rig.figure("error_rms_arcsec", (285.0_f64 / 10.0).sqrt());
}
