//! US8: API and disk contract. Oracles: the synthetic sample 416+x+3y,
//! standing 2.16 arcsec/pixel solve at 30 degrees, scripted outcomes and the
//! FITS specification's byte layout. Counts/bytes are exact; WCS uses 1e-12.

use crate::{
    stack_camera::ScriptedCamera,
    stack_gate::{CameraScripts, GateScripts},
    stack_loop::{cards, stand_in_rig, Rig, Scripted, ScriptedSolver},
    support::live as support,
};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use finder::{
    imaging::{
        fits::{encode_cube, CardValue, RealCube},
        solver::SolveError,
        stack::service::label,
    },
    AppState,
};
use scope_node::clock::NodeClock;
use serde_json::{json, Value};
use simulator::VirtualClock;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tower::ServiceExt;

const ROUTE: &str = "/api/imaging/stack";

async fn request(
    app: &Router,
    method: &str,
    path: &str,
    body: &str,
    origin: Option<&str>,
    content_type: &str,
) -> (StatusCode, String, Vec<u8>) {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("host", "localhost")
        .header("content-type", content_type);
    if let Some(origin) = origin {
        builder = builder.header("origin", origin);
    }
    let response = app
        .clone()
        .oneshot(builder.body(Body::from(body.to_owned())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let content_type = response
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    (
        status,
        content_type,
        to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec(),
    )
}
async fn call(app: &Router, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
    let (status, _, bytes) = request(
        app,
        method,
        path,
        &body.to_string(),
        None,
        "application/json",
    )
    .await;
    (status, serde_json::from_slice(&bytes).unwrap())
}
async fn ok(app: &Router, method: &str, path: &str, body: Value) -> Value {
    let (status, value) = call(app, method, path, body).await;
    assert_eq!(status, StatusCode::OK, "{value}");
    value
}
async fn wait(rig: &Rig, done: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..3000 {
        let view = ok(&rig.app, "GET", "/api/imaging", Value::Null).await;
        if done(&view["stack"]) {
            return view["stack"].clone();
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!(
        "stack never reached awaited state: {:?}",
        rig.state.stack().unwrap().view()
    );
}
fn folder(rig: &Rig, stack: &Value) -> PathBuf {
    rig.root.join("runs").join(stack["id"].as_str().unwrap())
}
fn manifest(folder: &Path) -> Value {
    serde_json::from_slice(&fs::read(folder.join("stack.json")).unwrap()).unwrap()
}
fn real_at(bytes: &[u8], plane: u32, x: u32, y: u32) -> f32 {
    let header = cards(bytes);
    let number = |name| {
        header
            .iter()
            .find(|(key, _)| key == name)
            .unwrap()
            .1
            .parse::<u32>()
            .unwrap()
    };
    let end = bytes
        .as_chunks::<80>()
        .0
        .iter()
        .position(|c| &c[..8] == b"END     ")
        .unwrap();
    let start = ((end + 1) * 80).div_ceil(2880) * 2880;
    let offset = start
        + ((plane * number("NAXIS1") * number("NAXIS2") + y * number("NAXIS1") + x) * 4) as usize;
    f32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap())
}
fn close(value: f64, expected: f64, tolerance: f64) {
    assert!(
        (value - expected).abs() <= tolerance,
        "{value} != {expected}"
    );
}
fn assert_subs(rig: &Rig, stack: &Value) -> Value {
    let folder = folder(rig, stack);
    let doc = manifest(&folder);
    for key in [
        "id",
        "taken",
        "stacked",
        "dropped",
        "options",
        "ended_reason",
    ] {
        assert_eq!(doc[key], stack[key], "{key}");
    }
    let subs = doc["subs"].as_array().unwrap();
    assert_eq!(subs.len() as u64, stack["taken"].as_u64().unwrap());
    assert!(subs
        .windows(2)
        .all(|w| w[0]["frame_id"].as_u64() < w[1]["frame_id"].as_u64()));
    let count = |reason: &str| subs.iter().filter(|s| s["outcome"] == reason).count() as u64;
    assert_eq!(count("stacked"), stack["stacked"].as_u64().unwrap());
    for (reason, number) in stack["dropped"].as_object().unwrap() {
        assert_eq!(count(reason), number.as_u64().unwrap(), "{reason}");
    }
    for sub in subs {
        if sub["outcome"] == "frame_failed" {
            assert!(sub["file"].is_null());
        } else {
            assert_eq!(
                sub["file"],
                format!("subs/{:06}.fits", sub["frame_id"].as_u64().unwrap())
            );
            assert!(folder.join(sub["file"].as_str().unwrap()).is_file());
        }
        if sub["outcome"] == "stacked" {
            assert!(sub["turn_deg"].is_number());
            let shift = sub["shift_px"].as_array().unwrap();
            assert_eq!(shift.len(), 2);
            assert!(shift.iter().all(Value::is_number));
            assert!(sub["rejected_px"].is_u64());
        }
        if sub["outcome"] == "stacked" || sub["outcome"] == "soft" {
            assert!(sub["focus_arcsec"].is_number());
        }
    }
    doc
}

/// US8-S1: cards read independently, raw big-endian pixels, decoded PNG and
/// every sub's outcome. No display function supplies an expected value.
#[tokio::test(start_paused = true)]
async fn start_stop_and_keep_three_files() {
    let rig = stand_in_rig("stack-console-files", ScriptedSolver::new(), None).await;
    let view = ok(&rig.app, "POST", ROUTE, json!({})).await;
    assert_eq!(view["state"], "ready");
    assert_eq!(view["stack"]["state"], "stacking");
    assert_eq!(
        view["stack"]["options"],
        json!({"reject":true,"focus_gate":true,"gains":[1.0,1.0,1.0],"stretch":10.0})
    );
    let unchanged = ok(&rig.app, "POST", ROUTE, json!({})).await;
    assert_eq!(unchanged["stack"]["id"], view["stack"]["id"]);
    assert_eq!(unchanged["stack"]["options"], view["stack"]["options"]);
    let id = view["stack"]["id"].as_str().unwrap();
    let folder = folder(&rig, &view["stack"]);
    let initial = manifest(&folder);
    assert_eq!(initial["format"], "finder-stack/1");
    assert_eq!(initial["finder"], env!("CARGO_PKG_VERSION"));
    assert_eq!(initial["id"], id);
    let utc = initial["started_utc"].as_str().unwrap();
    assert_eq!(utc.len(), 23);
    assert_eq!(&utc[4..5], "-");
    assert_eq!(&utc[10..11], "T");
    assert_eq!(
        initial["camera"],
        json!({"model":"synthetic imaging camera", "serial":null})
    );
    assert!(initial.get("ended_at").is_none());
    wait(&rig, |s| s["stacked"].as_u64().unwrap() >= 3).await;
    let ended = ok(&rig.app, "DELETE", ROUTE, Value::Null).await;
    assert_eq!(ended["state"], "ready");
    let stack = &ended["stack"];
    assert_eq!(stack["state"], "ended");
    assert_eq!(stack["ended_reason"], "stopped");
    assert_eq!(
        stack["files"],
        json!({"fits":format!("runs/{id}/stack.fits"), "png":format!("runs/{id}/stack.png"),"json":format!("runs/{id}/stack.json")})
    );
    for dir in [&folder, &folder.join("subs")] {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            assert!(!path.to_string_lossy().ends_with(".part"));
            if path.is_file() {
                assert!(!fs::read(path)
                    .unwrap()
                    .windows(10)
                    .any(|w| w == b"test-token"));
            }
        }
    }
    let bytes = fs::read(folder.join("stack.fits")).unwrap();
    assert_eq!(bytes.len(), 40320);
    let header = cards(&bytes);
    let expected = [
        "SIMPLE", "BITPIX", "NAXIS", "NAXIS1", "NAXIS2", "NAXIS3", "IMAGETYP", "STACKID", "NSUBS",
        "EXPTIME", "SUBEXP", "DATE-OBS", "INSTRUME", "XBINNING", "YBINNING", "ROWORDER", "CTYPE1",
        "CTYPE2", "CRPIX1", "CRPIX2", "CRVAL1", "CRVAL2", "CD1_1", "CD1_2", "CD2_1", "CD2_2",
    ];
    assert_eq!(
        header.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(),
        expected
    );
    assert_eq!(
        &bytes[expected.len() * 80..expected.len() * 80 + 8],
        b"END     "
    );
    let value = |name| header.iter().find(|(k, _)| k == name).unwrap().1.as_str();
    for (key, want) in [
        ("SIMPLE", "T"),
        ("BITPIX", "-32"),
        ("NAXIS", "3"),
        ("NAXIS1", "64"),
        ("NAXIS2", "48"),
        ("NAXIS3", "3"),
        ("IMAGETYP", "STACK"),
        ("STACKID", id),
        ("INSTRUME", "synthetic imaging camera"),
        ("XBINNING", "1"),
        ("YBINNING", "1"),
        ("ROWORDER", "TOP-DOWN"),
        ("CTYPE1", "RA---TAN"),
        ("CTYPE2", "DEC--TAN"),
    ] {
        assert_eq!(value(key), want);
    }
    for key in ["NSUBS", "EXPTIME"] {
        close(
            value(key).parse().unwrap(),
            stack["stacked"].as_f64().unwrap(),
            0.0,
        );
    }
    close(value("SUBEXP").parse().unwrap(), 1.0, 0.0);
    assert_eq!(value("DATE-OBS").len(), 23);
    let reference_cards = cards(&fs::read(folder.join("subs/000001.fits")).unwrap());
    assert_eq!(
        value("DATE-OBS"),
        reference_cards
            .iter()
            .find(|(k, _)| k == "DATE-OBS")
            .unwrap()
            .1
    );
    for (key, want) in [
        ("CRPIX1", 32.5),
        ("CRPIX2", 24.5),
        ("CRVAL1", 83.8221),
        ("CRVAL2", -5.3911),
        ("CD1_1", -0.0005196152423),
        ("CD1_2", -0.0003),
        ("CD2_1", 0.0003),
        ("CD2_2", -0.0005196152423),
    ] {
        close(value(key).parse().unwrap(), want, 1e-12);
    }
    assert_eq!(real_at(&bytes, 0, 2, 2), 424.0);
    assert_eq!(real_at(&bytes, 1, 3, 2), 425.0);
    assert_eq!(real_at(&bytes, 2, 3, 3), 428.0);
    let png = image::load_from_memory(&fs::read(folder.join("stack.png")).unwrap()).unwrap();
    assert_eq!(
        (png.width(), png.height(), png.color()),
        (64, 48, image::ColorType::Rgb8)
    );
    let doc = assert_subs(&rig, stack);
    assert_eq!(doc["format"], "finder-stack/1");
    assert!(doc["target"].is_null());
    assert_eq!(doc["reference"]["frame_id"], 1);
    assert_eq!(doc["reference"]["file"], "subs/000001.fits");
    for (row, want) in doc["reference"]["cd_deg"]
        .as_array()
        .unwrap()
        .iter()
        .zip([[-0.0005196152, -0.0003], [0.0003, -0.0005196152]])
    {
        for (n, w) in row.as_array().unwrap().iter().zip(want) {
            close(n.as_f64().unwrap(), w, 1e-10);
        }
    }
}

struct NodeTime(VirtualClock);
impl NodeClock for NodeTime {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
}

#[tokio::test(start_paused = true)]
async fn every_dropped_sub_is_listed_and_counts_match_fits() {
    let slow = stand_in_rig(
        "stack-console-behind",
        ScriptedSolver::new().delay(Duration::from_secs(3)),
        None,
    )
    .await;
    ok(&slow.app, "POST", ROUTE, json!({})).await;
    wait(&slow, |s| s["taken"].as_u64().unwrap() >= 12).await;
    let view = ok(&slow.app, "DELETE", ROUTE, Value::Null).await;
    let stack = &view["stack"];
    assert!(stack["dropped"]["behind"].as_u64().unwrap() >= 1);
    assert!(stack["dropped"]["unprocessed"].as_u64().unwrap() >= 1);
    assert!(stack["taken"].as_u64() > stack["stacked"].as_u64());
    assert_subs(&slow, stack);
    let header = cards(&fs::read(folder(&slow, stack).join("stack.fits")).unwrap());
    assert_eq!(
        header
            .iter()
            .find(|(k, _)| k == "NSUBS")
            .unwrap()
            .1
            .parse::<u64>()
            .unwrap(),
        stack["stacked"].as_u64().unwrap()
    );
    let camera = ScriptedCamera::new(Arc::new(NodeTime(VirtualClock::new(support::START))))
        .fail_at(3, "the camera fell over")
        .gain_at(5, 50.0);
    let rig = stand_in_rig(
        "stack-console-outcomes",
        ScriptedSolver::new()
            .solve_at(3, Scripted::Fail(SolveError::NotSolved { detail: None }))
            .measure_at(7, Ok(9.0)),
        Some(Box::new(camera)),
    )
    .await;
    ok(&rig.app, "POST", ROUTE, json!({})).await;
    wait(&rig, |s| {
        s["taken"].as_u64().unwrap() >= 12 && s["queued"] == 0
    })
    .await;
    let view = ok(&rig.app, "DELETE", ROUTE, Value::Null).await;
    let stack = &view["stack"];
    for reason in ["frame_failed", "settings", "not_solved"] {
        assert_eq!(stack["dropped"][reason], 1);
    }
    assert!(stack["dropped"]["soft"].as_u64().unwrap() >= 1);
    assert_subs(&rig, stack);
}

#[tokio::test(start_paused = true)]
async fn celestial_precision_and_colour_options_reach_saved_files() {
    let rig = stand_in_rig(
        "stack-console-precision",
        ScriptedSolver::new().solve_at(
            1,
            Scripted::Solve {
                ra_deg: 83.822123456789,
                dec_deg: -5.391123456789,
                up_deg: 30.0,
                arcsec_per_px: 2.16,
                mirrored: false,
            },
        ),
        None,
    )
    .await;
    ok(&rig.app, "POST", ROUTE, json!({})).await;
    wait(&rig, |s| s["stacked"].as_u64().unwrap() >= 3).await;
    let view = ok(&rig.app, "DELETE", ROUTE, Value::Null).await;
    let header = cards(&fs::read(folder(&rig, &view["stack"]).join("stack.fits")).unwrap());
    for (key, want) in [("CRVAL1", 83.822123457), ("CRVAL2", -5.391123457)] {
        close(
            header
                .iter()
                .find(|(k, _)| k == key)
                .unwrap()
                .1
                .parse()
                .unwrap(),
            want,
            1e-12,
        );
    }
    let rig = stand_in_rig("stack-console-colour", ScriptedSolver::new(), None).await;
    let mut pictures = Vec::new();
    for body in [json!({}), json!({"gains":[1,1,0.1]}), json!({"stretch":1})] {
        ok(&rig.app, "POST", ROUTE, body).await;
        wait(&rig, |s| s["stacked"].as_u64().unwrap() >= 3).await;
        let view = ok(&rig.app, "DELETE", ROUTE, Value::Null).await;
        pictures.push(
            image::load_from_memory(
                &fs::read(folder(&rig, &view["stack"]).join("stack.png")).unwrap(),
            )
            .unwrap()
            .to_rgb8(),
        );
    }
    assert!(pictures[0]
        .pixels()
        .zip(pictures[1].pixels())
        .any(|(a, b)| a[2] != b[2]));
    assert_ne!(pictures[0], pictures[2]);
}

/// US8-S2: origin precedes body, body precedes availability. Error wording
/// for body shape is axum's; only its JSON shape and status are contractual.
#[tokio::test(start_paused = true)]
async fn request_refusals_and_preview() {
    let rig = stand_in_rig("stack-console-refusals", ScriptedSolver::new(), None).await;
    for (method, body, origin, kind, want) in [
        (
            "POST",
            "{}",
            Some("http://foreign.test"),
            "application/json",
            403,
        ),
        ("POST", r#"{"reject":"yes"}"#, None, "application/json", 400),
        ("POST", r#"{"colour":true}"#, None, "application/json", 400),
        ("POST", r#"{"gains":[1,2]}"#, None, "application/json", 400),
        ("POST", "{}", None, "text/plain", 415),
        (
            "POST",
            r#"{"gains":[0,1,1]}"#,
            None,
            "application/json",
            400,
        ),
        (
            "DELETE",
            "",
            Some("http://foreign.test"),
            "application/json",
            403,
        ),
    ] {
        let (status, _, bytes) = request(&rig.app, method, ROUTE, body, origin, kind).await;
        assert_eq!(status.as_u16(), want);
        let error: Value = serde_json::from_slice(&bytes).unwrap();
        assert!(error["error"].is_string());
        if body.contains("[0,1,1]") {
            assert_eq!(error["error"], "gains must be three numbers from 0.1 to 10");
        }
        assert!(rig.state.stack().unwrap().view().is_none());
    }
    for body in [
        json!([true, true, [1, 1, 1], 10]),
        json!([]),
        Value::Null,
        json!({"reject":null}),
        json!({"focus_gate":null}),
        json!({"stretch":"10"}),
    ] {
        let (status, error) = call(&rig.app, "POST", ROUTE, body).await;
        assert_eq!(status, 400, "{error}");
        assert!(error["error"].is_string());
        assert!(rig.state.stack().unwrap().view().is_none());
    }
    assert!(ok(&rig.app, "DELETE", ROUTE, Value::Null)
        .await
        .get("stack")
        .is_none());
    let path = "/api/imaging/stack/preview.jpg?anything=ignored";
    let (status, error) = call(&rig.app, "GET", path, Value::Null).await;
    assert_eq!(status, 404);
    assert_eq!(error["error"], "the console holds no stack picture");
    ok(&rig.app, "POST", ROUTE, json!({})).await;
    wait(&rig, |s| s.get("preview").is_some()).await;
    let (status, kind, bytes) = request(&rig.app, "GET", path, "", None, "application/json").await;
    assert_eq!(status, 200);
    assert_eq!(kind, "image/jpeg");
    assert!(image::load_from_memory(&bytes).is_ok());
    ok(&rig.app, "DELETE", ROUTE, Value::Null).await;
    let off = finder::router(AppState::shared(
        support::DB.clone(),
        finder::samples::Samples::default(),
    ));
    for (method, body, want) in [
        ("POST", json!({}), 409),
        ("DELETE", Value::Null, 409),
        ("POST", json!({"reject":"yes"}), 400),
    ] {
        let (status, value) = call(&off, method, ROUTE, body).await;
        assert_eq!(status.as_u16(), want);
        assert!(value["error"].is_string());
        if want == 409 {
            assert_eq!(value["error"], "this console takes no imaging frames");
        }
    }
    assert_eq!(call(&off, "GET", path, Value::Null).await.0, 404);
}

/// US8-S3/S4: target names, empty save and the stack's lifetime in both views.
#[tokio::test(start_paused = true)]
async fn labels_empty_stack_and_views() {
    for (name, want) in [
        ("NGC 1976", "NGC-1976"),
        ("M42", "M42"),
        ("Barnard’s Loop", "Barnard-s-Loop"),
        ("Nébuleuse", "N-buleuse"),
    ] {
        assert_eq!(label(name), want);
    }
    let rig = stand_in_rig("stack-console-views", ScriptedSolver::new(), None).await;
    for path in ["/api/imaging", "/api/session"] {
        let v = ok(&rig.app, "GET", path, Value::Null).await;
        let imaging = if path.ends_with("session") {
            &v["imaging"]
        } else {
            &v
        };
        assert!(imaging.get("stack").is_none());
    }
    let mut previous = Value::Null;
    for (target, suffix) in [("M42", "Z-M42"), ("NGC 7000", "Z-NGC-7000")] {
        ok(&rig.app, "PUT", "/api/session", json!({"target":target})).await;
        let start = ok(&rig.app, "POST", ROUTE, json!({})).await;
        let id = &start["stack"]["id"];
        assert_ne!(id, &previous);
        assert!(id.as_str().unwrap().ends_with(suffix));
        for path in ["/api/imaging", "/api/session"] {
            let v = ok(&rig.app, "GET", path, Value::Null).await;
            let imaging = if path.ends_with("session") {
                &v["imaging"]
            } else {
                &v
            };
            assert_eq!(&imaging["stack"]["id"], id);
            assert_eq!(imaging["stack"]["state"], "stacking");
        }
        let ended = ok(&rig.app, "DELETE", ROUTE, Value::Null).await;
        let stack = &ended["stack"];
        assert_eq!(stack["stacked"], 0);
        assert_eq!(
            stack["files"],
            json!({"json":format!("runs/{}/stack.json",id.as_str().unwrap())})
        );
        let folder = folder(&rig, stack);
        assert!(!folder.join("stack.fits").exists());
        assert!(!folder.join("stack.png").exists());
        assert_eq!(fs::read_dir(folder.join("subs")).unwrap().count(), 0);
        assert_eq!(
            manifest(&folder)["target"],
            if target == "M42" { "M42" } else { "NGC-7000" }
        );
        for path in ["/api/imaging", "/api/session"] {
            let v = ok(&rig.app, "GET", path, Value::Null).await;
            let imaging = if path.ends_with("session") {
                &v["imaging"]
            } else {
                &v
            };
            assert_eq!(&imaging["stack"]["id"], id);
            assert_eq!(imaging["stack"]["state"], "ended");
        }
        previous = id.clone();
    }
    let off = finder::router(AppState::shared(
        support::DB.clone(),
        finder::samples::Samples::default(),
    ));
    let v = ok(&off, "GET", "/api/imaging", Value::Null).await;
    assert!(v.get("stack").is_none());
    let v = ok(&off, "GET", "/api/session", Value::Null).await;
    assert!(v["imaging"].get("stack").is_none());
}

/// US8-S5: first failing write stops the save, preserving its ending reason.
#[tokio::test(start_paused = true)]
async fn folder_and_save_failures() {
    let rig = stand_in_rig("stack-console-folder", ScriptedSolver::new(), None).await;
    fs::create_dir_all(&rig.root).unwrap();
    fs::write(rig.root.join("runs"), b"blocked").unwrap();
    let (status, error) = call(&rig.app, "POST", ROUTE, json!({})).await;
    assert_eq!(status, 500);
    assert!(error["error"]
        .as_str()
        .unwrap()
        .starts_with("cannot make the stack folder:"));
    assert!(rig.state.stack().unwrap().view().is_none());
    assert!(rig.state.imaging().unwrap().holder().is_none());
    for remove in [true, false] {
        let rig = stand_in_rig("stack-console-save-fails", ScriptedSolver::new(), None).await;
        ok(&rig.app, "POST", ROUTE, json!({})).await;
        let stack = wait(&rig, |s| s["stacked"].as_u64().unwrap() >= 2).await;
        let folder = folder(&rig, &stack);
        if remove {
            fs::remove_dir_all(&folder).unwrap();
        } else {
            fs::create_dir(folder.join("stack.fits.part")).unwrap();
        }
        let view = ok(&rig.app, "DELETE", ROUTE, Value::Null).await;
        assert_eq!(view["stack"]["state"], "ended");
        assert_eq!(view["stack"]["ended_reason"], "stopped");
        assert!(view["stack"].get("files").is_none());
        assert!(view["stack"]["save_error"]
            .as_str()
            .unwrap()
            .starts_with("could not save the stack:"));
        assert!(!folder.join("stack.png").exists());
    }
}

/// US8-S6: FITS standard's big-endian IEEE singles and 2880-byte padding.
#[test]
fn cube_bytes_and_padding() {
    let bytes = encode_cube(&RealCube {
        width: 2,
        height: 2,
        planes: 3,
        samples: (1..=12).map(|n| n as f32).collect(),
        cards: vec![("NSUBS".into(), CardValue::Int(4))],
    });
    assert_eq!(bytes.len(), 5760);
    assert_eq!(
        cards(&bytes),
        [
            ("SIMPLE", "T"),
            ("BITPIX", "-32"),
            ("NAXIS", "3"),
            ("NAXIS1", "2"),
            ("NAXIS2", "2"),
            ("NAXIS3", "3"),
            ("NSUBS", "4")
        ]
        .map(|(k, v)| (k.into(), v.into()))
    );
    assert_eq!(&bytes[560..568], b"END     ");
    assert!(bytes[640..2880].iter().all(|b| *b == b' '));
    assert_eq!(
        &bytes[2880..2900],
        &[0x3f, 0x80, 0, 0, 0x40, 0, 0, 0, 0x40, 0x40, 0, 0, 0x40, 0x80, 0, 0, 0x40, 0xa0, 0, 0]
    );
    assert_eq!(&bytes[2924..2928], &[0x41, 0x40, 0, 0]);
    for plane in 0..3 {
        for y in 0..2 {
            for x in 0..2 {
                assert_eq!(
                    real_at(&bytes, plane, x, y),
                    (plane * 4 + y * 2 + x + 1) as f32
                );
            }
        }
    }
    assert!(bytes[2928..].iter().all(|b| *b == 0));
}
