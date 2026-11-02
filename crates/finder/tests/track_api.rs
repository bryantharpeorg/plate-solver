//! US7 oracles: rendered catalog stars and the simulator's true direction in
//! paused time; FR-001/002 rates and the tracker's timestamped 60-second samples.
#[path = "goto_api.rs"]
mod goto_api;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use finder::{ephemeris, model::separation, sky, targets};
use goto_api::Rig;
use serde_json::{json, Value};
use tower::ServiceExt;
const START: f64 = 1_768_528_800.0;

async fn write(
    rig: &Rig,
    method: &str,
    path: &str,
    body: Value,
    foreign: bool,
) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("host", "localhost")
        .header("content-type", "application/json");
    if foreign {
        request = request.header("origin", "http://elsewhere.example");
    }
    let response = rig
        .app
        .clone()
        .oneshot(request.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
async fn track(rig: &Rig, on: bool) -> Value {
    let (status, body) = write(rig, "POST", "/api/track", json!({"on":on}), false).await;
    assert_eq!(status, 200, "{body}");
    body
}
async fn tracking(rig: &Rig) {
    let mut done = None;
    let mut last_running = rig.now();
    for _ in 0..5000 {
        let body = rig.view().await;
        if body["goto"]["state"] == "done" {
            // The preceding sample bounds the end from below, so polling
            // cannot add its 0.1-second interval to the two-second allowance.
            done.get_or_insert(last_running);
        } else {
            last_running = rig.now();
        }
        if body["track"]["state"] == "tracking" {
            assert_eq!(body["goto"]["state"], "done", "Track must wait for GoTo");
            if let Some(done) = done {
                assert!(rig.now() - done <= 2.0);
            }
            assert_eq!(body["track"]["target"]["messier"], "M42");
            assert_eq!(body["track"]["wanted"], true);
            return;
        }
        assert!(
            !matches!(body["goto"]["state"].as_str(), Some("failed" | "halted")),
            "{body}"
        );
        if let Some(done) = done {
            assert!(rig.now() - done <= 2.0, "{body}");
        }
        rig.tick().await;
    }
    panic!("never tracked: {}", rig.view().await);
}
async fn moving(rig: &Rig) {
    for _ in 0..20 {
        rig.tick().await;
        if !rig.stationary().await {
            return;
        }
    }
    panic!("accepted Track did not move a connected mount");
}
async fn accepted() -> Rig {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    rig.sync().await;
    assert_eq!(rig.view().await["mount"]["connected"], true);
    track(&rig, true).await;
    tracking(&rig).await;
    moving(&rig).await;
    rig
}
async fn quiet(rig: &Rig, seconds: usize) {
    let before = rig.mount.true_direction();
    for _ in 0..seconds * 10 {
        rig.tick().await;
        assert!(rig.stationary().await);
    }
    let after = rig.mount.true_direction();
    assert!(separation(before.az, before.alt, after.az, after.alt) * 3600.0 < 60.0);
}

#[tokio::test(start_paused = true)]
async fn track_arms_during_goto_then_starts_within_two_seconds() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    rig.sync().await;
    assert_eq!(rig.call("/api/goto", false).await.0, 200);
    assert_eq!(track(&rig, true).await["track"]["state"], "armed");
    tracking(&rig).await;
    moving(&rig).await;
    track(&rig, false).await;
}
#[tokio::test(start_paused = true)]
async fn five_degree_error_starts_goto_and_off_leaves_telescope_where_it_is() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    // Hand push the eyepiece five degrees below M42; independent simulator truth.
    let target = targets::find("M42").unwrap();
    let (alt, az) =
        simulator::sky::horizontal(target.vector(), simulator::Site::default(), rig.now());
    rig.mount.push_to(alt - 5.0, az);
    rig.sync().await;
    for _ in 0..20 {
        rig.tick().await;
    }
    assert!((rig.error_arcsec("M42") / 3600.0 - 5.0).abs() < 0.02);
    let body = track(&rig, true).await;
    assert_eq!(body["track"]["state"], "armed");
    assert!(matches!(
        rig.view().await["goto"]["state"].as_str(),
        Some("slewing" | "solving" | "correcting")
    ));
    tracking(&rig).await;
    moving(&rig).await;
    let start = rig.now();
    let body = track(&rig, false).await;
    while !rig.stationary().await {
        assert!(rig.now() - start < 0.5);
        rig.tick().await;
    }
    assert!(rig.now() - start <= 0.5);
    assert_eq!(body["track"]["state"], "off");
    assert_eq!(body["track"]["wanted"], false);
    assert_eq!(body["track"]["ended_reason"], "stopped");
    quiet(&rig, 10).await;
}
#[tokio::test(start_paused = true)]
async fn below_horizon_and_foreign_origin_refuse_after_positive_track_control() {
    let rig = accepted().await;
    track(&rig, false).await;
    rig.target("M8");
    assert_eq!(
        finder::pushto::class(
            targets::find("M8").unwrap(),
            rig.state.session().snapshot(rig.now()).site,
            15.0,
            rig.now()
        ),
        finder::pushto::Class::BelowHorizon
    );
    let (status, body) = write(&rig, "POST", "/api/track", json!({"on":true}), false).await;
    assert_eq!(status, 409);
    assert!(
        body["error"]
            .as_str()
            .unwrap()
            .contains("M8 is below the horizon"),
        "{body}"
    );
    quiet(&rig, 2).await;
    rig.target("M42");
    let (status, body) = write(&rig, "POST", "/api/track", json!({"on":true}), true).await;
    assert_eq!(status, 403);
    assert!(body["error"].as_str().unwrap().contains("origin"));
    quiet(&rig, 2).await;
}
#[tokio::test(start_paused = true)]
async fn foreign_bodiless_stop_ends_tracking_and_no_rate_restarts_it() {
    let rig = accepted().await;
    let start = rig.now();
    assert_eq!(rig.call("/api/stop", true).await.0, 200);
    while !rig.stationary().await {
        assert!(rig.now() - start < 1.5);
        rig.tick().await;
    }
    assert!(rig.now() - start <= 1.5);
    quiet(&rig, 10).await;
    let body = rig.view().await;
    assert_eq!(body["track"]["state"], "off");
    assert_eq!(body["track"]["wanted"], false);
    assert_eq!(body["track"]["ended_reason"], "stopped");
}
#[tokio::test(start_paused = true)]
async fn imaging_camera_edits_are_checked_and_atomic() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    let camera = json!({"width_mm":23.5,"height_mm":15.6,"pitch_um":3.76});
    assert_eq!(
        write(
            &rig,
            "PUT",
            "/api/session",
            json!({"imaging_camera":camera}),
            false
        )
        .await
        .0,
        200
    );
    for (field, value) in [
        ("pitch_um", 0.0),
        ("pitch_um", 50.1),
        ("width_mm", 0.0),
        ("width_mm", 100.1),
        ("height_mm", -1.0),
        ("height_mm", 100.1),
    ] {
        let mut bad = camera.clone();
        bad[field] = json!(value);
        let (status, body) = write(
            &rig,
            "PUT",
            "/api/session",
            json!({"target":"M8","imaging_camera":bad}),
            false,
        )
        .await;
        assert_eq!(status, 400, "{body}");
        assert!(body["error"].is_string());
        let body = rig.view().await;
        assert_eq!(body["imaging_camera"], camera);
        assert_eq!(body["target"]["messier"], "M42");
    }
    let (_, body) = write(
        &rig,
        "PUT",
        "/api/session",
        json!({"imaging_camera":null}),
        false,
    )
    .await;
    assert!(body.get("imaging_camera").is_none());
}
#[tokio::test(start_paused = true)]
async fn readout_uses_live_errors_last_sixty_seconds_and_fr001_fr002() {
    let rig = accepted().await;
    let camera = json!({"width_mm":23.5,"height_mm":15.6,"pitch_um":3.76});
    assert_eq!(
        write(
            &rig,
            "PUT",
            "/api/session",
            json!({"imaging_camera":camera}),
            false
        )
        .await
        .0,
        200
    );
    for _ in 0..750 {
        rig.tick().await;
    }
    let tracker = rig.state.session().tracker().unwrap();
    let samples = tracker.state().samples;
    assert!(samples.len() > 30, "live solves must reach the tracker");
    assert!(samples.iter().all(|s| rig.now() - s.time <= 60.0));
    assert!(samples.first().unwrap().time < rig.now() - 50.0);
    let expected = (samples
        .iter()
        .map(|s| s.east_arcsec.powi(2) + s.north_arcsec.powi(2))
        .sum::<f64>()
        / samples.len() as f64)
        .sqrt();
    let body = rig.view().await;
    let readout = &body["track"]["readout"];
    let actual = readout["rms_arcsec"].as_f64().unwrap();
    assert!((actual - expected).abs() <= 1e-9, "{actual} vs {expected}");
    assert!(actual <= 20.0, "{actual}");
    let target = targets::find("M42").unwrap();
    let site = rig.state.session().snapshot(rig.now()).site.unwrap();
    let (alt, az) = sky::alt_az(
        target.ra_deg,
        target.dec_deg,
        &sky::Observer {
            lat: site.lat_deg,
            lon: site.lon_deg,
            time: body["time"].as_f64().unwrap(),
        },
    );
    let rotation =
        ephemeris::altaz_rates_from_alt_az(site.lat_deg, alt, az).field_rotation_deg_per_h;
    assert!(
        (readout["field_rotation_deg_per_min"].as_f64().unwrap() - rotation / 60.0).abs() <= 1e-9
    );
    let ephemeris::Exposure::Seconds(exposure) =
        ephemeris::longest_exposure(23.5, 15.6, 3.76, rotation.to_radians() / 3600.0, 1.0)
    else {
        panic!("finite M42 exposure");
    };
    assert!((readout["longest_exposure_s"].as_f64().unwrap() - exposure).abs() <= 1e-9);
    track(&rig, false).await;
}

#[tokio::test(start_paused = true)]
async fn track_names_missing_mount_sync_site_target_and_sun_refusals() {
    let rig = Rig::new(START, 50.0, 120.0, false).await;
    let (status, body) = write(&rig, "POST", "/api/track", json!({"on":true}), false).await;
    assert_eq!(status, 409);
    assert!(body["error"].as_str().unwrap().contains("no mount"));
    assert!(rig.view().await.get("track").is_none());
    let rig = accepted().await;
    track(&rig, false).await;
    rig.controls.stop_frames();
    for _ in 0..10 {
        rig.tick().await;
    }
    rig.state.session().reset_model(false, rig.now()).unwrap();
    let (status, body) = write(&rig, "POST", "/api/track", json!({"on":true}), false).await;
    assert_eq!(status, 409);
    assert!(body["error"].as_str().unwrap().contains("no sync"));
    quiet(&rig, 2).await;
    // Restore a qualified sync before exercising the remaining guard precedence.
    rig.controls.start_frames();
    rig.sync().await;
    for edit in [
        json!({"site":null}),
        json!({"site":{"lat_deg":40,"lon_deg":-75},"target":null}),
    ] {
        assert_eq!(write(&rig, "PUT", "/api/session", edit, false).await.0, 200);
        let (status, body) = write(&rig, "POST", "/api/track", json!({"on":true}), false).await;
        assert_eq!(status, 409);
        assert!(
            body["error"].as_str().unwrap().contains("needed")
                || body["error"].as_str().unwrap().contains("no target"),
            "{body}"
        );
        quiet(&rig, 1).await;
    }
    let (ra, dec) = ephemeris::sun(sky::julian_date(rig.now()));
    let lon = (ra - sky::gmst_deg(sky::julian_date(rig.now())) + 180.0).rem_euclid(360.0) - 180.0;
    assert_eq!(
        write(
            &rig,
            "PUT",
            "/api/session",
            json!({"site":{"lat_deg":dec+85.0,"lon_deg":lon},"target":"M42"}),
            false
        )
        .await
        .0,
        200
    );
    let (status, body) = write(&rig, "POST", "/api/track", json!({"on":true}), false).await;
    assert_eq!(status, 409);
    assert!(body["error"].as_str().unwrap().contains("Sun is up"));
    quiet(&rig, 2).await;
}

#[tokio::test(start_paused = true)]
async fn armed_stop_cancels_handoff_and_failed_goto_keeps_its_reason() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    rig.sync().await;
    assert_eq!(track(&rig, true).await["track"]["state"], "armed");
    assert_eq!(rig.call("/api/stop", true).await.0, 200);
    for _ in 0..15 {
        rig.tick().await;
    }
    quiet(&rig, 10).await;
    assert_eq!(rig.view().await["track"]["ended_reason"], "stopped");
    assert_eq!(track(&rig, true).await["track"]["state"], "armed");
    // Stop the GoTo directly to prove its terminal reason reaches the armed state.
    rig.state.goto().unwrap().stop().await;
    for _ in 0..15 {
        rig.tick().await;
    }
    let body = rig.view().await;
    assert_eq!(body["track"]["state"], "off");
    assert_eq!(body["track"]["wanted"], false);
    assert_eq!(body["track"]["ended_reason"], body["goto"]["reason"]);
    quiet(&rig, 2).await;
}

#[tokio::test(start_paused = true)]
async fn already_centred_track_is_immediate_and_repeated_on_is_idempotent() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    let target = targets::find("M42").unwrap();
    rig.sim.point_eyepiece_at(target.ra_deg, target.dec_deg);
    rig.sync().await;
    for _ in 0..15 {
        rig.tick().await;
    }
    assert!(rig.error_arcsec("M42") < 60.0);
    let body = track(&rig, true).await;
    assert_eq!(body["track"]["state"], "tracking");
    assert!(body.get("goto").is_none());
    assert!(body["track"]["readout"].get("longest_exposure_s").is_none());
    let tracker = rig.state.session().tracker().unwrap();
    track(&rig, true).await;
    assert!(std::sync::Arc::ptr_eq(
        &tracker,
        &rig.state.session().tracker().unwrap()
    ));
    moving(&rig).await;
    track(&rig, false).await;
}

#[tokio::test(start_paused = true)]
async fn dropping_console_cancels_tracking_without_retaining_the_session() {
    let rig = accepted().await;
    let session = std::sync::Arc::downgrade(&rig.state.session());
    let mount = rig.mount.clone();
    drop(rig);
    for _ in 0..40 {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        if session.upgrade().is_none() {
            break;
        }
    }
    assert!(
        session.upgrade().is_none(),
        "tracker and link must not retain each other through the session"
    );
    use scope_node::mount::{Axis, Mount};
    assert!(!mount.state(Axis::Azimuth).await.moving);
    assert!(!mount.state(Axis::Altitude).await.moving);
}
