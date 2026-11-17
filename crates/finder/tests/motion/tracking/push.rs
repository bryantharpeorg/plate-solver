//! Oracles: the simulator's true eyepiece direction and the session's own view,
//! sampled in paused time; angular acceptance is 60 arcseconds, and the pointing
//! model's syncs and epochs are read as the session keeps them.
use crate::support::{goto::Rig, tracker};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use finder::{ephemeris, model::separation, session::Aim, sky, sky::Observer, targets};
use serde_json::{json, Value};
use std::time::Duration;
use tower::ServiceExt;
const START: f64 = 1_768_528_800.0;
const TICKS_PER_SECOND: usize = 10;

async fn write(rig: &Rig, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(path)
        .header("host", "localhost")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = rig.app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

async fn track(rig: &Rig, on: bool) -> Value {
    let (status, body) = write(rig, "POST", "/api/track", json!({"on":on})).await;
    assert_eq!(status, 200, "{body}");
    body
}

/// The J2000 place the tube points at now, from the simulator's own ray.
fn true_place(rig: &Rig) -> (f64, f64) {
    let direction = rig.mount.true_direction();
    let vector = tracker::equatorial(direction.alt, direction.az, rig.now());
    let (ra, dec) = vector.to_radec();
    (ra.to_degrees(), dec.to_degrees())
}

fn push_place(view: &Value) -> (f64, f64) {
    let push = &view["track"]["push"];
    (
        push["ra_deg"].as_f64().expect("push place"),
        push["dec_deg"].as_f64().expect("push place"),
    )
}

/// Read the session view every 100 ms of virtual time until the condition holds,
/// for at most 120 s of virtual time.
async fn until(rig: &Rig, mut condition: impl FnMut(&Value) -> bool) -> Value {
    let start = rig.now();
    loop {
        let view = rig.view().await;
        if condition(&view) {
            return view;
        }
        assert!(rig.now() - start < 120.0, "never came true: {view}");
        rig.tick().await;
    }
}

/// The rig on M42 as spec.md § User Scenarios defines it: centred as routes.rs's
/// `centred()`, then 30 s more of tracking so the mount's play is taken up.
async fn on_m42() -> Rig {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    let target = targets::find("M42").unwrap();
    rig.sim.point_eyepiece_at(target.ra_deg, target.dec_deg);
    rig.sync().await;
    assert_eq!(rig.view().await["mount"]["connected"], true);
    // Track reads the newest solved pointing; wait for one solved on M42.
    for _ in 0..10 {
        let pointing = rig.view().await["pointing"].clone();
        let within = separation(
            pointing["ra_deg"].as_f64().unwrap(),
            pointing["dec_deg"].as_f64().unwrap(),
            target.ra_deg,
            target.dec_deg,
        ) < 1.0 / 60.0;
        if within {
            break;
        }
        rig.tick().await;
    }
    let body = track(&rig, true).await;
    assert_eq!(body["track"]["state"], "tracking", "{body}");
    assert!(
        body.get("goto").is_none(),
        "a centred Track ran a GoTo: {body}"
    );
    assert_eq!(body["track"]["target"]["messier"], "M42", "{body}");
    assert_eq!(body["track"]["wanted"], true, "{body}");
    for _ in 0..20 {
        rig.tick().await;
        if !rig.stationary().await {
            break;
        }
    }
    assert!(
        !rig.stationary().await,
        "accepted Track did not move the mount"
    );
    for _ in 0..30 * TICKS_PER_SECOND {
        rig.tick().await;
    }
    assert_eq!(rig.view().await["track"]["state"], "tracking");
    rig
}

/// Wait until the next solved pointing is numbered after this one, so no frame is
/// open at the moment the caller pushes next (spec.md, plan trap 14).
async fn after_pointing(rig: &Rig, seq: u64) {
    until(rig, |view| {
        view["pointing"]["seq"].as_u64().is_some_and(|s| s > seq)
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn a_two_degree_push_is_followed_and_read_at_the_place_followed() {
    let rig = on_m42().await;
    let target = targets::find("M42").unwrap();
    let pushed = rig.now();
    rig.mount.push_by(0.0, -2.0);
    let view = until(&rig, |view| {
        view["track"]["push"]["state"].as_str() == Some("following")
    })
    .await;
    assert_eq!(view["track"]["state"], "tracking", "{view}");
    assert_eq!(view["track"]["wanted"], true, "{view}");
    assert_eq!(view["track"]["target"]["id"], "NGC 1976", "{view}");
    let at = view["track"]["push"]["at"].as_f64().unwrap();
    assert!(
        at >= pushed + 5.0,
        "followed {at} < 5 s after the push {pushed}"
    );
    let (ra, dec) = push_place(&view);
    let (true_ra, true_dec) = true_place(&rig);
    assert!(
        separation(ra, dec, true_ra, true_dec) * 3600.0 <= 60.0,
        "followed place {ra} {dec} vs true place {true_ra} {true_dec}"
    );
    // FR-010: the readout's field rotation is the place followed at the view's time.
    let site = rig.state.session().snapshot(rig.now()).site.unwrap();
    let time = view["time"].as_f64().unwrap();
    let (alt, az) = sky::alt_az(
        ra,
        dec,
        &Observer {
            lat: site.lat_deg,
            lon: site.lon_deg,
            time,
        },
    );
    let rotation =
        ephemeris::altaz_rates_from_alt_az(site.lat_deg, alt, az).field_rotation_deg_per_h;
    let readout = view["track"]["readout"]["field_rotation_deg_per_min"]
        .as_f64()
        .unwrap();
    assert!(
        (readout - rotation / 60.0).abs() <= 1e-9,
        "{readout} vs {}",
        rotation / 60.0
    );
    for _ in 0..120 {
        for _ in 0..TICKS_PER_SECOND {
            rig.tick().await;
        }
        let (true_ra, true_dec) = true_place(&rig);
        assert!(
            separation(ra, dec, true_ra, true_dec) * 3600.0 <= 60.0,
            "the eyepiece left the place followed at {}",
            rig.now()
        );
        assert!(
            separation(true_ra, true_dec, target.ra_deg, target.dec_deg) > 1.9,
            "the eyepiece was dragged back at {}",
            rig.now()
        );
    }
    track(&rig, false).await;
}

#[tokio::test(start_paused = true)]
async fn a_three_arcminute_push_is_corrected_with_no_push_key() {
    let rig = on_m42().await;
    let target = targets::find("M42").unwrap();
    rig.mount.push_by(0.0, -0.05);
    let start = rig.now();
    loop {
        let view = rig.view().await;
        assert!(view["track"].get("push").is_none(), "{view}");
        assert_eq!(view["track"]["state"], "tracking", "{view}");
        let (true_ra, true_dec) = true_place(&rig);
        if separation(true_ra, true_dec, target.ra_deg, target.dec_deg) * 3600.0 <= 60.0 {
            break;
        }
        assert!(
            rig.now() - start < 30.0,
            "the small push was not corrected within 30 s"
        );
        rig.tick().await;
    }
    // The correction holds for the rest of the 30 s, still with no push key.
    while rig.now() - start < 30.0 {
        let view = rig.view().await;
        assert!(view["track"].get("push").is_none(), "{view}");
        assert_eq!(view["track"]["state"], "tracking", "{view}");
        rig.tick().await;
    }
    let (true_ra, true_dec) = true_place(&rig);
    assert!(
        separation(true_ra, true_dec, target.ra_deg, target.dec_deg) * 3600.0 <= 60.0,
        "the correction did not hold"
    );
    track(&rig, false).await;
}

#[tokio::test(start_paused = true)]
async fn a_push_in_two_steps_shows_moving_then_follows_without_pulling_back() {
    let rig = on_m42().await;
    let target = targets::find("M42").unwrap();
    let seq = rig.view().await["pointing"]["seq"].as_u64().unwrap();
    after_pointing(&rig, seq).await;
    let first = rig.now();
    rig.mount.push_by(0.0, -1.0);
    let mut views = Vec::new();
    let mut places = Vec::new();
    let mut syncs = Vec::new();
    let mut second = None;
    let view = loop {
        if second.is_none() && rig.now() >= first + 1.0 {
            second = Some(rig.now());
            rig.mount.push_by(0.0, -1.0);
        }
        let view = rig.view().await;
        views.push(view.clone());
        places.push(true_place(&rig));
        syncs.push(
            rig.state
                .session()
                .model(rig.now())
                .syncs
                .iter()
                .map(|s| s.time)
                .collect::<Vec<_>>(),
        );
        if view["track"]["push"]["state"].as_str() == Some("following") {
            break view;
        }
        assert!(rig.now() - first < 120.0, "never followed: {view}");
        rig.tick().await;
    };
    let second = second.expect("the second push happened");
    let at = view["track"]["push"]["at"].as_f64().unwrap();
    assert!(
        at >= second + 5.0,
        "followed {at} < 5 s after the second push {second}"
    );
    let (ra, dec) = push_place(&view);
    let (true_ra, true_dec) = true_place(&rig);
    assert!(
        separation(ra, dec, true_ra, true_dec) * 3600.0 <= 60.0,
        "followed place {ra} {dec} vs true place {true_ra} {true_dec}"
    );
    // Every view with a push before the follow was moving by hand with no place,
    // and every view before that kept Track tracking.
    let mut seen_push = false;
    let mut distance = None;
    for (view, (true_ra, true_dec)) in views.iter().zip(&places) {
        let push = view["track"].get("push");
        match push {
            Some(push) if push["state"].as_str() == Some("moving") => {
                assert!(
                    push.get("ra_deg").is_none(),
                    "a moving push carries a place: {push}"
                );
                seen_push = true;
            }
            Some(push) if push["state"].as_str() == Some("following") => {}
            _ => {
                assert!(
                    view["track"].get("push").is_none(),
                    "an unexpected push shape: {view}"
                );
                if !seen_push {
                    assert_eq!(view["track"]["state"], "tracking", "{view}");
                }
            }
        }
        let now = separation(*true_ra, *true_dec, target.ra_deg, target.dec_deg);
        if let Some(previous) = distance {
            assert!(
                previous - now <= 1.0 / 60.0,
                "the eyepiece was pulled back from {previous} to {now}"
            );
        }
        distance = Some(now);
    }
    assert!(seen_push, "the push was never moving by hand");
    // FR-006: no sync was recorded between the first push and the follow.
    for times in &syncs {
        assert!(
            !times.iter().any(|t| *t > first && *t < at),
            "a sync was recorded while moving by hand: {times:?}"
        );
    }
    track(&rig, false).await;
}

#[tokio::test(start_paused = true)]
async fn track_off_while_moving_clears_the_push_and_lets_syncs_in() {
    let rig = on_m42().await;
    rig.mount.push_by(0.0, -1.0);
    until(&rig, |view| {
        view["track"]["push"]["state"].as_str() == Some("moving")
    })
    .await;
    let off = rig.now();
    let (_, body) = write(&rig, "POST", "/api/track", json!({"on":false})).await;
    assert_eq!(body["track"]["state"], "off", "{body}");
    assert_eq!(body["track"]["ended_reason"], "stopped", "{body}");
    assert!(body["track"].get("push").is_none(), "{body}");
    let start = rig.now();
    loop {
        let syncs = &rig.state.session().model(rig.now()).syncs;
        assert!(
            rig.now() - start < 10.0,
            "no sync after Track off: {syncs:?}"
        );
        if syncs.iter().any(|s| s.time > off) {
            break;
        }
        rig.tick().await;
    }
    track(&rig, false).await;
}

#[tokio::test(start_paused = true)]
async fn a_fifteen_arcminute_push_opens_one_new_epoch_around_the_push() {
    let rig = on_m42().await;
    let before = rig.state.session().model(rig.now()).epochs.len();
    assert!(before >= 1, "the rig must have an epoch before the push");
    let seq = rig.view().await["pointing"]["seq"].as_u64().unwrap();
    after_pointing(&rig, seq).await;
    let pushed = rig.now();
    rig.mount.push_by(0.0, -0.25);
    until(&rig, |view| {
        view["track"]["push"]["state"].as_str() == Some("following")
    })
    .await;
    // The follow's own sync is timed at `at`; wait until the model holds one.
    let start = rig.now();
    let model = loop {
        let model = rig.state.session().model(rig.now());
        assert!(
            rig.now() - start < 120.0,
            "no sync timed after the push: {:?}",
            model.syncs
        );
        if model.syncs.iter().any(|s| s.time > pushed) {
            break model;
        }
        rig.tick().await;
    };
    assert_eq!(model.epochs.len(), before + 1, "{:?}", model.epochs);
    for sync in &model.syncs {
        if sync.epoch + 1 < model.epochs.len() {
            assert!(
                sync.time < pushed,
                "an old epoch's sync is timed after the push: {:?}",
                sync
            );
        } else {
            assert!(
                sync.time > pushed,
                "the new epoch's sync is timed before the push: {:?}",
                sync
            );
        }
    }
    track(&rig, false).await;
}

#[tokio::test(start_paused = true)]
async fn a_follow_below_the_minimum_altitude_ends_too_low_with_no_push() {
    let rig = on_m42().await;
    let target = targets::find("M42").unwrap();
    let (alt, _) =
        simulator::sky::horizontal(target.vector(), simulator::Site::default(), rig.now());
    let (_, body) = write(
        &rig,
        "PUT",
        "/api/session",
        json!({"min_alt_deg": alt - 1.0}),
    )
    .await;
    assert_eq!(body["min_alt_deg"], alt - 1.0, "{body}");
    rig.mount.push_by(0.0, -3.0);
    let view = until(&rig, |view| view["track"]["state"] == "off").await;
    assert_eq!(view["track"]["ended_reason"], "too_low", "{view}");
    assert!(view["track"].get("push").is_none(), "{view}");
    let start = rig.now();
    while !rig.stationary().await {
        assert!(rig.now() - start < 2.0, "the axes did not come to rest");
        rig.tick().await;
    }
    assert!(rig.now() - start <= 2.0);
}

#[tokio::test(start_paused = true)]
async fn checks_push_is_false_while_backlash_is_measured_and_true_after() {
    let target = targets::find("M81").unwrap().clone();
    let turn = tracker::reversal(&target, 0);
    let rig = tracker::Rig::new(turn - 600.0, target, finder::session::Session::default()).await;
    let watch = async {
        let mut readings = Vec::new();
        while rig.now() < turn + 600.0 {
            let measured = rig
                .body()
                .await
                .get("backlash")
                .and_then(|b| b.get("az_deg"))
                .is_some();
            readings.push((rig.tracker.state().checks_push, measured));
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        readings
    };
    let ((), readings) = tokio::join!(rig.follow(turn + 600.0, 60.0), watch);
    assert!(
        readings
            .iter()
            .any(|(checks, measured)| !checks && !measured),
        "no reading while the backlash was measured: {readings:?}"
    );
    let (checks, measured) = *readings.last().expect("the watch read at least once");
    assert!(
        measured,
        "the azimuth backlash was never stored: {readings:?}"
    );
    assert!(checks, "checks_push did not return after the measurement");
    rig.tracker.stop("stopped").await;
}

#[tokio::test(start_paused = true)]
async fn an_aim_change_disarms_the_push_check_and_is_corrected() {
    let rig = on_m42().await;
    let aim = rig.view().await["aim"].clone();
    let moved = Aim {
        x: aim["x"].as_f64().unwrap() + 40.0,
        y: aim["y"].as_f64().unwrap(),
    };
    rig.state.session().set_aim(Some(moved), rig.now()).unwrap();
    let start = rig.now();
    while rig.now() - start < 60.0 {
        let view = rig.view().await;
        assert!(view["track"].get("push").is_none(), "{view}");
        assert_eq!(view["track"]["state"], "tracking", "{view}");
        rig.tick().await;
    }
    track(&rig, false).await;
}
