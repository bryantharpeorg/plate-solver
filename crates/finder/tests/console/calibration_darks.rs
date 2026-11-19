//! US6: oracles are H's declared sensor places, independently read saved FITS
//! files, and uniform scripted samples. Counts and texts are exact; each noisy
//! simulator comparison states its tolerance. Every wait uses paused time.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use finder::{
    imaging::{calibration::service::HOLDER, fits::CardValue as C},
    link::Transport,
    session::Session,
    AppState,
};
use scope_link::memory;
use scope_node::{
    clock::NodeClock,
    imaging::{
        synthetic::SyntheticImagingCamera, Exposure, ImagingCamera, ImagingError, ImagingFrame,
        ImagingFuture, ImagingInfo, ImagingSettings, ImagingStatus,
    },
    service::{NodeControls, NodeService},
    synthetic::SyntheticCamera,
    Settings,
};
use serde_json::{json, Value};
use simulator::{FlatPanel, HotPixel, LightPath, SensorPattern, Simulator, VirtualClock};
use tower::ServiceExt;

use crate::{
    calibration_console::{console_rig, request, sim_rig, wait_until, Rig},
    calibration_library::{read_fits, ReadFits},
    support::{live as support, Scratch},
};

const DARKS: &str = "/api/imaging/calibration/darks";
const CAL: &str = "/api/imaging/calibration";

type Carried = Arc<dyn Fn(usize) -> f64 + Send + Sync>;

/// Shared with US7: functions of exposure number (starting at one), independent
/// of the code taking the set. The stand-in supplies settings, timing and abort.
pub(crate) struct Script {
    pub level: Arc<dyn Fn(usize, f64) -> f64 + Send + Sync>,
    pub gain: Option<Carried>,
    pub temp: Option<Carried>,
    pub exposure: Option<Carried>,
    pub fail: Option<usize>,
}

impl Script {
    pub(crate) fn new(level: impl Fn(usize, f64) -> f64 + Send + Sync + 'static) -> Self {
        Self {
            level: Arc::new(level),
            gain: None,
            temp: None,
            exposure: None,
            fail: None,
        }
    }
}

#[derive(Clone)]
pub(crate) struct ScriptedCamera {
    camera: Arc<SyntheticImagingCamera>,
    script: Arc<Script>,
    calls: Arc<Mutex<Vec<f64>>>,
}

impl ScriptedCamera {
    pub(crate) fn calls(&self) -> Vec<f64> {
        self.calls.lock().unwrap().clone()
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
    fn abort(&self) -> ImagingFuture<'_, ()> {
        self.camera.abort()
    }
    fn expose<'a>(&'a self, exposure: &'a Exposure) -> ImagingFuture<'a, ImagingFrame> {
        Box::pin(async move {
            let n = {
                let mut calls = self.calls.lock().unwrap();
                calls.push(exposure.exposure_ms);
                calls.len()
            };
            let mut frame = self.camera.expose(exposure).await?;
            if self.script.fail == Some(n) {
                return Err(ImagingError::Failed("the camera fell over".into()));
            }
            let sample = (self.script.level)(n, exposure.exposure_ms)
                .round()
                .clamp(0.0, 65535.0) as u16;
            for bytes in frame.samples.as_chunks_mut::<2>().0 {
                bytes.copy_from_slice(&sample.to_le_bytes());
            }
            if let Some(f) = &self.script.gain {
                frame.gain = Some(f(n));
            }
            if let Some(f) = &self.script.temp {
                frame.temp_c = Some(f(n));
            }
            if let Some(f) = &self.script.exposure {
                frame.exposure_ms = f(n);
            }
            Ok(frame)
        })
    }
}

struct NodeTime(VirtualClock);
impl NodeClock for NodeTime {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
}

/// Shared scripted console, served through the same transport as a real node.
pub(crate) fn scripted_rig(root: &Path, script: Script) -> (Rig, ScriptedCamera) {
    scripted_console(Some(root), script, true)
}

fn scripted_console(root: Option<&Path>, script: Script, service: bool) -> (Rig, ScriptedCamera) {
    let sim = sim_rig();
    let clock = Arc::new(NodeTime(sim.clock().clone()));
    let camera = ScriptedCamera {
        camera: Arc::new(SyntheticImagingCamera::new(clock.clone())),
        script: Arc::new(script),
        calls: Arc::new(Mutex::new(Vec::new())),
    };
    let node = NodeService::with_controls(
        SyntheticCamera::new(clock.clone()),
        clock,
        "test-token",
        Settings::default(),
        NodeControls::default(),
    )
    .unwrap()
    .with_imaging(camera.clone());
    node.controls()
        .set_frame_interval(Duration::from_millis(500));
    let transport = Transport::connector(move || {
        let node = node.clone();
        async move {
            let (client, server) = tokio::io::duplex(64 * 1024);
            tokio::spawn(async move {
                if service {
                    memory::serve_routes_on(server, node.routes())
                        .await
                        .unwrap();
                } else {
                    memory::serve_on(server, node.into_server()).await.unwrap();
                }
            });
            Ok::<_, std::io::Error>(client)
        }
    });
    let session = Session::default();
    session
        .edit(
            serde_json::from_value(
                json!({"node":{"address":"scope.local:1","token":"test-token"},
        "site":{"lat_deg":40,"lon_deg":-75}}),
            )
            .unwrap(),
            support::START,
        )
        .unwrap();
    let state = AppState::shared(support::DB.clone(), finder::samples::Samples::default())
        .with_session(session)
        .with_clock(Arc::new(support::ConsoleClock(sim.clock().clone())))
        .with_link(transport)
        .with_live();
    let state = if let Some(root) = root {
        state.with_imaging(root)
    } else {
        state
    };
    (
        Rig {
            app: finder::router(state.clone()),
            link: state.link().unwrap(),
            state,
        },
        camera,
    )
}

async fn get(app: &Router, path: &str) -> Value {
    let (status, body) = request(app, "GET", path, Value::Null, None).await;
    assert_eq!(status, StatusCode::OK);
    body
}
async fn edit(app: &Router, body: Value) {
    let (status, body) = request(app, "PUT", "/api/imaging/settings", body, None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
}
async fn ready(app: &Router) {
    wait_until(app, "/api/imaging", |v| v["state"] == "ready").await;
}
async fn start(app: &Router, body: Value) -> Value {
    let (status, body) = request(app, "POST", DARKS, body, None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}
async fn finish(app: &Router, state: &str) -> Value {
    wait_until(app, CAL, |v| v["set"]["state"] == state).await
}
async fn restored(rig: &Rig, exposure: f64) {
    assert_eq!(rig.state.imaging().unwrap().holder(), None);
    assert_eq!(
        get(&rig.app, "/api/imaging").await["settings"]["exposure_ms"],
        exposure
    );
}
fn near(value: &Value, expected: f64, tolerance: f64) {
    let n = value.as_f64().unwrap();
    assert!(
        (n - expected).abs() <= tolerance,
        "{n} != {expected} ± {tolerance}"
    );
}
fn card(image: &ReadFits, key: &str) -> C {
    image
        .cards
        .iter()
        .find(|(k, _)| k == key)
        .unwrap()
        .1
        .clone()
}
fn read(root: &Path, file: &str) -> ReadFits {
    read_fits(&fs::read(root.join(file)).unwrap())
}
fn files(root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_dir() {
                paths.extend(files(&path));
            } else {
                paths.push(path);
            }
        }
    }
    paths.sort();
    paths
}
fn covered_h(sim: &Simulator) {
    sim.set_light_path(LightPath {
        cover: true,
        ..Default::default()
    });
    sim.set_sensor_pattern(SensorPattern {
        column_amplitude: 0.0,
        hot_pixels: [(100, 50, 4000.0), (101, 51, 2000.0), (600, 400, 40000.0)]
            .into_iter()
            .map(|(x, y, electrons_per_s)| HotPixel {
                x,
                y,
                electrons_per_s,
            })
            .collect(),
    });
}
fn map(root: &Path, id: &str) -> Value {
    serde_json::from_slice(
        &fs::read(root.join(format!("calibration/darks/{id}.badpixels.json"))).unwrap(),
    )
    .unwrap()
}

/// US6-S1: H adds 200, 100, 2000 counts at 200 ms; master is independently
/// compared sample by sample with the arithmetic mean of all four saved darks.
#[tokio::test(start_paused = true)]
async fn covered_darks_keep_frames_and_make_the_mean_and_map() {
    assert_eq!(HOLDER, "a set of darks or flats");
    let scratch = Scratch::new("darks-mean");
    let root = scratch.0.join("imaging");
    let sim = sim_rig();
    covered_h(&sim);
    let rig = console_rig(&sim, &root);
    ready(&rig.app).await;
    edit(&rig.app, json!({"gain":0,"exposure_ms":200})).await;
    assert_eq!(rig.state.imaging().unwrap().holder(), None);
    let answer = start(&rig.app, json!({"count":4})).await;
    assert_eq!(rig.state.imaging().unwrap().holder(), Some(HOLDER));
    let set = &answer["set"];
    let id = set["id"].as_str().unwrap();
    assert_eq!(id.len(), 21);
    assert!(id.starts_with("dark-20260116T02"));
    assert!(id[14..20].bytes().all(|b| b.is_ascii_digit()));
    assert!(id.ends_with('Z'));
    for (key, expected) in [
        ("kind", json!("dark")),
        ("state", json!("offset")),
        ("wanted", json!(4)),
        ("taken", json!(0)),
        ("exposure_ms", json!(200.0)),
    ] {
        assert_eq!(set[key], expected);
    }
    let done = finish(&rig.app, "done").await;
    let set = &done["set"];
    assert_eq!(set["taken"], 4);
    assert_eq!(set["offset_level"], 160.0);
    assert_eq!(set["level"], 160.0);
    assert!(set.get("error").is_none());
    assert!(set.get("not_dark").is_none());
    restored(&rig, 200.0).await;
    assert_eq!(done["masters"].as_array().unwrap().len(), 1);
    let master = &done["masters"][0];
    for (key, expected) in [
        ("id", json!(id)),
        ("kind", json!("dark")),
        ("frames", json!(4)),
        ("checked", json!(true)),
        ("model", json!("simulated imaging camera")),
        ("serial", json!("SIM-0001")),
        ("width", json!(1236)),
        ("height", json!(822)),
        ("origin_x", json!(0)),
        ("origin_y", json!(0)),
        ("bits", json!(16)),
        ("bayer", json!("RGGB")),
        ("gain", json!(0.0)),
        ("offset", json!(10.0)),
        ("binning", json!(1)),
        ("set_temp_c", Value::Null),
        ("temp_c", json!(20.0)),
        ("exposure_ms", json!(200.0)),
        ("bad_pixels", json!(3)),
        ("label", Value::Null),
        ("file", json!(format!("darks/{id}.fits"))),
    ] {
        assert_eq!(master[key], expected, "{key}");
    }
    near(&master["level"], 160.0, 0.5);
    let paths = files(&root.join("frames"));
    assert_eq!(paths.len(), 5);
    let mut sum = vec![0.0f32; 1236 * 822];
    for (i, path) in paths.iter().enumerate() {
        assert_eq!(
            path.strip_prefix(&root).unwrap(),
            Path::new(&format!("frames/2026-01-16/{:06}.fits", i + 1))
        );
        let image = read_fits(&fs::read(path).unwrap());
        assert_eq!(card(&image, "IMAGETYP"), C::Text("DARK".into()));
        assert_eq!(card(&image, "PURPOSE"), C::Text("dark".into()));
        assert_eq!(
            card(&image, "EXPTIME"),
            C::Real(if i == 0 { 0.001 } else { 0.2 })
        );
        if i > 0 {
            for (sum, sample) in sum.iter_mut().zip(image.samples) {
                *sum += sample;
            }
        }
    }
    let image = read(&root, &format!("calibration/darks/{id}.fits"));
    assert_eq!((image.width, image.height, image.bitpix), (1236, 822, -32));
    assert_eq!(
        image.samples,
        sum.into_iter().map(|s| s / 4.0).collect::<Vec<_>>()
    );
    assert_eq!(card(&image, "IMAGETYP"), C::Text("MASTER DARK".into()));
    assert_eq!(card(&image, "NFRAMES"), C::Int(4));
    assert_eq!(card(&image, "DARKTEST"), C::Logical(true));
    assert_eq!(card(&image, "EXPTIME"), C::Real(0.2));
    for (x, y, expected, tolerance) in [
        (100, 50, 360.0, 20.0),
        (101, 51, 260.0, 12.0),
        (600, 400, 2160.0, 60.0),
        (0, 0, 160.0, 2.0),
    ] {
        assert!((image.samples[y * 1236 + x] - expected).abs() <= tolerance);
    }
    assert_eq!(
        map(&root, id)["pixels"],
        json!([[100, 50], [101, 51], [600, 400]])
    );
}

/// US6-S2: the simulator's binned H places are the oracle, not a rebin here.
#[tokio::test(start_paused = true)]
async fn binned_cooled_darks_carry_the_settings_and_map() {
    let scratch = Scratch::new("darks-binned");
    let root = scratch.0.join("imaging");
    let sim = sim_rig();
    covered_h(&sim);
    let rig = console_rig(&sim, &root);
    ready(&rig.app).await;
    edit(
        &rig.app,
        json!({"gain":0,"exposure_ms":200,"binning":2,"cooler_on":true,"set_temp_c":-10}),
    )
    .await;
    start(&rig.app, json!({"count":3})).await;
    let done = finish(&rig.app, "done").await;
    let m = &done["masters"][0];
    for (key, expected) in [
        ("width", json!(618)),
        ("height", json!(411)),
        ("binning", json!(2)),
        ("set_temp_c", json!(-10.0)),
        ("temp_c", json!(-10.0)),
        ("frames", json!(3)),
    ] {
        assert_eq!(m[key], expected);
    }
    let id = m["id"].as_str().unwrap();
    let image = read(&root, &format!("calibration/darks/{id}.fits"));
    assert_eq!((image.width, image.height), (618, 411));
    assert_eq!(card(&image, "XBINNING"), C::Int(2));
    assert_eq!(card(&image, "CCD-TEMP"), C::Real(-10.0));
    assert_eq!(card(&image, "SET-TEMP"), C::Real(-10.0));
    assert_eq!(
        map(&root, id)["pixels"],
        json!([[50, 24], [51, 25], [300, 200]])
    );
}

/// US6-S3: sky adds 20 counts at gain 120/1 s, one at gain 0/0.2 s;
/// the latter is below the level line but its field's touching stars are lit.
#[tokio::test(start_paused = true)]
async fn uncovered_sky_stars_and_panel_each_fail_the_dark_test() {
    for case in 0..3 {
        let scratch = Scratch::new("darks-uncovered");
        let root = scratch.0.join("imaging");
        let sim = sim_rig();
        if case == 2 {
            sim.set_light_path(LightPath {
                flat_panel: Some(FlatPanel {
                    electrons_per_s: 40_000_000.0,
                    colour: [1.0; 3],
                }),
                ..Default::default()
            });
        }
        let rig = console_rig(&sim, &root);
        ready(&rig.app).await;
        if case == 1 {
            edit(&rig.app, json!({"gain":0,"exposure_ms":200})).await;
        }
        start(&rig.app, json!({"count":3})).await;
        let failed = finish(&rig.app, "failed").await;
        let s = &failed["set"];
        assert_eq!(s["not_dark"], true);
        assert_eq!(s["taken"], 0);
        assert_eq!(failed["masters"], json!([]));
        let error = s["error"].as_str().unwrap();
        if case == 2 {
            assert!(error.starts_with("the camera is not covered: its shortest exposure reads "));
            assert!(error.ends_with(" of 65535"));
            let offset = s["offset_level"].as_f64().unwrap();
            assert!((39000.0..=41000.0).contains(&offset));
        } else {
            near(&s["offset_level"], 160.0, 1.0);
            near(
                &s["level"],
                if case == 0 { 180.0 } else { 161.0 },
                if case == 0 { 2.0 } else { 1.0 },
            );
            assert!(error.starts_with(if case == 0 {
                "the darks are not dark: one reads "
            } else {
                "the darks are not dark: one holds "
            }));
            assert!(error.ends_with(if case == 0 {
                "; cover the telescope"
            } else {
                " bright samples that are not hot pixels; cover the telescope"
            }));
        }
        assert_eq!(
            files(&root.join("frames")).len(),
            if case == 2 { 1 } else { 2 }
        );
        restored(&rig, if case == 1 { 200.0 } else { 1000.0 }).await;
    }
}

/// US6-S4: strict inequalities at offset+2 and 20% of 65535=13107.
#[tokio::test(start_paused = true)]
async fn scripted_thresholds_anyway_and_exact_errors() {
    for case in 0..6 {
        let scratch = Scratch::new("darks-threshold");
        let root = scratch.0.join("imaging");
        let script = Script::new(move |n, _| match case {
            0 => {
                if n == 1 {
                    160.0
                } else {
                    162.0
                }
            }
            1 => {
                if n == 1 {
                    160.0
                } else if n == 2 {
                    162.0
                } else {
                    163.0
                }
            }
            2 | 5 => 13108.0,
            3 => {
                if n == 1 {
                    13107.0
                } else {
                    13109.0
                }
            }
            _ => {
                if n == 1 {
                    160.0
                } else {
                    163.0
                }
            }
        });
        let (rig, camera) = scripted_rig(&root, script);
        ready(&rig.app).await;
        start(
            &rig.app,
            if case >= 4 {
                json!({"count":3,"anyway":true})
            } else {
                json!({"count":3})
            },
        )
        .await;
        let failed = case == 1 || case == 2;
        let result = finish(&rig.app, if failed { "failed" } else { "done" }).await;
        let set = &result["set"];
        restored(&rig, 1000.0).await;
        if failed {
            assert_eq!(set["not_dark"], true);
            assert_eq!(result["masters"], json!([]));
            assert_eq!(set["taken"], if case == 1 { 1 } else { 0 });
            assert_eq!(
                set["error"],
                if case == 1 {
                    "the darks are not dark: one reads 163 against an offset level of 160; cover the telescope"
                } else {
                    "the camera is not covered: its shortest exposure reads 13108 of 65535"
                }
            );
            assert_eq!(camera.calls().len(), if case == 1 { 3 } else { 1 });
        } else {
            assert_eq!(camera.calls(), vec![1.0, 1000.0, 1000.0, 1000.0]);
            assert!(set.get("not_dark").is_none());
            let level = if case == 0 {
                162.0
            } else if case == 3 {
                13109.0
            } else if case == 5 {
                13108.0
            } else {
                163.0
            };
            assert_eq!(
                set["offset_level"],
                if case == 3 {
                    13107.0
                } else if case == 5 {
                    13108.0
                } else {
                    160.0
                }
            );
            let m = &result["masters"][0];
            assert_eq!(m["level"], level);
            assert_eq!(m["checked"], case < 4);
            assert_eq!(m["model"], "synthetic imaging camera");
            assert_eq!(m["serial"], Value::Null);
            let image = read(
                &root,
                &format!("calibration/{}", m["file"].as_str().unwrap()),
            );
            assert!(image.samples.iter().all(|&v| v == level as f32));
            assert_eq!(card(&image, "DARKTEST"), C::Logical(case < 4));
        }
    }
}

/// US6-S5: refusal precedence, including absence of a lease for a single frame.
#[tokio::test(start_paused = true)]
async fn route_refusals_do_not_change_the_library_or_set() {
    let scratch = Scratch::new("darks-refusals");
    let root = scratch.0.join("imaging");
    let (rig, _) = scripted_rig(&root, Script::new(|_, _| 160.0));
    ready(&rig.app).await;
    for body in [
        json!({}),
        json!({"count":2}),
        json!({"count":65}),
        json!({"anyway":true}),
    ] {
        let (status, answer) = request(&rig.app, "POST", DARKS, body, None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(
            answer,
            json!({"error":"count must be a whole number from 3 to 64"})
        );
    }
    for body in [
        json!({"count":2.5}),
        json!({"count":"4"}),
        json!({"count":4,"anyway":"yes"}),
        json!({"count":4,"force":true}),
    ] {
        let (status, answer) = request(&rig.app, "POST", DARKS, body, None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(answer["error"].is_string());
    }
    let (status, _) = request(
        &rig.app,
        "POST",
        DARKS,
        json!({"count":4}),
        Some("http://elsewhere.example"),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let response = rig
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(DARKS)
                .header("host", "localhost")
                .header("content-type", "text/plain")
                .body(Body::from("{\"count\":4}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(get(&rig.app, CAL).await, json!({"masters":[]}));
    edit(&rig.app, json!({"exposure_ms":10000})).await;
    assert_eq!(
        request(&rig.app, "POST", "/api/imaging/frame", Value::Null, None)
            .await
            .0,
        StatusCode::OK
    );
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(rig.state.imaging().unwrap().holder(), None);
    let (status, answer) = request(&rig.app, "POST", DARKS, json!({"count":4}), None).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(answer["error"], "an exposure is in progress");
    assert_eq!(get(&rig.app, CAL).await, json!({"masters":[]}));
    rig.state.imaging().unwrap().abort().await;
    rig.state.imaging().unwrap().ended(1).await;
    start(&rig.app, json!({"count":3})).await;
    wait_until(&rig.app, CAL, |v| v["set"]["state"] == "frames").await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    let before = get(&rig.app, CAL).await;
    let (status, answer) = request(&rig.app, "POST", DARKS, json!({"count":3}), None).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        answer["error"],
        "a set of darks or flats is already being taken"
    );
    assert_eq!(get(&rig.app, CAL).await, before);
    let id = before["set"]["id"].as_str().unwrap();
    request(
        &rig.app,
        "DELETE",
        &format!("{CAL}/{id}"),
        Value::Null,
        None,
    )
    .await;
    finish(&rig.app, "stopped").await;

    let (rig, _) = scripted_console(Some(&root), Script::new(|_, _| 160.0), false);
    wait_until(&rig.app, "/api/imaging", |v| v["state"] == "no_service").await;
    let (status, answer) = request(&rig.app, "POST", DARKS, json!({"count":4}), None).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(answer["error"], "this scope node has no imaging service");
    assert_eq!(get(&rig.app, CAL).await, json!({"masters":[]}));
    let (rig, _) = scripted_console(None, Script::new(|_, _| 160.0), true);
    for count in [2, 4] {
        let (status, answer) = request(&rig.app, "POST", DARKS, json!({"count":count}), None).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(answer["error"], "this console takes no imaging frames");
    }
    assert_eq!(
        request(&rig.app, "POST", DARKS, json!({"count":"4"}), None)
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        request(
            &rig.app,
            "POST",
            DARKS,
            json!({"count":4}),
            Some("http://elsewhere.example")
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(get(&rig.app, CAL).await, json!({"masters":[]}));
}

async fn exposing_dark(app: &Router) {
    wait_until(app, "/api/imaging", |v| {
        v["calibration"]["set"]["state"] == "frames" && v["status"]["exposing"] == true
    })
    .await;
}

/// US6-S6: DELETE during a dark, abort during a dark, and DELETE before any
/// offset frame exists. Stop must release the lease before publishing stopped.
#[tokio::test(start_paused = true)]
async fn stopping_during_frames_or_before_the_offset_releases_the_lease() {
    for case in 0..3 {
        let scratch = Scratch::new("darks-stop");
        let root = scratch.0.join("imaging");
        let sim = sim_rig();
        sim.set_light_path(LightPath {
            cover: true,
            ..Default::default()
        });
        let rig = console_rig(&sim, &root);
        ready(&rig.app).await;
        edit(&rig.app, json!({"exposure_ms":10000})).await;
        let answer = start(&rig.app, json!({"count":3})).await;
        let id = answer["set"]["id"].as_str().unwrap();
        let path = format!("{CAL}/{id}");
        if case != 2 {
            exposing_dark(&rig.app).await;
        } else {
            assert_eq!(answer["set"]["state"], "offset");
        }
        assert_eq!(rig.state.imaging().unwrap().holder(), Some(HOLDER));
        let before = tokio::time::Instant::now();
        let (status, _) = request(
            &rig.app,
            if case == 1 { "POST" } else { "DELETE" },
            if case == 1 {
                "/api/imaging/abort"
            } else {
                &path
            },
            Value::Null,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let stopped = finish(&rig.app, "stopped").await;
        assert!(
            before.elapsed() <= Duration::from_secs(2),
            "stop took {:?}",
            before.elapsed()
        );
        assert!(stopped["set"].get("error").is_none());
        assert_eq!(stopped["masters"], json!([]));
        restored(&rig, 10000.0).await;
        if case != 2 {
            assert_eq!(
                get(&rig.app, "/api/imaging").await["frame"]["state"],
                "aborted"
            );
        }
        assert_eq!(
            request(&rig.app, "DELETE", &path, Value::Null, None)
                .await
                .0,
            StatusCode::NOT_FOUND
        );
    }
}

/// US6-S7: the third exposure is the second dark. Frame metadata, rather than
/// the console's requested settings, must catch a camera changing by itself.
#[tokio::test(start_paused = true)]
async fn camera_failure_settings_drift_and_unsettled_temperature_fail() {
    for case in 0..6 {
        let scratch = Scratch::new("darks-camera-errors");
        let root = scratch.0.join("imaging");
        let mut script = Script::new(|_, _| 160.0);
        match case {
            0 => script.fail = Some(3),
            1 => script.gain = Some(Arc::new(|n| if n < 3 { 100.0 } else { 150.0 })),
            2 => {
                script.exposure = Some(Arc::new(|n| {
                    if n == 1 {
                        1.0
                    } else if n == 3 {
                        1500.0
                    } else {
                        1000.0
                    }
                }))
            }
            3 | 4 => script.temp = Some(Arc::new(|_| -7.5)),
            _ => script.temp = Some(Arc::new(|n| -8.8 - 0.2 * n as f64)),
        }
        let (rig, camera) = scripted_rig(&root, script);
        ready(&rig.app).await;
        if case >= 3 {
            edit(&rig.app, json!({"cooler_on":true,"set_temp_c":-10})).await;
        }
        start(
            &rig.app,
            if case == 4 {
                json!({"count":3,"anyway":true})
            } else {
                json!({"count":3})
            },
        )
        .await;
        let result = finish(&rig.app, if case == 5 { "done" } else { "failed" }).await;
        restored(&rig, 1000.0).await;
        let s = &result["set"];
        assert!(s.get("not_dark").is_none());
        if case == 5 {
            let m = &result["masters"][0];
            assert_eq!(m["set_temp_c"], -10.0);
            near(&m["temp_c"], -9.4, 0.001);
        } else {
            assert_eq!(result["masters"], json!([]));
            assert_eq!(s["taken"], if case < 3 { 1 } else { 0 });
            assert_eq!(
                s["error"],
                match case {
                    0 => "a dark failed: the exposure failed: the camera fell over",
                    1 | 2 => "the imaging settings changed during the set",
                    _ => "the sensor is at -7.5 °C, not yet at its set temperature -10.0 °C",
                }
            );
            assert_eq!(camera.calls().len(), if case < 3 { 3 } else { 1 });
        }
    }
}

/// US6-S8: each set consumes four ordinary frame ids; newest matching master
/// wins. The independently declared map has exactly H's three bad pixels.
#[tokio::test(start_paused = true)]
async fn later_imaging_frames_match_the_newest_dark_and_its_map() {
    let scratch = Scratch::new("darks-match");
    let root = scratch.0.join("imaging");
    let sim = sim_rig();
    covered_h(&sim);
    let rig = console_rig(&sim, &root);
    ready(&rig.app).await;
    edit(&rig.app, json!({"gain":0,"exposure_ms":200})).await;
    let mut previous = String::new();
    for n in 1..=2 {
        let answer = start(&rig.app, json!({"count":3})).await;
        let id = answer["set"]["id"].as_str().unwrap();
        assert_ne!(id, previous);
        let done = finish(&rig.app, "done").await;
        assert_eq!(done["masters"].as_array().unwrap().len(), n);
        assert_eq!(
            request(&rig.app, "POST", "/api/imaging/frame", Value::Null, None)
                .await
                .0,
            StatusCode::OK
        );
        let saved = wait_until(&rig.app, "/api/imaging", |v| {
            v["frame"]["id"] == n * 5 && v["frame"]["state"] == "saved"
        })
        .await;
        assert_eq!(
            saved["calibration"]["match"],
            json!({"frame":n*5,"dark":id,"flat":null,"bad_pixels":3,"missing":["flat"]})
        );
        previous = id.to_owned();
    }
}

/// US6-S9: the existing lease owns exclusivity. A foreign holder's exposure
/// must preserve that lease's refusal rather than hide it with frame-in-flight.
#[tokio::test(start_paused = true)]
async fn the_sets_lease_refuses_other_takers_and_honours_a_foreign_holder() {
    let scratch = Scratch::new("darks-lease");
    let root = scratch.0.join("imaging");
    let sim = sim_rig();
    sim.set_light_path(LightPath {
        cover: true,
        ..Default::default()
    });
    let rig = console_rig(&sim, &root);
    ready(&rig.app).await;
    edit(&rig.app, json!({"exposure_ms":10000})).await;
    let answer = start(&rig.app, json!({"count":3})).await;
    exposing_dark(&rig.app).await;
    let imaging = rig.state.imaging().unwrap();
    assert_eq!(imaging.holder(), Some(HOLDER));
    for (method, path, body) in [
        ("POST", "/api/imaging/frame", Value::Null),
        ("PUT", "/api/imaging/settings", json!({"exposure_ms":1000})),
    ] {
        let (status, answer) = request(&rig.app, method, path, body, None).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert!(answer["error"].as_str().unwrap().contains(HOLDER));
    }
    let refusal = imaging.lease("probe").unwrap_err();
    assert_eq!(refusal.status, StatusCode::CONFLICT);
    assert!(refusal.message.contains(HOLDER));
    assert_eq!(
        get(&rig.app, "/api/imaging").await["settings"]["exposure_ms"],
        10000.0
    );
    let path = format!("{CAL}/{}", answer["set"]["id"].as_str().unwrap());
    assert_eq!(
        request(&rig.app, "DELETE", &path, Value::Null, None)
            .await
            .0,
        StatusCode::OK
    );
    finish(&rig.app, "stopped").await;
    assert_eq!(imaging.holder(), None);

    let scratch = Scratch::new("darks-foreign-lease");
    let root = scratch.0.join("imaging");
    let sim = sim_rig();
    sim.set_light_path(LightPath {
        cover: true,
        ..Default::default()
    });
    let rig = console_rig(&sim, &root);
    ready(&rig.app).await;
    edit(&rig.app, json!({"exposure_ms":10000})).await;
    let imaging = rig.state.imaging().unwrap();
    let probe = imaging.lease("probe").unwrap();
    probe.take("probe").await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
    let (status, answer) = request(&rig.app, "POST", DARKS, json!({"count":3}), None).await;
    assert_eq!(status, StatusCode::CONFLICT);
    let error = answer["error"].as_str().unwrap();
    assert!(error.contains("probe"));
    assert_ne!(error, "an exposure is in progress");
    assert!(get(&rig.app, CAL).await.get("set").is_none());
    assert_eq!(
        request(&rig.app, "POST", "/api/imaging/abort", Value::Null, None)
            .await
            .0,
        StatusCode::OK
    );
    wait_until(&rig.app, "/api/imaging", |v| {
        v["frame"]["state"] == "aborted"
    })
    .await;
    drop(probe);
    let answer = start(&rig.app, json!({"count":3})).await;
    assert_eq!(answer["set"]["state"], "offset");
    assert_eq!(imaging.holder(), Some(HOLDER));
    let path = format!("{CAL}/{}", answer["set"]["id"].as_str().unwrap());
    request(&rig.app, "DELETE", &path, Value::Null, None).await;
    finish(&rig.app, "stopped").await;
}
