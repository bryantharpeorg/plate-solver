//! Reproduce from the repository root: copy this file to
//! crates/finder/tests/us8_capture.rs, run
//! cargo test -p finder --locked --test us8_capture -- --nocapture,
//! then remove the temporary test file. No existing oracle is edited.
//! Uses US6's real solver / simulated node harness in paused virtual time.
#[path = "goto_api.rs"]
mod api;
use api::Rig;
use serde_json::{json, Value};
const START: f64 = 1_768_528_800.0;
fn save(name: &str, value: &Value) {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("web/src/test/fixtures");
    std::fs::write(
        dir.join(name),
        serde_json::to_string_pretty(value).unwrap() + "\n",
    )
    .unwrap();
}
async fn capture(rig: &Rig) -> Vec<Value> {
    let mut sessions = Vec::new();
    for _ in 0..5000 {
        let view = rig.view().await;
        if sessions.last().is_none_or(|last: &Value| {
            last["goto"]["state"] != view["goto"]["state"]
                || last["goto"]["correction"] != view["goto"]["correction"]
        }) {
            sessions.push(view.clone());
        }
        if matches!(
            view["goto"]["state"].as_str(),
            Some("done" | "failed" | "halted")
        ) {
            return sessions;
        }
        rig.tick().await;
    }
    panic!("capture did not finish");
}
#[tokio::test(start_paused = true)]
async fn record_sessions() {
    let rig = Rig::new(START, 40.0, 120.0, true).await;
    rig.sync().await;
    save("session-ready.json", &rig.view().await);
    assert_eq!(rig.call("/api/goto", false).await.0, 200);
    let sessions = capture(&rig).await;
    assert_eq!(sessions.last().unwrap()["goto"]["state"], "done");
    assert!(sessions
        .iter()
        .any(|s| s["goto"]["correction"].as_u64().unwrap() > 0));
    save("session-goto-m42.json", &json!(sessions));
    rig.target("M8");
    save("session-below-horizon.json", &rig.view().await);
    rig.site_for_altitude("M42", 15.2);
    save("session-low-approach.json", &rig.view().await);
    rig.site_for_altitude("M42", 50.0);
    rig.state.session().reset_model(false, rig.now()).unwrap();
    save("session-no-sync.json", &rig.view().await);

    let sun = Rig::new(START, 40.0, 120.0, true).await;
    sun.sync().await;
    let (ra, dec) = finder::ephemeris::sun(finder::sky::julian_date(sun.now()));
    let lon = (ra - finder::sky::gmst_deg(finder::sky::julian_date(sun.now())) + 180.0)
        .rem_euclid(360.0)
        - 180.0;
    sun.state
        .session()
        .edit(
            serde_json::from_value(
                json!({"site":{"lat_deg":dec+85.0,"lon_deg":lon},"target":"M31"}),
            )
            .unwrap(),
            sun.now(),
        )
        .unwrap();
    save("session-sun-up.json", &sun.view().await);

    // Independent inverse Rodrigues rotation puts the first sync at azimuth
    // 120°, altitude 50°, using the upward backlash flank (US7's fixture).
    let (a, e, t) = (
        120_f64.to_radians(),
        50_f64.to_radians(),
        130_f64.to_radians(),
    );
    let v = [e.cos() * a.cos(), e.cos() * a.sin(), e.sin()];
    let k = [-t.sin(), t.cos(), 0.0];
    let (s, c) = (-1_f64).to_radians().sin_cos();
    let dot = k[0] * v[0] + k[1] * v[1];
    let cross = [k[1] * v[2], -k[0] * v[2], k[0] * v[1] - k[1] * v[0]];
    let v = [0, 1, 2].map(|i| v[i] * c + cross[i] * s + k[i] * dot * (1.0 - c));
    let second = Rig::new(
        START,
        v[2].asin().to_degrees() - 0.85,
        v[1].atan2(v[0]).to_degrees() - 0.85,
        true,
    )
    .await;
    second.move_to([4500, 4500]).await;
    for _ in 0..15 {
        second.tick().await;
    }
    second
        .state
        .session()
        .reset_model(false, second.now())
        .unwrap();
    second.sync().await;
    assert_eq!(second.call("/api/model/second-sync", false).await.0, 200);
    save("session-second-sync-slewing.json", &second.view().await);
    let states = capture(&second).await;
    assert_eq!(states.last().unwrap()["goto"]["state"], "done");
    let view = second.view().await;
    assert_eq!(view["model"]["syncs"], 2);
    save("session-two-syncs.json", &view);
}
