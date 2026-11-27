//! The record's altitude and the judging at Track's altitude (054 US3), on the
//! motion rig: the record keeps the altitude it was read at, a check judges a
//! reading against the record carried to the reading's altitude, and the README
//! says so. Oracles: the airmass written here as the secant of the zenith
//! distance, the exponent 0.6 and the bounds 1 and 1.30 written here; 033's
//! thresholds 1.10 and 1.30; the simulator's own altitude of the main telescope;
//! the scale the view shows, checked against the focal length and the pixel
//! (206264.8″ × 19.12 µm ÷ 1800 mm, about 2.191″); and the README's words as
//! they are written.

use std::{path::Path, sync::Arc, time::Duration};

use axum::{http::StatusCode, Router};
use finder::{
    imaging::simulated::SimulatedSolver,
    samples::Samples,
    session::{Aim, Session},
    AppState,
};
use scope_link::memory;
use serde_json::{json, Value};
use simulator::{mount::SimMount, SimNode, Simulator, Site, VirtualClock};

use crate::support::{goto::Stars, live as support, Scratch};

const TICK: Duration = Duration::from_millis(100);

/// The airmass of an altitude in degrees: the secant of the zenith distance,
/// written here and not taken from the code under test.
fn airmass(alt_deg: f64) -> f64 {
    1.0 / alt_deg.to_radians().sin()
}

/// A record of `px` pixels read at `from_deg`, as it judges a reading at
/// `to_deg`: times the airmass ratio to the power 0.6, held between 1 and 1.30.
fn carried_px(px: f64, from_deg: f64, to_deg: f64) -> f64 {
    px * (airmass(to_deg) / airmass(from_deg))
        .powf(0.6)
        .clamp(1.0, 1.30)
}

/// 033's `focus.json`, written by hand with 054's altitude: tonight's record of
/// `px` pixels, read at `alt_deg` degrees at `at`, of the rig's own camera (the
/// simulated imaging camera, 1236 × 822 at 19.12 µm).
fn write_record(root: &Path, px: f64, at: f64, alt_deg: f64) {
    std::fs::create_dir_all(root).unwrap();
    let file = json!({
        "format": "imaging-focus/1",
        "camera": {
            "model": "simulated imaging camera",
            "serial": "SIM-0001",
            "width": 1236,
            "height": 822,
            "pixel_um": 19.12
        },
        "record": {
            "tonight": { "px": px, "at": at, "since": at, "alt_deg": alt_deg }
        }
    });
    std::fs::write(root.join("focus.json"), file.to_string()).unwrap();
}

/// The motion rig (spec.md, § User Scenarios): the simulator with its mount
/// pushed to 50° and 120° and the focus error given, served over a duplex by
/// its node; the session with that node, the site 40° N 75° W (the simulator's)
/// and the target M42; the console with the imaging folder `root`, the
/// simulated solver and `with_focal_length_mm(1800.0)`, so that the scale is
/// about 2.19″ a pixel. Ready and synced, with the aim at the eyepiece pixel.
struct Rig {
    app: Router,
    sim: Simulator,
    session: Arc<Session>,
}

async fn rig(root: &Path, focus_error: f64) -> Rig {
    let sim = Simulator::new(
        support::DB.clone(),
        Site::default(),
        VirtualClock::new(support::START),
    );
    sim.connect_mount(SimMount::new(sim.clock().clone()));
    sim.push_to(50.0, 120.0);
    sim.set_focus_error(focus_error);
    let node = Arc::new(
        SimNode::new(sim.clone(), "test-token", Default::default())
            .unwrap()
            .with_frame_interval(Duration::from_millis(500)),
    );
    let transport = finder::link::Transport::connector(move || {
        let node = node.clone();
        async move {
            let (client, server) = tokio::io::duplex(64 * 1024);
            let service = node.service();
            tokio::spawn(async move {
                memory::serve_routes_on(server, service.routes())
                    .await
                    .unwrap();
            });
            Ok::<_, std::io::Error>(client)
        }
    });
    let session = Session::default();
    let site = Site::default();
    session
        .edit(
            serde_json::from_value(json!({
                "node": {"address": "scope.local:1", "token": "test-token"},
                "site": {"lat_deg": site.lat_deg, "lon_deg": site.lon_deg},
                "target": "M42"
            }))
            .unwrap(),
            support::START,
        )
        .unwrap();
    let state = AppState::shared(support::DB.clone(), Samples::default())
        .with_session(session)
        .with_clock(Arc::new(support::ConsoleClock(sim.clock().clone())))
        .with_link(transport)
        .with_live_solver(Arc::new(Stars))
        .with_imaging(root)
        .with_imaging_solver(Arc::new(SimulatedSolver::new(sim.clone())))
        .with_focal_length_mm(1800.0);
    let rig = Rig {
        app: finder::router(state.clone()),
        sim,
        session: state.session(),
    };
    rig.wait("/api/imaging", |v| v["state"] == "ready").await;
    rig.wait("/api/session", |v| {
        v["model"]["syncs"].as_u64().is_some_and(|n| n > 0)
    })
    .await;
    let (x, y) = rig.sim.optics().eyepiece_px;
    rig.session
        .set_aim(Some(Aim { x, y }), rig.sim.clock().now())
        .unwrap();
    rig
}

impl Rig {
    async fn ok(&self, method: &str, path: &str, body: Value) -> Value {
        let (status, value) = support::request(&self.app, method, path, body).await;
        assert_eq!(status, StatusCode::OK, "{method} {path}: {value}");
        value
    }

    async fn get(&self, path: &str) -> Value {
        self.ok("GET", path, Value::Null).await
    }

    async fn wait(&self, path: &str, done: impl Fn(&Value) -> bool) -> Value {
        for _ in 0..9000 {
            let view = self.get(path).await;
            if done(&view) {
                return view;
            }
            tokio::time::sleep(TICK).await;
        }
        panic!("{path} did not reach its state: {}", self.get(path).await);
    }
}

/// A focus check, started now and read to its end, which it reaches by itself.
/// Answers the imaging view at that end; the run is told apart from an earlier
/// one by its start.
async fn check(rig: &Rig) -> Value {
    let before = rig.get("/api/imaging").await;
    let before = before["focus"]["aid"]["since"].clone();
    rig.ok("POST", "/api/imaging/focus", json!({"check": true}))
        .await;
    rig.wait("/api/imaging", |view| {
        view["focus"]["aid"]["state"] == "ended" && view["focus"]["aid"]["since"] != before
    })
    .await
}

/// US3-S1: a check made while Track is armed for its GoTo judges against the
/// record as it was read, and one made once Track is tracking judges against the
/// record carried to the altitude the reading was read at. The record is 2.0 px
/// read at 74°, written by hand; the simulator's scale is about 2.191″ a pixel.
#[tokio::test(start_paused = true)]
async fn the_check_judges_at_the_altitude_track_holds() {
    let scratch = Scratch::new("imaging-focus-altitude-check");
    let root = scratch.0.join("imaging");
    write_record(&root, 2.0, support::START - 600.0, 74.0);
    let rig = rig(&root, 0.0).await;

    // The first check is made at once, while Track is still armed for its GoTo:
    // it ends with no altitude, its reference uncarried.
    rig.ok("POST", "/api/track", json!({"on": true})).await;
    let first = check(&rig).await;
    let track = rig.get("/api/session").await;
    assert_eq!(track["track"]["state"], "armed", "{track}");
    let aid = &first["focus"]["aid"];
    assert_eq!(aid["ended_reason"], "done", "{first}");
    assert!(aid["reference"].is_object(), "{first}");
    assert!(aid["reference"].get("carried").is_none(), "{first}");
    assert!(aid["last"].is_object(), "{first}");
    assert!(aid["last"].get("alt_deg").is_none(), "{first}");

    // Track tracks; fifteen seconds later, a second check.
    rig.wait("/api/session", |v| v["track"]["state"] == "tracking")
        .await;
    tokio::time::sleep(Duration::from_secs(15)).await;
    let second = check(&rig).await;
    let true_alt = rig.sim.telescope().eyepiece.alt;
    let aid = &second["focus"]["aid"];
    assert_eq!(aid["ended_reason"], "done", "{second}");

    // The scale the view shows, from the focal length (1800 mm) and the pixel
    // (19.12 µm): 206264.8″ × 19.12 µm ÷ 1800 mm.
    let s = second["focus"]["arcsec_per_px"].as_f64().unwrap();
    assert!((s - 2.191).abs() < 0.01, "scale {s}");

    // The carried record: read at 74°, carried to the reading's altitude, which is
    // within half a degree of the simulator's.
    let carried = &aid["reference"]["carried"];
    assert_eq!(carried["from_alt_deg"].as_f64(), Some(74.0), "{second}");
    let alt = carried["alt_deg"].as_f64().unwrap();
    assert!(
        (alt - true_alt).abs() < 0.5,
        "altitude {alt} against the simulator's {true_alt}"
    );
    assert!((carried["from"].as_f64().unwrap() - 2.0 * s).abs() < 1e-9);
    assert_eq!(aid["last"]["alt_deg"].as_f64(), Some(alt), "{second}");

    let value = aid["reference"]["value"].as_f64().unwrap();
    let expected = carried_px(2.0 * s, 74.0, alt);
    assert!(
        (value - expected).abs() <= 1e-9 * expected,
        "reference {value} against {expected}"
    );

    // The verdict: 033's thresholds against the carried record. The median is
    // above what the record would be uncarried, so the carry is what lets it pass.
    let median = aid["median"]["value"].as_f64().unwrap();
    let ratio = median / value;
    let verdict = if ratio <= 1.10 {
        "good"
    } else if ratio <= 1.30 {
        "slightly_soft"
    } else {
        "clearly_out"
    };
    assert_eq!(verdict, "slightly_soft", "ratio {ratio}");
    assert_eq!(aid["verdict"], json!(verdict), "{second}");
    assert!(median > 1.30 * 2.0 * s, "median {median} against {s}");

    assert_eq!(
        second["focus"]["record"]["tonight"]["alt_deg"].as_f64(),
        Some(74.0),
        "{second}"
    );
}

/// US3-S2: the record made with Track off keeps no altitude, and the record made
/// with Track on, once it is tracking, keeps the altitude of its reading within
/// half a degree of the simulator's. A check made with no record has no reference.
#[tokio::test(start_paused = true)]
async fn the_record_keeps_the_altitude_it_was_read_at() {
    let scratch = Scratch::new("imaging-focus-altitude-record");
    let root = scratch.0.join("imaging");
    let rig = rig(&root, 0.5).await;

    let first = check(&rig).await;

    assert!(first["focus"]["aid"].get("reference").is_none(), "{first}");

    // The record made with Track off: it has tonight's size and no altitude.
    rig.ok("POST", "/api/imaging/focus", json!({})).await;
    rig.wait("/api/imaging", |v| {
        v["focus"]["record"]["tonight"].is_object()
    })
    .await;
    rig.ok("DELETE", "/api/imaging/focus", Value::Null).await;
    let off = rig
        .wait("/api/imaging", |v| v["focus"]["aid"]["state"] == "ended")
        .await;
    assert!(
        off["focus"]["record"]["tonight"].get("alt_deg").is_none(),
        "{off}"
    );

    // Track on, tracking; the focus error to zero; the aid to find its region.
    rig.ok("POST", "/api/track", json!({"on": true})).await;
    rig.wait("/api/session", |v| v["track"]["state"] == "tracking")
        .await;
    rig.sim.set_focus_error(0.0);
    rig.ok("POST", "/api/imaging/focus", json!({"find": true}))
        .await;
    let on = rig
        .wait("/api/imaging", |v| {
            v["focus"]["record"]["tonight"]["alt_deg"].is_number()
        })
        .await;
    let true_alt = rig.sim.telescope().eyepiece.alt;
    let alt = on["focus"]["record"]["tonight"]["alt_deg"]
        .as_f64()
        .unwrap();
    assert!(
        (alt - true_alt).abs() < 0.5,
        "record altitude {alt} against the simulator's {true_alt}"
    );
}

/// The section of `readme` from its heading `start` up to the first line that
/// starts with `end`, each line break read as a space.
fn section(readme: &str, start: &str, end: &str) -> String {
    readme
        .lines()
        .skip_while(|line| *line != start)
        .skip(1)
        .take_while(|line| !line.starts_with(end))
        .collect::<Vec<_>>()
        .join(" ")
}

/// US3-S3: the README says how the record is carried, in `## Focus`, and that the
/// check is judged against the record carried to the target's altitude, in
/// `## An imaging run`.
#[test]
fn the_readme_says_the_check_is_judged_at_the_records_altitude() {
    let readme = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md"))
        .expect("the finder's README must be readable");
    let focus = section(&readme, "## Focus", "## ");
    for words in [
        "grows as the airmass to the power 0.6",
        "judged against the record carried to its own altitude",
        "never by less than 1 or more than 1.30",
        "the far-out limit is about 11″",
    ] {
        assert!(focus.contains(words), "## Focus lacks {words:?}");
    }
    let run = section(&readme, "## An imaging run", "### Field checklist");
    assert!(
        run.contains("against the record carried to the target's altitude"),
        "## An imaging run lacks the carried check"
    );
}
