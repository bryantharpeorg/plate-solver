//! Oracles: simulator truth at mid-exposure (30 arcseconds), worked clock mapping
//! (1 microsecond), and the specified estimate/failure sequence (exact).
#[path = "live/support.rs"]
mod support;
use finder::{
    live::{SolveFuture, Solver},
    session::{FixedClock, Session},
    sky::Frame,
    solve::{self, SolveOptions, Status},
};
use math_core::{angular_distance, PinholeCamera, UnitVector};
use scope_link::proto::StarFrame;
use serde_json::json;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use support::*;

#[tokio::test(start_paused = true)]
async fn every_frame_for_ten_seconds_solves_at_the_true_eyepiece_and_reports_field() {
    let sim = sim();
    let state = state(&sim, None, true);
    let link = state.link().unwrap();
    let mut events = state.live().unwrap().subscribe();
    let app = finder::router(state);
    let mut ids = Vec::new();
    for _ in 0..20 {
        let report = event(&mut events).await;
        assert_eq!(report.report.status, Status::MatchFound);
        let (_, session) = request(&app, "GET", "/api/session", json!(null)).await;
        let p = &session["pointing"];
        assert_eq!(p["frame_id"], report.frame_id);
        assert_eq!(p["source"], "live");
        let truth = sim.directions_at(report.time).eyepiece;
        let error = angular_distance(
            UnitVector::from_radec(
                p["ra_deg"].as_f64().unwrap().to_radians(),
                p["dec_deg"].as_f64().unwrap().to_radians(),
            ),
            UnitVector::from_radec(truth.ra.to_radians(), truth.dec.to_radians()),
        )
        .to_degrees()
            * 3600.0;
        assert!(
            error < 30.0,
            "frame {} error {error} arcsec",
            report.frame_id
        );
        assert!((p["alt_deg"].as_f64().unwrap() - truth.alt).abs() * 3600.0 < 30.0);
        assert!((session["field"]["fov_deg"].as_f64().unwrap() - 12.0).abs() < 0.1);
        assert_eq!(session["field"]["db_min_fov_deg"], 10.0);
        assert_eq!(session["field"]["db_max_fov_deg"], 30.0);
        assert_eq!(session["field"]["unsolved_s"], 0.0);
        ids.push(report.frame_id);
    }
    assert!(ids.windows(2).all(|w| w[1] == w[0] + 1), "{ids:?}");
    assert!(sim.clock().now() - START >= 9.5);
    assert_eq!(
        ids.last().copied(),
        link.frames().borrow().as_ref().map(|f| f.id)
    );
}

struct Slow {
    newest: Mutex<Option<tokio::sync::watch::Receiver<Option<StarFrame>>>>,
    starts: Mutex<Vec<(u64, u64)>>,
}
impl Solver for Slow {
    fn solve(&self, frame: StarFrame, options: SolveOptions) -> SolveFuture<'_> {
        let newest = self
            .newest
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .borrow()
            .as_ref()
            .unwrap()
            .id;
        self.starts.lock().unwrap().push((frame.id, newest));
        Box::pin(async move {
            tokio::time::sleep(Duration::from_millis(1500)).await;
            solve::solve_centroids(
                &DB,
                &[],
                (frame.width as usize, frame.height as usize),
                &options,
            )
        })
    }
}
#[tokio::test(start_paused = true)]
async fn slow_solves_always_start_on_the_newest_frame_without_a_queue() {
    let sim = sim();
    let slow = Arc::new(Slow {
        newest: Mutex::new(None),
        starts: Mutex::new(Vec::new()),
    });
    let state = state(&sim, Some(slow.clone()), false);
    *slow.newest.lock().unwrap() = Some(state.link().unwrap().frames());
    let mut events = state.live().unwrap().subscribe();
    for _ in 0..6 {
        event(&mut events).await;
    }
    let starts = slow.starts.lock().unwrap();
    assert!(starts.len() >= 6);
    assert!(
        starts.iter().all(|(started, newest)| started == newest),
        "{starts:?}"
    );
    assert!(starts.windows(2).all(|w| w[1].0 > w[0].0 + 1), "{starts:?}");
}

struct Sequence(Mutex<Vec<SolveOptions>>);
impl Solver for Sequence {
    fn solve(&self, frame: StarFrame, options: SolveOptions) -> SolveFuture<'_> {
        let mut calls = self.0.lock().unwrap();
        let n = calls.len();
        calls.push(options);
        Box::pin(async move {
            let (mut report, _) = solve::solve_centroids(
                &DB,
                &[],
                (frame.width as usize, frame.height as usize),
                &options,
            );
            let solved = (n == 0).then(|| {
                Frame::from_solution(&plate_solver::Solution {
                    ra: Some(1.0),
                    dec: Some(0.2),
                    roll: Some(0.0),
                    camera: Some(PinholeCamera::new(1456.0, 1088.0, 12.04_f64.to_radians())),
                    ..Default::default()
                })
                .unwrap()
            });
            if solved.is_some() {
                report.status = Status::MatchFound;
            }
            (report, solved)
        })
    }
}
#[tokio::test(start_paused = true)]
async fn field_estimate_carries_with_five_percent_allowance_then_three_failures_clear_it() {
    let sim = sim();
    let sequence = Arc::new(Sequence(Mutex::new(Vec::new())));
    let state = state(&sim, Some(sequence.clone()), false);
    let mut events = state.live().unwrap().subscribe();
    for _ in 0..5 {
        event(&mut events).await;
    }
    let calls = sequence.0.lock().unwrap();
    assert_eq!(calls[0].fov, None);
    for o in &calls[1..4] {
        assert!((o.fov.unwrap() - 12.04).abs() < 1e-10);
        assert!((o.fov_max_error - 0.602).abs() < 1e-10);
    }
    assert_eq!(calls[4].fov, None);
}
#[tokio::test(start_paused = true)]
async fn worked_mid_exposure_mapping_is_200_point_050() {
    let session = Arc::new(Session::default());
    let (tx, frames) = tokio::sync::watch::channel(None);
    let (_status_tx, status) = tokio::sync::watch::channel(finder::link::LinkStatus {
        connected: true,
        offset_s: Some(0.050),
        ..Default::default()
    });
    let live = finder::live::Live::start(
        session.clone(),
        Arc::new(FixedClock(200.2)),
        frames,
        status,
        Arc::new(Sequence(Mutex::new(Vec::new()))),
        Arc::new(tokio::sync::Semaphore::new(1)),
        (10.0, 30.0),
    );
    let mut events = live.subscribe();
    tx.send_replace(Some(StarFrame {
        id: 7,
        start_s: 200.0,
        end_s: 200.2,
        width: 1456,
        height: 1088,
        centroids: vec![Default::default(); 4],
        ..Default::default()
    }));
    let report = event(&mut events).await;
    assert!((report.time - 200.050).abs() < 1e-6);
    assert!((session.snapshot(200.2).pointing.unwrap().time - 200.050).abs() < 1e-6);
}
#[tokio::test(start_paused = true)]
async fn tap_six_pixels_off_snaps_to_the_eyepiece_and_later_pointings_follow_it() {
    let sim = sim();
    let star = sim.field_stars(200.0, 1.0)[0].0;
    let (ra, dec) = DB
        .star_vector(pattern_database::StarId(star))
        .unwrap()
        .normalize()
        .unwrap()
        .to_radec();
    sim.point_eyepiece_at(ra.to_degrees(), dec.to_degrees());
    let state = state(&sim, None, false);
    let live = state.live().unwrap();
    let mut events = live.subscribe();
    let app = finder::router(state);
    let first = event(&mut events).await;
    let id = first.preview_seq.expect("first frame carries a preview");
    let (status, read) = request(
        &app,
        "POST",
        "/api/aim",
        json!({"frame_id":id,"x":706,"y":420}),
    )
    .await;
    assert_eq!(status, 200);
    assert!((read["aim"]["x"].as_f64().unwrap() - 700.0).abs() < 0.5);
    assert!((read["aim"]["y"].as_f64().unwrap() - 420.0).abs() < 0.5);
    for _ in 0..3 {
        let report = event(&mut events).await;
        let (_, read) = request(&app, "GET", "/api/session", json!(null)).await;
        let p = &read["pointing"];
        let truth = sim.directions_at(report.time).eyepiece;
        assert!(
            angular_distance(
                UnitVector::from_radec(
                    p["ra_deg"].as_f64().unwrap().to_radians(),
                    p["dec_deg"].as_f64().unwrap().to_radians()
                ),
                UnitVector::from_radec(truth.ra.to_radians(), truth.dec.to_radians())
            )
            .to_degrees()
                * 3600.0
                < 30.0
        );
    }
}
#[tokio::test(start_paused = true)]
async fn rich_frames_failing_for_thirty_seconds_report_the_whole_unsolved_interval() {
    let sim = sim();
    let state = state(&sim, Some(Arc::new(Fail)), false);
    let mut events = state.live().unwrap().subscribe();
    let app = finder::router(state);
    let first = event(&mut events).await;
    assert!(first.stars >= 25, "{}", first.stars);
    let begin = sim.clock().now();
    while sim.clock().now() - begin < 30.0 {
        let e = event(&mut events).await;
        assert!(e.stars >= 25);
        assert!(e.pointing_seq.is_none());
    }
    let (_, read) = request(&app, "GET", "/api/session", json!(null)).await;
    assert!(
        read["field"]["unsolved_s"].as_f64().unwrap() >= 30.0,
        "{read}"
    );
    assert!(read["field"]["fov_deg"].is_null());
}

#[tokio::test(start_paused = true)]
async fn immediate_failures_publish_only_after_receiving_the_preview_and_field() {
    let session = Arc::new(Session::default());
    let (tx, frames) = tokio::sync::watch::channel(None);
    let (_status_tx, status) = tokio::sync::watch::channel(finder::link::LinkStatus {
        connected: true,
        offset_s: Some(0.0),
        ..Default::default()
    });
    let live = finder::live::Live::start(
        session.clone(),
        Arc::new(FixedClock(200.2)),
        frames,
        status,
        Arc::new(Fail),
        Arc::new(tokio::sync::Semaphore::new(1)),
        (10.0, 30.0),
    );
    let mut events = live.subscribe();
    for id in 0..32 {
        let jpeg = vec![id as u8];
        tx.send_replace(Some(StarFrame {
            id,
            start_s: 200.0,
            end_s: 200.2,
            width: 1456,
            height: 1088,
            centroids: vec![Default::default(); 25],
            preview_jpeg: Some(jpeg.clone()),
            ..Default::default()
        }));
        let report = event(&mut events).await;
        assert_eq!(report.frame_id, id);
        assert_eq!(report.preview_seq, Some(id));
        assert_eq!(live.preview(id).unwrap().jpeg, jpeg);
        let field = session.snapshot(200.2).field.unwrap();
        assert_eq!(field.db_min_fov_deg, 10.0);
        assert_eq!(field.db_max_fov_deg, 30.0);
        assert_eq!(field.fov_deg, None);
        if id >= 8 {
            assert!(live.preview(id - 8).is_none());
        }
    }
}

struct HeldSolver {
    starts: tokio::sync::mpsc::UnboundedSender<u64>,
    release: tokio::sync::Semaphore,
}
impl Solver for HeldSolver {
    fn solve(&self, frame: StarFrame, options: SolveOptions) -> SolveFuture<'_> {
        self.starts.send(frame.id).unwrap();
        Box::pin(async move {
            self.release.acquire().await.unwrap().forget();
            solve::solve_centroids(&DB, &[], (1456, 1088), &options)
        })
    }
}

#[tokio::test(start_paused = true)]
async fn previews_arrive_while_busy_and_the_next_solve_takes_only_the_newest_frame() {
    let session = Arc::new(Session::default());
    let (tx, frames) = tokio::sync::watch::channel(None);
    let (_status_tx, status) = tokio::sync::watch::channel(finder::link::LinkStatus {
        connected: true,
        offset_s: Some(0.0),
        ..Default::default()
    });
    let (starts, mut started) = tokio::sync::mpsc::unbounded_channel();
    let solver = Arc::new(HeldSolver {
        starts,
        release: tokio::sync::Semaphore::new(0),
    });
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let live = finder::live::Live::start(
        session.clone(),
        Arc::new(FixedClock(200.2)),
        frames,
        status,
        solver.clone(),
        gate.clone(),
        (10.0, 30.0),
    );
    let mut events = live.subscribe();
    for id in 1..=4 {
        tx.send_replace(Some(StarFrame {
            id,
            start_s: 200.0,
            end_s: 200.2,
            width: 1456,
            height: 1088,
            centroids: vec![Default::default(); 25],
            preview_jpeg: Some(vec![id as u8]),
            ..Default::default()
        }));
        tick().await;
        assert_eq!(live.preview(id).unwrap().jpeg, vec![id as u8]);
        assert!(session.snapshot(200.2).field.is_some());
        if id == 1 {
            assert!(started.try_recv().is_err());
        } else if id == 2 {
            gate.add_permits(1);
            assert_eq!(started.recv().await, Some(2));
        }
    }
    assert!(started.try_recv().is_err());
    solver.release.add_permits(1);
    let first = event(&mut events).await;
    assert_eq!(first.frame_id, 2);
    assert_eq!(first.preview_seq, Some(4));
    assert_eq!(started.recv().await, Some(4));
    solver.release.add_permits(1);
    assert_eq!(event(&mut events).await.frame_id, 4);
    assert!(started.try_recv().is_err());
}

#[test]
fn unprojection_inverts_camera_distortion_and_rotation() {
    let frame = Frame::from_solution(&plate_solver::Solution {
        ra: Some(1.2),
        dec: Some(-0.3),
        roll: Some(0.7),
        distortion: Some(-0.12),
        camera: Some(PinholeCamera::new(1456.0, 1088.0, 12_f64.to_radians())),
        ..Default::default()
    })
    .unwrap();
    for pixel in [(728.0, 544.0), (700.0, 420.0), (30.0, 900.0)] {
        let ray = frame.unproject(pixel).unwrap();
        let back = frame.project(ray).unwrap();
        assert!((back.0 - pixel.0).hypot(back.1 - pixel.1) < 1e-5);
    }
    assert!(angular_distance(frame.center, frame.unproject((728.0, 544.0)).unwrap()) < 1e-12);
    assert!(frame.unproject((f64::NAN, 0.0)).is_none());
}

#[tokio::test(start_paused = true)]
async fn fewer_than_four_stars_count_toward_losing_the_estimate() {
    let session = Arc::new(Session::default());
    let (tx, frames) = tokio::sync::watch::channel(None);
    let (_status_tx, status) = tokio::sync::watch::channel(finder::link::LinkStatus {
        connected: true,
        offset_s: Some(0.0),
        ..Default::default()
    });
    let solver = Arc::new(Sequence(Mutex::new(Vec::new())));
    let live = finder::live::Live::start(
        session,
        Arc::new(FixedClock(200.2)),
        frames,
        status,
        solver.clone(),
        Arc::new(tokio::sync::Semaphore::new(1)),
        (10.0, 30.0),
    );
    let mut events = live.subscribe();
    for (id, count) in [4, 3, 0, 2, 4].into_iter().enumerate() {
        tx.send_replace(Some(StarFrame {
            id: id as u64,
            start_s: 200.0,
            end_s: 200.2,
            width: 1456,
            height: 1088,
            centroids: vec![Default::default(); count],
            ..Default::default()
        }));
        let event = event(&mut events).await;
        if id > 0 && id < 4 {
            assert_eq!(event.report.status, Status::TooFew);
        }
    }
    let calls = solver.0.lock().unwrap();
    assert_eq!(calls.len(), 5);
    assert_eq!(calls[4].fov, None);
}
