//! US3-S3–S7: Done counts middle frames independently of the aim point's
//! outcome. Oracles: simulator pixels (1.5 px for aim, 2 px for centre), the
//! shared pinhole arithmetic (0.02°), and 032's exact outcome words.

use axum::http::StatusCode;
use serde_json::{json, Value};
use simulator::Optics;

use super::imaging_align::{assert_centre, call, last_after, ready, rig, start, wait_until, Setup};

fn assert_aim(session: &Value, x: f64, y: f64) {
    assert_eq!(session["aim"]["source"], "measured", "{session}");
    for (key, expected) in [("x", x), ("y", y)] {
        assert!(
            (session["aim"][key].as_f64().unwrap() - expected).abs() <= 1.5,
            "{session}"
        );
    }
}

#[tokio::test(start_paused = true)]
async fn two_middle_frames_are_done_and_only_the_solved_frame_gets_its_aim_word() {
    let rig = rig((700.0, 420.0), Setup::default());
    ready(&rig.app).await;
    start(&rig.app, json!({"exposure_ms": 1000})).await;
    let first = last_after(&rig.app, 0).await;
    assert_eq!(first["centre"]["middle"], true);
    assert_eq!(first["done"], false);
    assert_eq!(first["aim"], "measured");
    let second = last_after(&rig.app, first["frame_id"].as_u64().unwrap()).await;
    assert_eq!(second["done"], true);
    let (status, session) = call(&rig.app, "GET", "/api/session", json!(null)).await;
    assert_eq!(status, StatusCode::OK);
    assert_aim(&session, 700.0, 420.0);
    rig.sim.set_optics(Optics {
        eyepiece_px: (-1100.0, 420.0),
        ..Optics::default()
    });
    let view = wait_until(&rig.app, |v| v["align"]["last"]["outcome"] == "not_solved").await;
    assert_eq!(view["align"]["state"], "running");
    assert!(view["align"]["last"].get("aim").is_none(), "{view}");
    assert_eq!(view["align"]["last"]["done"], false);
}

#[tokio::test(start_paused = true)]
async fn done_resets_off_frame_and_does_not_wait_for_the_aim_point_to_be_replaced() {
    let rig = rig((700.0, 420.0), Setup::default());
    ready(&rig.app).await;
    start(&rig.app, json!({"exposure_ms": 1000})).await;
    let mut after = 0;
    for _ in 0..3 {
        after = last_after(&rig.app, after).await["frame_id"]
            .as_u64()
            .unwrap();
    }
    rig.sim.set_optics(Optics {
        eyepiece_px: (-300.0, 420.0),
        ..Optics::default()
    });
    let off = wait_until(&rig.app, |v| v["align"]["last"]["aim"] == "off_frame").await;
    assert_eq!(off["align"]["last"]["done"], false);
    after = off["align"]["last"]["frame_id"].as_u64().unwrap();
    rig.sim.set_optics(Optics {
        eyepiece_px: (900.0, 700.0),
        ..Optics::default()
    });
    let mut middle = Vec::new();
    for _ in 0..20 {
        let last = last_after(&rig.app, after).await;
        after = last["frame_id"].as_u64().unwrap();
        if last["centre"]["middle"] == true {
            middle.push((last["aim"].clone(), last["done"].clone()));
        }
        if last["aim"] == "replaced" {
            break;
        }
    }
    assert_eq!(
        middle,
        vec![
            (json!("set_aside"), json!(false)),
            (json!("set_aside"), json!(true)),
            (json!("replaced"), json!(true)),
        ]
    );
    let (_, session) = call(&rig.app, "GET", "/api/session", json!(null)).await;
    assert_aim(&session, 900.0, 700.0);
}

#[tokio::test(start_paused = true)]
async fn a_slow_finder_can_show_done_without_a_live_aim_reading() {
    let rig = rig(
        (700.0, 420.0),
        Setup {
            interval_ms: 3000,
            ..Setup::default()
        },
    );
    ready(&rig.app).await;
    start(&rig.app, json!({})).await;
    let mut after = 0;
    for _ in 0..20 {
        let last = last_after(&rig.app, after).await;
        after = last["frame_id"].as_u64().unwrap();
        if last["aim"] == "no_live_pointing" && last["done"] == true {
            assert_centre(&last, (700.0, 420.0), true);
            return;
        }
    }
    panic!("no measured frame had no_live_pointing and done within 20 frames");
}

#[tokio::test(start_paused = true)]
async fn a_measured_aim_does_not_displace_the_hint_from_the_paired_fix_centre() {
    let rig = rig((1300.0, 544.0), Setup::default());
    ready(&rig.app).await;
    start(&rig.app, json!({})).await;
    let mut after = 0;
    for _ in 0..3 {
        after = last_after(&rig.app, after).await["frame_id"]
            .as_u64()
            .unwrap();
    }
    let (_, session) = call(&rig.app, "GET", "/api/session", json!(null)).await;
    assert_eq!(session["aim"]["source"], "measured", "{session}");
    // This is 13° from the measured aim but only 8.4° from the finder centre.
    rig.sim.set_optics(Optics {
        eyepiece_px: (-300.0, 544.0),
        ..Optics::default()
    });
    for _ in 0..20 {
        let last = last_after(&rig.app, after).await;
        after = last["frame_id"].as_u64().unwrap();
        if last["centre"]["on_frame"] == false {
            assert_centre(&last, (-300.0, 544.0), false);
            return;
        }
    }
    panic!("the centre off the frame never measured within 20 frames");
}

#[tokio::test(start_paused = true)]
async fn start_refuses_an_unknown_field_height() {
    let rig = rig(
        (700.0, 420.0),
        Setup {
            focal_mm: None,
            ..Setup::default()
        },
    );
    ready(&rig.app).await;
    assert_eq!(
        call(&rig.app, "POST", "/api/imaging/align", json!({})).await,
        (
            StatusCode::CONFLICT,
            json!({"error": "the imaging camera's field height is unknown: start the console with --focal-length-mm, or solve an imaging frame on the Imaging page"})
        )
    );
}
