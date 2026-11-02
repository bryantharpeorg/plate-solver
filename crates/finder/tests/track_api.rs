//! US7 oracles: the connected simulator's mechanics and rendered solves, the
//! tracker's actual 60-second samples, and FR-001/002 at explicit virtual UTC.
//! All timing bounds below use paused tokio time, sampled every 0.1 seconds.
#[path = "goto_api.rs"]
mod goto_api;

use axum::{body::Body, http::Request};
use finder::{ephemeris, model::separation, targets};
use goto_api::Rig;
use scope_node::mount::{Axis, Mount};
use serde_json::{json, Value};
use tower::ServiceExt;

const START: f64 = 1_768_528_800.0;
async fn request(
    app: &axum::Router,
    method: &str,
    path: &str,
    body: Value,
) -> (axum::http::StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

async fn track(rig: &Rig, on: bool) -> Value {
    let (status, body) = request(&rig.app, "POST", "/api/track", json!({"on":on})).await;
    assert_eq!(status, 200, "{body}");
    body
}

async fn tracking(rig: &Rig) {
    // Measure the handoff from the GoTo's terminal progress, not from the POST.
    let mut ended = None;
    for _ in 0..5000 {
        let progress = rig.state.goto().unwrap().progress();
        if let Some(progress) = progress {
            assert!(
                !matches!(progress.state, "failed" | "halted"),
                "{progress:?}"
            );
            if progress.state == "done" {
                ended.get_or_insert(rig.now());
            }
        }
        let body = rig.view().await;
        if body["track"]["state"] == "tracking" {
            assert!(rig.now() - ended.expect("tracking must follow GoTo completion") <= 2.0);
            assert_eq!(body["track"]["wanted"], true);
            assert_eq!(body["track"]["target"]["messier"], "M42");
            for _ in 0..5 {
                rig.tick().await;
                if !rig.stationary().await {
                    return;
                }
            }
            panic!("accepted tracking never moved either connected axis");
        }
        if let Some(end) = ended {
            assert!(rig.now() - end <= 2.0, "{body}");
        }
        rig.tick().await;
    }
    panic!("tracking never started: {}", rig.view().await);
}

async fn accepted() -> Rig {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    rig.sync().await;
    assert!(rig.view().await["mount"]["connected"].as_bool().unwrap());
    assert_eq!(track(&rig, true).await["track"]["state"], "armed");
    tracking(&rig).await;
    rig
}

async fn off(rig: &Rig) {
    let before = rig.now();
    let body = track(rig, false).await;
    assert_eq!(body["track"]["state"], "off");
    assert_eq!(body["track"]["wanted"], false);
    assert_eq!(body["track"]["ended_reason"], "stopped");
    while !rig.stationary().await && rig.now() - before < 0.5 {
        rig.tick().await;
    }
    assert!(rig.now() - before <= 0.5);
    assert!(rig.stationary().await);
}

async fn still(rig: &Rig, path: &str, foreign: bool, reason: &str) {
    let before = rig.mount.true_direction();
    let mut req = Request::post(path)
        .header("host", "localhost")
        .header("content-type", "application/json");
    if foreign {
        req = req.header("origin", "http://elsewhere.example");
    }
    let response = rig
        .app
        .clone()
        .oneshot(req.body(Body::from(r#"{"on":true}"#)).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), if foreign { 403 } else { 409 });
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(body["error"].as_str().unwrap().contains(reason), "{body}");
    for _ in 0..20 {
        rig.tick().await;
        assert!(rig.stationary().await);
    }
    let after = rig.mount.true_direction();
    assert!(separation(before.az, before.alt, after.az, after.alt) * 3600.0 < 1.0);
}

#[tokio::test(start_paused = true)]
async fn on_during_goto_starts_tracking_m42_within_two_seconds() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    rig.sync().await;
    assert_eq!(rig.call("/api/goto", false).await.0, 200);
    assert_eq!(track(&rig, true).await["track"]["state"], "armed");
    tracking(&rig).await;
    off(&rig).await;
}

#[tokio::test(start_paused = true)]
async fn failed_goto_records_its_reason_and_cancelled_arming_never_starts_rates() {
    for stop in [false, true] {
        let rig = Rig::new(START, 50.0, 120.0, true).await;
        rig.sync().await;
        assert_eq!(track(&rig, true).await["track"]["state"], "armed");
        // Prevent a successful final solve. The GoTo's own timeout is the
        // independent failure trigger; a separate run exercises observer Stop.
        rig.controls.stop_frames();
        if stop {
            assert_eq!(rig.call("/api/stop", true).await.0, 200);
        }
        for _ in 0..5000 {
            rig.tick().await;
            let body = rig.view().await;
            assert_ne!(body["track"]["state"], "tracking");
            if body["track"]["state"] == "off" {
                assert_eq!(body["track"]["wanted"], false);
                assert_eq!(
                    body["track"]["ended_reason"],
                    if stop {
                        json!("stopped")
                    } else {
                        assert_eq!(body["goto"]["state"], "failed");
                        body["goto"]["reason"].clone()
                    }
                );
                assert!(body["track"]["ended_reason"].is_string());
                for _ in 0..100 {
                    rig.tick().await;
                    assert!(rig.stationary().await);
                    assert_ne!(rig.view().await["track"]["state"], "tracking");
                }
                break;
            }
        }
        assert_eq!(rig.view().await["track"]["state"], "off");
    }
}

#[tokio::test(start_paused = true)]
async fn five_degrees_off_track_starts_goto_then_tracks() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    let m42 = targets::find("M42").unwrap();
    let (h, az) = simulator::sky::horizontal(m42.vector(), simulator::Site::default(), rig.now());
    rig.mount.push_to(h - 5.0, az);
    let truth = rig.mount.true_direction();
    assert!((separation(truth.az, truth.alt, az, h) - 5.0).abs() < 1e-9);
    rig.sync().await;
    assert!(rig.state.goto().unwrap().progress().is_none());
    let body = track(&rig, true).await;
    assert_eq!(body["track"]["state"], "armed");
    assert_eq!(
        rig.state.goto().unwrap().progress().unwrap().state,
        "slewing"
    );
    tracking(&rig).await;
    off(&rig).await;
}

#[tokio::test(start_paused = true)]
async fn off_stops_within_half_a_second_and_leaves_true_direction_fixed() {
    let rig = accepted().await;
    off(&rig).await;
    let before = rig.mount.true_direction();
    for _ in 0..100 {
        rig.tick().await;
        assert!(rig.stationary().await);
    }
    let after = rig.mount.true_direction();
    assert!(separation(before.az, before.alt, after.az, after.alt) * 3600.0 < 60.0);
}

#[tokio::test(start_paused = true)]
async fn live_preview_after_track_off_reaches_the_router_within_five_point_five_seconds() {
    let rig = accepted().await;
    off(&rig).await;
    let live = rig.state.live().unwrap();
    let mut events = live.subscribe();
    // Start just after a preview, exercising the full five-second cadence.
    let mut previous = None;
    for _ in 0..60 {
        rig.tick().await;
        while let Ok(report) = events.try_recv() {
            if report.preview_seq == Some(report.frame_id) {
                previous = Some(report.frame_id);
            }
        }
        if previous.is_some() {
            break;
        }
    }
    let previous = previous.expect("live previews must continue after Track off");
    rig.mount.push_to(40.0, 200.0);
    let pushed_at = rig.now();
    let mut fresh = None;
    for _ in 0..55 {
        rig.tick().await;
        assert!(rig.stationary().await);
        while let Ok(report) = events.try_recv() {
            // The 200 ms exposure must have started after the push. Receiving
            // an old JPEG, or publishing a preview without its frame, cannot pass.
            if report.frame_id > previous
                && report.preview_seq == Some(report.frame_id)
                && report.time - 0.1 > pushed_at
            {
                fresh = Some(report);
            }
        }
        if fresh.is_some() {
            break;
        }
    }
    let fresh = fresh.expect("no post-push preview within 5.5 seconds");
    assert!(rig.now() - pushed_at <= 5.5);
    let response = rig
        .app
        .clone()
        .oneshot(
            Request::get(format!("/api/live/preview.jpg?seq={}", fresh.frame_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["content-type"], "image/jpeg");
    let jpeg = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let image = image::load_from_memory_with_format(&jpeg, image::ImageFormat::Jpeg).unwrap();
    assert_eq!((image.width(), image.height()), (640, 478));
}

#[tokio::test(start_paused = true)]
async fn below_horizon_and_foreign_origin_refuse_after_accepted_tracking_moved() {
    let rig = accepted().await;
    off(&rig).await;
    rig.target("M8");
    let m8 = targets::find("M8").unwrap();
    assert!(simulator::sky::horizontal(m8.vector(), simulator::Site::default(), rig.now()).0 < 0.0);
    still(&rig, "/api/track", false, "M8 is below the horizon").await;
    rig.target("M42");
    still(&rig, "/api/track", true, "origin").await;
}

#[tokio::test(start_paused = true)]
async fn missing_mount_sync_site_target_and_sun_name_the_refusal() {
    let absent = Rig::new(START, 50.0, 120.0, false).await;
    still(&absent, "/api/track", false, "no mount").await;
    let rig = accepted().await;
    off(&rig).await;
    rig.controls.stop_frames();
    for _ in 0..5 {
        rig.tick().await;
    }
    for (edit, reason) in [
        (json!({"target":null}), "no target"),
        (json!({"target":"M42","site":null}), "site"),
        (json!({"site":{"lat_deg":0,"lon_deg":0}}), "Sun is up"),
    ] {
        let edit = if reason == "Sun is up" {
            let (ra, dec) = ephemeris::sun(finder::sky::julian_date(rig.now()));
            let lon = (ra - finder::sky::gmst_deg(finder::sky::julian_date(rig.now())) + 180.0)
                .rem_euclid(360.0)
                - 180.0;
            json!({"site":{"lat_deg":dec,"lon_deg":lon}})
        } else {
            edit
        };
        assert_eq!(request(&rig.app, "PUT", "/api/session", edit).await.0, 200);
        still(&rig, "/api/track", false, reason).await;
    }
    rig.state.session().reset_model(false, rig.now()).unwrap();
    still(&rig, "/api/track", false, "no sync").await;
}

#[tokio::test(start_paused = true)]
async fn impossible_imaging_camera_edits_are_atomic_and_checked() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    let camera = json!({"width_mm":23.5,"height_mm":15.6,"pitch_um":3.76});
    assert_eq!(
        request(
            &rig.app,
            "PUT",
            "/api/session",
            json!({"imaging_camera":camera})
        )
        .await
        .0,
        200
    );
    for (field, value) in [
        ("pitch_um", 0.0),
        ("pitch_um", 50.01),
        ("width_mm", 0.0),
        ("width_mm", 100.01),
        ("height_mm", -1.0),
        ("height_mm", 100.01),
    ] {
        let mut invalid = camera.clone();
        invalid[field] = json!(value);
        let before = rig.view().await;
        let (status, body) = request(
            &rig.app,
            "PUT",
            "/api/session",
            json!({"imaging_camera":invalid,"min_alt_deg":30}),
        )
        .await;
        assert_eq!(status, 400, "{body}");
        assert!(body["error"].is_string());
        let after = rig.view().await;
        assert_eq!(after["imaging_camera"], before["imaging_camera"]);
        assert_eq!(after["min_alt_deg"], before["min_alt_deg"]);
    }
}

#[tokio::test(start_paused = true)]
async fn live_solves_feed_exact_sixty_second_rms_rotation_and_exposure_readout() {
    let rig = accepted().await;
    assert_eq!(
        request(
            &rig.app,
            "PUT",
            "/api/session",
            json!({"imaging_camera":{"width_mm":23.5,"height_mm":15.6,"pitch_um":3.76}})
        )
        .await
        .0,
        200
    );
    for _ in 0..1200 {
        rig.tick().await;
    }
    // Freeze solve receipt so the router read and independent sample reduction
    // observe precisely the same window, without fabricating any error samples.
    rig.controls.stop_frames();
    for _ in 0..5 {
        rig.tick().await;
    }
    let body = rig.view().await;
    let samples = rig.state.session().tracker().unwrap().state().samples;
    let now = body["time"].as_f64().unwrap();
    assert!(samples.len() > 80, "live solves must reach the tracker");
    assert!(samples.iter().all(|s| now - s.time <= 60.0));
    assert!(samples.iter().any(|s| now - s.time > 50.0));
    let rms = (samples
        .iter()
        .map(|s| s.east_arcsec.powi(2) + s.north_arcsec.powi(2))
        .sum::<f64>()
        / samples.len() as f64)
        .sqrt();
    assert!(rms <= 20.0, "{rms}");
    let readout = &body["track"]["readout"];
    assert!((readout["rms_arcsec"].as_f64().unwrap() - rms).abs() <= 1e-9);
    let m42 = targets::find("M42").unwrap();
    let (alt, az) = finder::sky::alt_az(
        m42.ra_deg,
        m42.dec_deg,
        &finder::sky::Observer {
            lat: 40.0,
            lon: -75.0,
            time: now,
        },
    );
    let rotation = ephemeris::altaz_rates_from_alt_az(40.0, alt, az).field_rotation_deg_per_h;
    assert!(
        (readout["field_rotation_deg_per_min"].as_f64().unwrap() - rotation / 60.0).abs() <= 1e-9
    );
    let ephemeris::Exposure::Seconds(seconds) =
        ephemeris::longest_exposure(23.5, 15.6, 3.76, rotation.to_radians() / 3600.0, 1.0)
    else {
        panic!("M42 has field rotation");
    };
    assert!((readout["longest_exposure_s"].as_f64().unwrap() - seconds).abs() <= 1e-9);
    off(&rig).await;
}

#[tokio::test(start_paused = true)]
async fn foreign_bodiless_stop_ends_tracking_and_rates_stay_quiet_ten_seconds() {
    let rig = accepted().await;
    // Positive control above saw tracking drive an axis before the safety stop.
    let start = rig.now();
    assert_eq!(rig.call("/api/stop", true).await.0, 200);
    while !rig.stationary().await && rig.now() - start < 1.5 {
        rig.tick().await;
    }
    assert!(rig.now() - start <= 1.5);
    assert!(rig.stationary().await);
    let before = [
        rig.mount.state(Axis::Azimuth).await.position,
        rig.mount.state(Axis::Altitude).await.position,
    ];
    for _ in 0..100 {
        rig.tick().await;
        assert!(rig.stationary().await);
    }
    assert_eq!(
        [
            rig.mount.state(Axis::Azimuth).await.position,
            rig.mount.state(Axis::Altitude).await.position
        ],
        before
    );
    let body = rig.view().await;
    assert_eq!(body["track"]["state"], "off");
    assert_eq!(body["track"]["wanted"], false);
    assert_eq!(body["track"]["ended_reason"], "stopped");
}

#[tokio::test(start_paused = true)]
async fn stop_cancels_a_track_request_while_its_mount_rpc_is_pending() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    rig.sync().await;
    use std::{future::Future, task::Poll};
    let start = request(&rig.app, "POST", "/api/track", json!({"on":true}));
    tokio::pin!(start);
    // Poll exactly once: the first mount RPC must still be awaiting the node,
    // and Stop is polled before the start is allowed to make further progress.
    std::future::poll_fn(|cx| {
        assert!(start.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    let (stop, (status, _)) = tokio::join!(biased; rig.call("/api/stop", true), start);
    assert_eq!(stop.0, 200);
    assert!(status == 200 || status == 409);
    for _ in 0..100 {
        rig.tick().await;
        assert!(rig.stationary().await);
        let body = rig.view().await;
        if body.get("track").is_some() {
            assert_eq!(body["track"]["wanted"], false);
            assert_eq!(body["track"]["state"], "off");
        }
    }
}
