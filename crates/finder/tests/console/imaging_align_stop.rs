//! US3-S1/S2: exact settings, the simulator's mount truth and the files on
//! disk are the oracles. Stopping an exposure takes at most 5 s of paused time.

use std::time::Duration;

use axum::http::StatusCode;
use serde_json::json;

use super::imaging_align::{call, last_after, ready, rig, start, wait_until, Setup};

#[tokio::test(start_paused = true)]
async fn stop_restores_settings_keeps_only_the_last_frame_and_releases_the_camera() {
    let scratch = crate::support::Scratch::new("imaging-align-stop");
    let folder = scratch.0.join("imaging");
    let rig = rig(
        (700.0, 420.0),
        Setup {
            folder: Some(folder.clone()),
            ..Setup::default()
        },
    );
    ready(&rig.app).await;
    let (status, view) = call(&rig.app, "DELETE", "/api/imaging/align", json!(null)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(view.get("align").is_none(), "{view}");
    let (status, settings) = call(
        &rig.app,
        "PUT",
        "/api/imaging/settings",
        json!({"exposure_ms": 2000,
            "roi": {"x": 0, "y": 0, "width": 618, "height": 410},
            "save_at_node": true}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{settings}");
    let mount = rig.sim.mount().unwrap();
    let direction = mount.true_direction();
    start(&rig.app, json!({})).await;
    let mut after = 0;
    for _ in 0..3 {
        after = last_after(&rig.app, after).await["frame_id"]
            .as_u64()
            .unwrap();
    }
    let (status, view) = call(&rig.app, "DELETE", "/api/imaging/align", json!(null)).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    assert_eq!(view["align"]["state"], "ended");
    assert_eq!(view["align"]["ended"], "stopped");
    assert!(view["align"].get("error").is_none(), "{view}");
    assert_eq!(view["settings"], settings["settings"]);
    assert_eq!(mount.true_direction(), direction);
    let files: Vec<_> = std::fs::read_dir(folder.join("frames"))
        .unwrap()
        .flat_map(|date| std::fs::read_dir(date.unwrap().path()).unwrap())
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(files.len(), 2, "{files:?}");
    let fits = files
        .iter()
        .find(|file| file.extension().is_some_and(|ext| ext == "fits"))
        .expect("the last FITS frame");
    assert!(
        files.contains(&fits.with_extension("solve.json")),
        "{files:?}"
    );
    assert_eq!(
        call(&rig.app, "POST", "/api/imaging/frame", json!(null))
            .await
            .0,
        StatusCode::OK
    );
}

#[tokio::test(start_paused = true)]
async fn stop_aborts_a_long_exposure_before_it_finishes() {
    let rig = rig((700.0, 420.0), Setup::default());
    ready(&rig.app).await;
    start(&rig.app, json!({"exposure_ms": 60000})).await;
    wait_until(&rig.app, |v| {
        v["frame"]["purpose"] == "align" && v["frame"]["state"] == "exposing"
    })
    .await;
    let (status, view) = tokio::time::timeout(
        Duration::from_secs(5),
        call(&rig.app, "DELETE", "/api/imaging/align", json!(null)),
    )
    .await
    .expect("stop must abort rather than wait for the 60 s exposure");
    assert_eq!(status, StatusCode::OK, "{view}");
    assert_eq!(view["align"]["ended"], "stopped");
    assert_eq!(view["align"]["frames"], 0);
    assert_eq!(view["frame"]["state"], "aborted");
}
