//! US4: the finder camera's return takes the aim point from the centred place.
//! Oracles: simulator imaging truth and true eyepiece pixel, M42's catalogue
//! place, and the session's aim and model before each operation. Angular
//! tolerances are arcseconds, pixel tolerance is Euclidean; time is paused.
use super::centre::{m42, sep, Cause, Rig, CENTRE, FINDER_ARCSEC_PER_PX};
use finder::session::Aim;
use serde_json::json;

/// Unlike `displace`, leave the finder frames running throughout: Track must
/// take the new aim before a live pointing corrects the centring away.
async fn aim_off(rig: &Rig) {
    let (x, y) = rig.sim.optics().eyepiece_px;
    let angle = 30.0_f64.to_radians();
    rig.state
        .session()
        .set_aim(
            Some(Aim {
                x: x + 240.0 * angle.cos() / FINDER_ARCSEC_PER_PX,
                y: y + 240.0 * angle.sin() / FINDER_ARCSEC_PER_PX,
            }),
            rig.now(),
        )
        .unwrap();
    rig.track_on().await;
    rig.sleep(30.0).await;
}

// S1: both above the push threshold with frames returning, and below it with
// frames never stopped. Neither a push nor a correction may undo the step.
#[tokio::test(start_paused = true)]
async fn finder_return_keeps_the_centring_and_leaves_the_aim_for_track_on() {
    for frames_running in [false, true] {
        let rig = Rig::new(17.0).await;
        if frames_running {
            aim_off(&rig).await;
        } else {
            rig.displace(Cause::Aim, 390.0, 30.0).await;
        }
        let before = rig.get("/api/session").await;
        rig.solve().await;
        rig.call("POST", CENTRE, json!({})).await;
        let rested = rig.rests().await;
        assert!(rested["track"]["push"].is_null(), "{rested}");
        rig.sim.start_frames();
        for _ in 0..30 {
            rig.sleep(1.0).await;
            let session = rig.get("/api/session").await;
            assert!(session["track"]["push"].is_null(), "{session}");
        }
        let after = rig.get("/api/session").await;
        assert!(rig.off(m42()) <= 25.0, "{} off M42", rig.off(m42()));
        assert!(sep(rig.held(), m42()) <= 0.01);
        assert_eq!(after["model"]["syncs"], before["model"]["syncs"]);
        assert_eq!(after["model"]["epoch"], before["model"]["epoch"]);
        assert!(!rig.state.session().model(rig.now()).slip_pending);
        assert_eq!(after["aim"]["source"], "measured");
        let (x, y) = rig.sim.optics().eyepiece_px;
        let dx = after["aim"]["x"].as_f64().unwrap() - x;
        let dy = after["aim"]["y"].as_f64().unwrap() - y;
        assert!(dx.hypot(dy) <= 1.0, "aim: {}", after["aim"]);

        rig.call("POST", "/api/track", json!({"on": false})).await;
        rig.sleep(2.0).await;
        rig.track_on().await;
        rig.sleep(30.0).await;
        assert!(
            rig.off(m42()) <= 25.0,
            "{} off after Track on",
            rig.off(m42())
        );
    }
}

// S2: a late return must leave the old aim exactly alone; a fresh centring
// gives the next steady live pointing a new chance to measure it.
#[tokio::test(start_paused = true)]
async fn an_expired_centring_does_not_take_the_aim_but_the_next_one_does() {
    let rig = Rig::new(17.0).await;
    rig.displace(Cause::Aim, 240.0, 30.0).await;
    let before = rig.get("/api/session").await;
    rig.solve().await;
    rig.call("POST", CENTRE, json!({})).await;
    rig.rests().await;
    rig.sleep(60.0).await;
    rig.sim.start_frames();
    rig.sleep(30.0).await;
    assert_eq!(rig.get("/api/session").await["aim"], before["aim"]);

    rig.solve().await;
    rig.call("POST", CENTRE, json!({})).await;
    let after = rig.rests().await;
    assert_eq!(after["aim"]["source"], "measured");
}

// S3: the imaging solve's held aim must be dropped when the centring supplies
// the aim, so Track off cannot apply that older reading over it.
#[tokio::test(start_paused = true)]
async fn track_off_keeps_the_centred_aim_instead_of_applying_the_held_reading() {
    let rig = Rig::new(17.0).await;
    aim_off(&rig).await;
    rig.solve().await;
    assert!(rig.state.session().held_aim().is_some());
    rig.call("POST", CENTRE, json!({})).await;
    rig.rests().await;
    rig.sleep(30.0).await;
    let before = rig.get("/api/session").await;
    rig.call("POST", "/api/track", json!({"on": false})).await;
    rig.sleep(2.0).await;
    assert_eq!(rig.get("/api/session").await["aim"], before["aim"]);
}
