//! US7: uniform scripted samples and the specification's worked linear search
//! are independent oracles. Counts, exposures and words are exact; comparisons
//! with rounded levels allow 0.001. Every wait runs in paused time.

use std::{
    fs,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use finder::{
    imaging::{calibration::service::HOLDER, fits::CardValue as C},
    AppState,
};
use serde_json::{json, Value};
use tower::ServiceExt;

use crate::{
    calibration_console::{request, wait_until, Rig},
    calibration_darks::{scripted_rig, Script},
    calibration_library::{read_fits, ReadFits},
    support::{live as support, Scratch},
};

pub(crate) const CAL: &str = "/api/imaging/calibration";
pub(crate) const DARKS: &str = "/api/imaging/calibration/darks";
pub(crate) const FLATS: &str = "/api/imaging/calibration/flats";

pub(crate) async fn edit(app: &Router, body: Value) {
    let (status, answer) = request(app, "PUT", "/api/imaging/settings", body, None).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
}
pub(crate) async fn post(app: &Router, path: &str, body: Value) -> Value {
    let (status, answer) = request(app, "POST", path, body, None).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    answer
}
pub(crate) async fn finish(app: &Router, state: &str) -> Value {
    wait_until(app, CAL, |v| v["set"]["state"] == state).await
}
pub(crate) async fn settle(app: &Router) -> Value {
    wait_until(app, CAL, |v| {
        matches!(
            v["set"]["state"].as_str(),
            Some("needs_darks" | "done" | "failed" | "stopped")
        )
    })
    .await
}
pub(crate) fn near(value: &Value, expected: f64, tolerance: f64) {
    let n = value.as_f64().unwrap();
    assert!(
        (n - expected).abs() <= tolerance,
        "{n} != {expected} ± {tolerance}"
    );
}
pub(crate) fn read_master(root: &Path, master: &Value) -> ReadFits {
    read_fits(
        &fs::read(
            root.join("calibration")
                .join(master["file"].as_str().unwrap()),
        )
        .unwrap(),
    )
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
async fn ready(app: &Router) {
    wait_until(app, "/api/imaging", |v| v["state"] == "ready").await;
}
async fn restored(rig: &Rig, exposure: f64) {
    assert_eq!(rig.state.imaging().unwrap().holder(), None);
    let (_, view) = request(&rig.app, "GET", "/api/imaging", Value::Null, None).await;
    assert_eq!(view["settings"]["exposure_ms"], exposure);
}
async fn refuse(app: &Router, path: &str, body: Value, error: &str) {
    let (status, answer) = request(app, "POST", path, body, None).await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert_eq!(answer["error"], error);
}
async fn drop_set(app: &Router, set: &Value) {
    assert_eq!(
        request(
            app,
            "DELETE",
            &format!("{CAL}/{}", set["id"].as_str().unwrap()),
            Value::Null,
            None
        )
        .await
        .0,
        StatusCode::OK
    );
}

/// US7-S1: 8160 and 16160 counts give 80 counts/ms. The third search
/// exposure reaches 45% of 65535. Flat/dark temperatures deliberately differ:
/// the master must retain the flats' 20°C, independent of the darks' 25°C.
#[tokio::test(start_paused = true)]
async fn linear_search_then_darks_keeps_frames_and_makes_a_normalised_master() {
    let scratch = Scratch::new("flats-linear");
    let root = scratch.0.join("imaging");
    let covered = Arc::new(AtomicBool::new(false));
    let flag = covered.clone();
    let mut script = Script::new(move |_, e| {
        if flag.load(Ordering::Acquire) {
            160.0
        } else {
            160.0 + 80.0 * e
        }
    });
    script.temp = Some(Arc::new(|n| if n <= 6 { 20.0 } else { 25.0 }));
    let (rig, camera) = scripted_rig(&root, script);
    ready(&rig.app).await;
    edit(&rig.app, json!({"exposure_ms":100})).await;
    let answer = post(&rig.app, FLATS, json!({"count":3,"label":"desk"})).await;
    let s = &answer["set"];
    assert_eq!(s["kind"], "flat");
    assert_eq!(s["state"], "search");
    assert_eq!(s["label"], "desk");
    assert_eq!(s["wanted"], 3);
    let id = s["id"].as_str().unwrap();
    assert!(id.starts_with("flat-20260116T02") && id.ends_with('Z'));
    assert_eq!(id.len(), 21);
    assert_eq!(rig.state.imaging().unwrap().holder(), Some(HOLDER));
    let waiting = finish(&rig.app, "needs_darks").await;
    assert_eq!(
        camera.calls(),
        [100.0, 200.0, 366.634, 366.634, 366.634, 366.634]
    );
    assert_eq!(waiting["set"]["exposure_ms"], 366.634);
    assert_eq!(waiting["set"]["taken"], 3);
    near(&waiting["set"]["level_share"], 0.45, 0.001);
    assert!(waiting["set"].get("error").is_none());
    assert_eq!(waiting["masters"], json!([]));
    restored(&rig, 100.0).await;

    // Waiting flats hold no lease: a normal exposure setting can change.
    edit(&rig.app, json!({"exposure_ms":200})).await;
    covered.store(true, Ordering::Release);
    let answer = post(&rig.app, FLATS, json!({"darks":true})).await;
    assert_eq!(answer["set"]["state"], "flat_darks");
    assert_eq!(answer["set"]["darks_taken"], 0);
    assert_eq!(rig.state.imaging().unwrap().holder(), Some(HOLDER));
    let done = finish(&rig.app, "done").await;
    assert_eq!(&camera.calls()[6..], [1.0, 366.634, 366.634, 366.634]);
    assert_eq!(done["set"]["darks_taken"], 3);
    assert_eq!(done["set"]["taken"], 3);
    assert_eq!(done["masters"].as_array().unwrap().len(), 1);
    let m = &done["masters"][0];
    for (key, expected) in [
        ("id", json!(id)),
        ("kind", json!("flat")),
        ("label", json!("desk")),
        ("frames", json!(3)),
        ("dark_frames", json!(3)),
        ("checked", json!(true)),
        ("exposure_ms", json!(366.634)),
        ("imaging_angle_deg", Value::Null),
        ("gain", json!(100.0)),
        ("offset", json!(10.0)),
        ("width", json!(64)),
        ("height", json!(48)),
        ("bad_pixels", Value::Null),
        ("temp_c", json!(20.0)),
        ("file", json!(format!("flats/{id}.fits"))),
    ] {
        assert_eq!(m[key], expected, "{key}");
    }
    near(&m["level_share"], 0.45, 0.001);
    let master = read_master(&root, m);
    assert!(master.samples.iter().all(|&v| v == 1.0));
    for (key, expected) in [
        ("IMAGETYP", C::Text("MASTER FLAT".into())),
        ("NFRAMES", C::Int(3)),
        ("NDARKS", C::Int(3)),
        ("DARKTEST", C::Logical(true)),
        ("EXPTIME", C::Real(0.366634)),
        ("FLATLBL", C::Text("desk".into())),
    ] {
        assert_eq!(card(&master, key), expected);
    }
    assert!(!master.cards.iter().any(|(k, _)| k == "IMGANGLE"));
    let mut frames = Vec::new();
    for date in fs::read_dir(root.join("frames")).unwrap() {
        for file in fs::read_dir(date.unwrap().path()).unwrap() {
            frames.push(file.unwrap().path());
        }
    }
    frames.sort();
    assert_eq!(frames.len(), 10);
    for (i, file) in frames.iter().enumerate() {
        let image = read_fits(&fs::read(file).unwrap());
        assert_eq!(
            card(&image, "IMAGETYP"),
            C::Text(if i < 6 { "FLAT" } else { "DARK" }.into())
        );
    }
    restored(&rig, 200.0).await;
}

/// US7-S2: worked examples pin every branch, the 30s cap, rounding, settings
/// held to the first search frame, and the eighth (not ninth) exposure's text.
#[tokio::test(start_paused = true)]
async fn search_paths_and_exact_failures_restore_the_original_exposure() {
    for case in 0..7 {
        let scratch = Scratch::new(&format!("flats-search-{case}"));
        let root = scratch.0.join("imaging");
        let mut script = Script::new(move |n, e| match case {
            1 => 160.0 + 40.0 * e,
            2 => 160.0 + 0.01 * e,
            3 => 65535.0,
            4 => 20000.0,
            5 => {
                if n % 2 == 1 {
                    19491.0
                } else {
                    39491.0
                }
            }
            _ => 160.0 + 80.0 * e,
        });
        if case == 6 {
            script.gain = Some(Arc::new(|n| if n == 1 { 100.0 } else { 150.0 }));
        }
        let (rig, camera) = scripted_rig(&root, script);
        ready(&rig.app).await;
        let original = if matches!(case, 0 | 1 | 3) {
            1000.0
        } else {
            100.0
        };
        edit(&rig.app, json!({"exposure_ms":original})).await;
        post(&rig.app, FLATS, json!({"count":3,"label":"desk"})).await;
        let result = settle(&rig.app).await;
        let (exposures, error): (&[f64], Option<&str>) = match case {
            0 => (&[1000.0,125.0,250.0,366.634,366.634,366.634,366.634], None),
            1 => (&[1000.0,500.0,733.269,733.269,733.269,733.269], None),
            2 => (&[100.0,200.0,30000.0], Some("the flat is too dim: 30000 ms reads 0.7% of full scale; is the panel lit?")),
            3 => (&[1000.0,125.0,15.625,1.953,1.0], Some("the flat is too bright: 1 ms reads 100.0% of full scale; dim the panel")),
            4 => (&[100.0,200.0], Some("the flat's level does not rise with the exposure: 100 ms and 200 ms read 20000 and 20000")),
            5 => (&[100.0,200.0,149.999,174.999,162.499,168.749,165.624,167.186], Some("no exposure put the flat between 40 and 50% of full scale in 8 tries; the last, 167.186 ms, read 60.3%")),
            _ => (&[100.0,200.0], Some("the imaging settings changed during the set")),
        };
        assert_eq!(camera.calls(), exposures, "case {case}");
        assert_eq!(
            result["set"]["state"],
            if error.is_some() {
                "failed"
            } else {
                "needs_darks"
            }
        );
        if let Some(error) = error {
            assert_eq!(result["set"]["error"], error);
        }
        assert_eq!(result["masters"], json!([]));
        restored(&rig, original).await;
    }
}

/// US7-S3: the sixth exposure is the third kept flat. A rejected second step
/// retains the original flats, clears its error on retry, and does not replace
/// their settings with changed gain. Anyway bypasses darkness, recorded in FITS.
#[tokio::test(start_paused = true)]
async fn failures_retain_flats_for_retry_and_anyway_records_the_skipped_test() {
    for case in 0..5 {
        let scratch = Scratch::new(&format!("flats-retry-{case}"));
        let root = scratch.0.join("imaging");
        let covered = Arc::new(AtomicBool::new(false));
        let flag = covered.clone();
        let mut script = Script::new(move |n, e| {
            if flag.load(Ordering::Acquire) {
                if case == 4 && e > 1.0 {
                    170.0
                } else {
                    160.0
                }
            } else if case == 0 && n == 6 {
                40000.0
            } else {
                160.0 + 80.0 * e
            }
        });
        if case == 1 {
            script.fail = Some(1);
        }
        let (rig, _) = scripted_rig(&root, script);
        ready(&rig.app).await;
        edit(&rig.app, json!({"exposure_ms":100})).await;
        post(&rig.app, FLATS, json!({"count":3,"label":"desk"})).await;
        let first = settle(&rig.app).await;
        if case < 2 {
            assert_eq!(first["set"]["state"], "failed");
            assert_eq!(
                first["set"]["error"],
                if case == 0 {
                    "a flat reads 61.0% of full scale, outside 40 to 50; is the panel steady?"
                } else {
                    "a flat failed: the exposure failed: the camera fell over"
                }
            );
            assert_eq!(first["set"]["taken"], if case == 0 { 2 } else { 0 });
        } else {
            assert_eq!(first["set"]["state"], "needs_darks");
            if case != 2 {
                covered.store(true, Ordering::Release);
            }
            if case == 3 {
                edit(&rig.app, json!({"gain":200})).await;
            }
            post(&rig.app, FLATS, json!({"darks":true})).await;
            let retry = settle(&rig.app).await;
            assert_eq!(retry["set"]["state"], "needs_darks");
            assert_eq!(retry["set"]["taken"], 3);
            assert_eq!(retry["set"]["darks_taken"], 0);
            assert_eq!(retry["masters"], json!([]));
            assert_eq!(retry["set"]["error"], match case {
                2 => "the darks are not dark: one reads 29491 against an offset level of 240; cover the telescope",
                3 => "the imaging settings changed during the set",
                _ => "the darks are not dark: one reads 170 against an offset level of 160; cover the telescope",
            });
            assert_eq!(
                retry["set"].get("not_dark"),
                if case == 3 { None } else { Some(&json!(true)) }
            );
            restored(&rig, 100.0).await;
            covered.store(true, Ordering::Release);
            if case == 3 {
                edit(&rig.app, json!({"gain":100})).await;
            }
            post(
                &rig.app,
                FLATS,
                if case == 4 {
                    json!({"darks":true,"anyway":true})
                } else {
                    json!({"darks":true})
                },
            )
            .await;
            let done = finish(&rig.app, "done").await;
            assert!(done["set"].get("error").is_none());
            assert!(done["set"].get("not_dark").is_none());
            assert_eq!(done["masters"][0]["checked"], case != 4);
            let master = read_master(&root, &done["masters"][0]);
            assert_eq!(card(&master, "DARKTEST"), C::Logical(case != 4));
            assert!(master.samples.iter().all(|&s| s == 1.0));
        }
        restored(&rig, 100.0).await;
    }
}

/// US7-S4: invalid requests do not take the lease. The same running/waiting
/// refusal point applies to both routes, before an exposure of the set's own.
#[tokio::test(start_paused = true)]
async fn request_order_waiting_drop_and_stopping_the_second_step() {
    let scratch = Scratch::new("flats-routes");
    let root = scratch.0.join("imaging");
    let covered = Arc::new(AtomicBool::new(false));
    let flag = covered.clone();
    let (rig, _) = scripted_rig(
        &root,
        Script::new(move |_, e| {
            if flag.load(Ordering::Acquire) {
                160.0
            } else {
                160.0 + 80.0 * e
            }
        }),
    );
    ready(&rig.app).await;
    let shape = "send count and label to take flats, or darks: true to take their darks";
    for (body, error) in [
        (json!({}), shape),
        (json!({"count":3}), shape),
        (json!({"label":"desk"}), shape),
        (json!({"count":3,"label":"desk","darks":true}), shape),
        (json!({"count":3,"label":"desk","anyway":true}), shape),
        (json!({"anyway":true}), shape),
        (json!({"darks":false}), shape),
        (
            json!({"count":2,"label":"desk"}),
            "count must be a whole number from 3 to 64",
        ),
        (
            json!({"count":65,"label":"desk"}),
            "count must be a whole number from 3 to 64",
        ),
    ] {
        let (status, answer) = request(&rig.app, "POST", FLATS, body, None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(answer["error"], error);
    }
    for label in ["", "Desk", "-a", "a_b", "é", &"a".repeat(33)] {
        let (status, answer) = request(
            &rig.app,
            "POST",
            FLATS,
            json!({"count":3,"label":label}),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(answer["error"], "label must be 1 to 32 characters of a to z, 0 to 9 and -, starting with a letter or digit");
    }
    for body in [
        json!({"count":"3","label":"desk"}),
        json!({"count":3.5,"label":"desk"}),
        json!({"darks":true,"unknown":1}),
    ] {
        assert_eq!(
            request(&rig.app, "POST", FLATS, body, None).await.0,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        request(
            &rig.app,
            "POST",
            FLATS,
            json!({}),
            Some("https://foreign.example")
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let response = rig
        .app
        .clone()
        .oneshot(
            Request::post(FLATS)
                .header("content-type", "text/plain")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    refuse(
        &rig.app,
        FLATS,
        json!({"darks":true}),
        "no flats are waiting for their darks",
    )
    .await;
    assert_eq!(rig.state.imaging().unwrap().holder(), None);
    let off = finder::router(AppState::shared(
        support::DB.clone(),
        finder::samples::Samples::default(),
    ));
    refuse(
        &off,
        FLATS,
        json!({"count":2,"label":"desk"}),
        "this console takes no imaging frames",
    )
    .await;
    assert_eq!(
        request(&off, "POST", FLATS, json!({"count":"3"}), None)
            .await
            .0,
        StatusCode::BAD_REQUEST
    );

    edit(&rig.app, json!({"exposure_ms":10000})).await;
    let started = post(
        &rig.app,
        FLATS,
        json!({"count":3,"label":"desk","darks":false,"anyway":false}),
    )
    .await;
    wait_until(&rig.app, "/api/imaging", |v| {
        v["frame"]["state"] == "exposing"
    })
    .await;
    let running = "a set of darks or flats is already being taken";
    for (path, body) in [
        (DARKS, json!({"count":3})),
        (FLATS, json!({"count":3,"label":"again"})),
        (FLATS, json!({"darks":true})),
    ] {
        refuse(&rig.app, path, body, running).await;
    }
    drop_set(&rig.app, &started["set"]).await;
    finish(&rig.app, "stopped").await;
    restored(&rig, 10000.0).await;
    edit(&rig.app, json!({"exposure_ms":100})).await;
    post(&rig.app, FLATS, json!({"count":3,"label":"desk"})).await;
    let waiting = finish(&rig.app, "needs_darks").await;
    for (path, body) in [
        (DARKS, json!({"count":3})),
        (FLATS, json!({"count":3,"label":"again"})),
    ] {
        refuse(
            &rig.app,
            path,
            body,
            "flats are waiting for their darks: take those or drop them first",
        )
        .await;
    }
    drop_set(&rig.app, &waiting["set"]).await;
    finish(&rig.app, "stopped").await;
    refuse(
        &rig.app,
        FLATS,
        json!({"darks":true}),
        "no flats are waiting for their darks",
    )
    .await;
    post(&rig.app, FLATS, json!({"count":3,"label":"0-desk"})).await;
    finish(&rig.app, "needs_darks").await;
    covered.store(true, Ordering::Release);
    let darks = post(&rig.app, FLATS, json!({"darks":true})).await;
    refuse(&rig.app, DARKS, json!({"count":3}), running).await;
    drop_set(&rig.app, &darks["set"]).await;
    let stopped = finish(&rig.app, "stopped").await;
    assert_eq!(stopped["masters"], json!([]));
    restored(&rig, 100.0).await;
}
