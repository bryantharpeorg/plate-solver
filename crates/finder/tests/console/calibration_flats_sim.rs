//! US10: independent FITS reads, the simulator's declared V and D, and its
//! true imaging angle are the oracles. Each comparison states its tolerance;
//! every wait uses paused time.

use std::{fs, sync::Arc, time::Duration};

use axum::{http::StatusCode, Router};
use finder::imaging::{
    calibration::{library::FrameKey, Mosaic},
    fits::{self, CardValue as C},
    simulated::SimulatedSolver,
};
use serde_json::{json, Value};
use simulator::{
    DustShadow, FlatPanel, HotPixel, ImagingOptics, LightPath, SensorPattern, Simulator,
};

use crate::{
    calibration_console::{console_rig, request, sim_rig, wait_until},
    calibration_flats::{edit, near, post, read_master, settle, CAL, DARKS, FLATS},
    calibration_library::{read_fits, ReadFits},
    support::Scratch,
};

fn panel() -> FlatPanel {
    FlatPanel {
        electrons_per_s: 1_000_000.0,
        colour: [0.8, 1.0, 0.6],
    }
}

fn patterned_sim() -> Simulator {
    let sim = sim_rig();
    sim.set_sensor_pattern(SensorPattern {
        hot_pixels: [(100, 50, 4000.0), (101, 51, 2000.0), (600, 400, 40000.0)]
            .map(|(x, y, electrons_per_s)| HotPixel {
                x,
                y,
                electrons_per_s,
            })
            .to_vec(),
        column_amplitude: 14.0,
    });
    sim.set_light_path(LightPath {
        flat_panel: Some(panel()),
        corner_light: 0.6,
        dust: vec![DustShadow {
            x: 300.0,
            y: 200.0,
            radius_px: 10.0,
            depth: 0.3,
        }],
        ..Default::default()
    });
    sim
}

fn cover(sim: &Simulator, covered: bool) {
    sim.set_light_path(LightPath {
        cover: covered,
        flat_panel: None,
        ..sim.light_path()
    });
}

/// The specification's formulas evaluated here, independently of the renderer.
fn transmission(width: u32, height: u32, x: u32, y: u32) -> f64 {
    let (x, y) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
    let (hw, hh) = (f64::from(width) / 2.0, f64::from(height) / 2.0);
    let v = 1.0 - 0.4 * ((x - hw).powi(2) + (y - hh).powi(2)) / (hw * hw + hh * hh);
    let d = if (x - 300.0).powi(2) + (y - 200.0).powi(2) <= 100.0 {
        0.7
    } else {
        1.0
    };
    v * d
}

fn colour(x: u32, y: u32) -> usize {
    match (x % 2, y % 2) {
        (0, 0) => 0,
        (1, 1) => 2,
        _ => 1,
    }
}

fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    values[(values.len() - 1) / 2]
}

async fn ready(app: &Router) {
    wait_until(app, "/api/imaging", |v| v["state"] == "ready").await;
}

async fn set_ended(app: &Router, expected: &str) -> Value {
    let view = settle(app).await;
    assert_eq!(view["set"]["state"], expected, "{view}");
    view
}

async fn flat_master(app: &Router, sim: &Simulator, count: u32) -> Value {
    post(app, FLATS, json!({"count":count,"label":"dob"})).await;
    set_ended(app, "needs_darks").await;
    cover(sim, true);
    post(app, FLATS, json!({"darks":true})).await;
    let done = set_ended(app, "done").await;
    done["masters"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["kind"] == "flat")
        .unwrap()
        .clone()
}

async fn saved(app: &Router) -> Value {
    let answer = post(app, "/api/imaging/frame", Value::Null).await;
    let id = &answer["frame"]["id"];
    wait_until(app, "/api/imaging", |v| {
        v["frame"]["id"] == *id && v["frame"]["state"] == "saved"
    })
    .await
}

async fn solved(app: &Router) -> Value {
    let view = saved(app).await;
    let id = &view["frame"]["id"];
    post(app, "/api/imaging/solve", Value::Null).await;
    let view = wait_until(app, "/api/imaging", |v| {
        v["solve"]["frame_id"] == *id && v["solve"]["state"] != "solving"
    })
    .await;
    assert_eq!(view["solve"]["state"], "solved", "{view}");
    view
}

async fn clear_aim(app: &Router) -> Value {
    let (status, answer) = request(app, "DELETE", "/api/aim", Value::Null, None).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let (status, view) = request(app, "GET", "/api/imaging", Value::Null, None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(view["relation"].get("angle_deg").is_none());
    view
}

/// US10-S1: the flat is V·D / median(V·D) for each colour, after its
/// own dark removes the offset, fixed column pattern and hot-pixel signal.
#[tokio::test(start_paused = true)]
async fn simulated_flat_matches_vignetting_and_dust_including_hot_pixels() {
    let scratch = Scratch::new("flats-sim-formula");
    let root = scratch.0.join("imaging");
    let sim = patterned_sim();
    let rig = console_rig(&sim, &root);
    ready(&rig.app).await;
    edit(&rig.app, json!({"gain":0,"exposure_ms":100})).await;
    let entry = flat_master(&rig.app, &sim, 4).await;
    near(&entry["exposure_ms"], 132.9, 0.5);
    near(&entry["level_share"], 0.45, 0.005);
    assert!(entry["imaging_angle_deg"].is_null());
    let master = read_master(&root, &entry);
    assert_eq!(
        (master.width, master.height, master.bitpix),
        (1236, 822, -32)
    );
    assert!(!master.cards.iter().any(|(k, _)| k == "IMGANGLE"));
    let mut medians = [0.0; 3];
    for (c, m) in medians.iter_mut().enumerate() {
        let mut truth = Vec::new();
        let mut actual = Vec::new();
        for y in 0..master.height {
            for x in 0..master.width {
                if colour(x, y) == c {
                    truth.push(transmission(master.width, master.height, x, y));
                    actual.push(f64::from(master.samples[(y * master.width + x) as usize]));
                }
            }
        }
        *m = median(truth);
        near(&json!(median(actual)), 1.0, 0.002);
    }
    for (x, y, tolerance) in [
        (619, 410, 0.01),
        (1, 0, 0.01),
        (301, 200, 0.01),
        (901, 600, 0.01),
        (618, 410, 0.01),
        (0, 0, 0.01),
        (619, 411, 0.01),
        (1235, 821, 0.01),
        (100, 50, 0.02),
        (600, 400, 0.02),
    ] {
        let actual =
            f64::from(master.samples[(y * master.width + x) as usize]) * medians[colour(x, y)];
        let expected = transmission(master.width, master.height, x, y);
        assert!(
            (actual - expected).abs() <= tolerance,
            "({x}, {y}): {actual} != {expected}"
        );
    }
}

/// US10-S2: 032's measured angle, rather than the simulator's true angle,
/// must survive in the index and FITS file and drive the later turn warning.
#[tokio::test(start_paused = true)]
async fn measured_angle_is_kept_in_the_master_and_warns_after_a_turn() {
    let scratch = Scratch::new("flats-sim-angle");
    let root = scratch.0.join("imaging");
    let sim = sim_rig();
    let rig = console_rig(&sim, &root);
    let state = rig
        .state
        .clone()
        .with_imaging_solver(Arc::new(SimulatedSolver::new(sim.clone())));
    let app = finder::router(state);
    ready(&app).await;
    edit(&app, json!({"gain":0})).await;
    wait_until(&app, "/api/session", |v| v["pointing"]["source"] == "live").await;
    tokio::time::sleep(Duration::from_secs(5)).await;
    let first = solved(&app).await;
    let a0 = first["relation"]["angle_deg"].as_f64().unwrap();
    near(&json!(a0), 17.0, 0.25);
    sim.set_light_path(LightPath {
        flat_panel: Some(FlatPanel {
            colour: [1.0; 3],
            ..panel()
        }),
        ..Default::default()
    });
    post(&app, FLATS, json!({"count":3,"label":"dob"})).await;
    set_ended(&app, "needs_darks").await;
    // The angle belongs to the flats in hand, even if the observer clears
    // the measured aim before their darks are taken.
    clear_aim(&app).await;
    cover(&sim, true);
    post(&app, FLATS, json!({"darks":true})).await;
    let done = set_ended(&app, "done").await;
    let entry = &done["masters"][0];
    near(&entry["exposure_ms"], 125.0, 0.001);
    assert!(entry["imaging_angle_deg"].is_number(), "{entry}");
    near(&entry["imaging_angle_deg"], a0, 1e-9);
    let master = read_master(&root, entry);
    let card = &master
        .cards
        .iter()
        .find(|(k, _)| k == "IMGANGLE")
        .expect("IMGANGLE card")
        .1;
    let C::Real(angle) = card else {
        panic!("IMGANGLE is not real: {card:?}")
    };
    near(&json!(angle), a0, 1e-6);
    let index: Value =
        serde_json::from_slice(&fs::read(root.join("calibration/index.json")).unwrap()).unwrap();
    near(&index["masters"][0]["imaging_angle_deg"], a0, 1e-9);

    sim.set_light_path(LightPath::default());
    let view = saved(&app).await;
    assert_eq!(view["calibration"]["match"]["flat"], entry["id"]);
    assert!(view["calibration"]["match"]
        .get("flat_turned_deg")
        .is_none());
    let view = solved(&app).await;
    near(&view["relation"]["angle_deg"], a0, 0.25);
    assert!(view["calibration"]["match"]
        .get("flat_turned_deg")
        .is_none());
    sim.set_imaging_optics(ImagingOptics {
        angle_deg: 19.5,
        ..sim.imaging_optics()
    });
    let mut view = Value::Null;
    for _ in 0..3 {
        view = solved(&app).await;
    }
    let a1 = view["relation"]["angle_deg"].as_f64().unwrap();
    near(&json!(a1), 19.5, 0.25);
    let matched = &view["calibration"]["match"];
    assert_eq!(matched["flat"], entry["id"]);
    near(&matched["flat_turned_deg"], a1 - a0, 1e-6);
    near(&matched["flat_turned_deg"], 2.5, 0.5);
    let view = clear_aim(&app).await;
    assert_eq!(view["calibration"]["match"]["flat"], entry["id"]);
    assert!(view["calibration"]["match"]
        .get("flat_turned_deg")
        .is_none());

    // Conversely, an angle measured only after the last flat must not be
    // retroactively written into a master whose flats had no angle.
    sim.set_light_path(LightPath {
        flat_panel: Some(FlatPanel {
            colour: [1.0; 3],
            ..panel()
        }),
        ..Default::default()
    });
    post(&app, FLATS, json!({"count":3,"label":"unknown"})).await;
    set_ended(&app, "needs_darks").await;
    sim.set_light_path(LightPath::default());
    let view = solved(&app).await;
    assert!(view["relation"]["angle_deg"].is_number());
    cover(&sim, true);
    post(&app, FLATS, json!({"darks":true})).await;
    let done = set_ended(&app, "done").await;
    let unknown = done["masters"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["label"] == "unknown")
        .unwrap();
    assert!(unknown["imaging_angle_deg"].is_null());
    assert!(!read_master(&root, unknown)
        .cards
        .iter()
        .any(|(k, _)| k == "IMGANGLE"));
}

fn block_median(image: &ReadFits, samples: &[f32], x: u32, y: u32) -> f64 {
    let mut block = Vec::new();
    for row in y..y + 100 {
        for col in x..x + 100 {
            block.push(f64::from(samples[(row * image.width + col) as usize]));
        }
    }
    median(block)
}

/// US10-S3: the simulator's 10s sky contributes 50 counts before V·D.
/// Calibrating with masters made by the console must leave 50·median(V·D).
#[tokio::test(start_paused = true)]
async fn console_masters_make_the_simulated_sky_background_level() {
    let scratch = Scratch::new("flats-sim-sky");
    let root = scratch.0.join("imaging");
    let sim = patterned_sim();
    cover(&sim, true);
    let rig = console_rig(&sim, &root);
    ready(&rig.app).await;
    edit(&rig.app, json!({"gain":0,"exposure_ms":10000})).await;
    post(&rig.app, DARKS, json!({"count":3})).await;
    let darks = set_ended(&rig.app, "done").await;
    let dark_id = darks["masters"][0]["id"].as_str().unwrap();
    sim.set_light_path(LightPath {
        cover: false,
        flat_panel: Some(panel()),
        ..sim.light_path()
    });
    edit(&rig.app, json!({"exposure_ms":100})).await;
    let flat = flat_master(&rig.app, &sim, 3).await;
    cover(&sim, false);
    edit(&rig.app, json!({"exposure_ms":10000})).await;
    let view = saved(&rig.app).await;
    let bytes = fs::read(root.join(view["frame"]["file"].as_str().unwrap())).unwrap();
    let raw = read_fits(&bytes);
    let key = FrameKey::from_image(&fits::decode(&bytes).unwrap()).unwrap();
    let mut sub = Mosaic {
        width: raw.width,
        height: raw.height,
        bayer: Some("RGGB".into()),
        samples: raw.samples.clone(),
    };
    let calibration = rig.state.calibration().unwrap();
    let applied = tokio::task::spawn_blocking(move || {
        let applied = calibration.library().calibrate(&key, &mut sub, None, None);
        (applied, sub)
    })
    .await
    .unwrap();
    let (applied, sub) = applied;
    assert_eq!(applied.selection.dark.as_deref(), Some(dark_id));
    assert_eq!(applied.selection.flat.as_deref(), flat["id"].as_str());
    assert_eq!(applied.selection.bad_pixels, Some(3));
    assert!(applied.selection.missing.is_empty());
    assert!(applied.problems.is_empty(), "{:?}", applied.problems);
    assert_eq!(
        (
            applied.report.dark,
            applied.report.flat,
            applied.report.bad_pixels
        ),
        ("applied", "applied", "map")
    );
    let mut truth = Vec::new();
    for y in 0..raw.height {
        for x in 0..raw.width {
            truth.push(transmission(raw.width, raw.height, x, y));
        }
    }
    let expected = 50.0 * median(truth);
    for (x, y) in [(0, 0), (568, 361)] {
        near(
            &json!(block_median(&raw, &sub.samples, x, y)),
            expected,
            3.0,
        );
    }
    assert!(
        (block_median(&raw, &raw.samples, 0, 0) - block_median(&raw, &raw.samples, 568, 361)).abs()
            > 12.0
    );
    let (_, calibration) = request(&rig.app, "GET", CAL, Value::Null, None).await;
    assert_eq!(calibration["match"]["dark"], dark_id);
    assert_eq!(calibration["match"]["flat"], flat["id"]);
    assert_eq!(calibration["match"]["bad_pixels"], 3);
    assert_eq!(calibration["match"]["missing"], json!([]));
}
