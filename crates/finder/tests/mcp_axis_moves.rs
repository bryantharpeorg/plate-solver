//! US5 oracles: simulator positions (exact), its 3 deg/s² acceleration law
//! (0.1 deg), command timestamps (0.1 s), and independently transformed solar
//! coordinates (1e-6 deg). Every running motion is sampled at 0.1 s.
#[path = "mcp/support.rs"]
mod support;

use axum::{body::Body, http::Request, Router};
use finder::{ephemeris, model::separation, sky};
use scope_node::mount::{Axis, Mount};
use serde_json::{json, Value};
use std::{
    io::Write,
    sync::{Arc, Mutex},
};
use support::{
    sim::{MountCommand, Rig, START},
    *,
};

const PATH: &str = "no path keeps clear of the Sun guard, minimum altitude and cable wind";
const LIMITED: &str = "the rate limit of 2 moving calls a second was reached";
#[derive(Clone, Default)]
struct Log(Arc<Mutex<Vec<u8>>>);
impl Write for Log {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl Log {
    fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}
async fn tool(app: &Router, name: &str, args: Value) -> Value {
    let response = send(app, call(name, args)).await;
    assert_eq!(response.status(), 200);
    let rpc = body(response).await;
    assert!(rpc.get("error").is_none(), "{rpc}");
    let result = rpc["result"].clone();
    assert!(!result.to_string().contains("test-token"));
    result
}
fn accepted(result: Value) {
    assert_ne!(result["isError"], true, "{result}");
    assert_eq!(result["structuredContent"], json!({"started":true}));
}
fn error(result: Value, reason: &str) {
    assert_eq!(result["isError"], true, "{result}");
    assert_eq!(result["content"][0]["text"], reason, "{result}");
}
async fn page(rig: &Rig, path: &str, args: Value) -> Value {
    let response = send(
        &rig.app,
        Request::post(path)
            .header("host", "localhost:8080")
            .header("content-type", "application/json")
            .body(Body::from(args.to_string()))
            .unwrap(),
    )
    .await;
    assert_eq!(response.status(), 200);
    body(response).await
}
fn edit(rig: &Rig, args: Value) {
    rig.state
        .session()
        .edit(serde_json::from_value(args).unwrap(), rig.now())
        .unwrap();
}
async fn ticks(rig: &Rig, n: usize) {
    for _ in 0..n {
        rig.tick().await;
    }
}
async fn positions(rig: &Rig) -> [i32; 2] {
    [
        rig.mount.state(Axis::Azimuth).await.position,
        rig.mount.state(Axis::Altitude).await.position,
    ]
}
async fn quiet(rig: &Rig, before: [i32; 2]) {
    for _ in 0..10 {
        rig.tick().await;
        assert_eq!(positions(rig).await, before);
        for axis in [Axis::Azimuth, Axis::Altitude] {
            assert!(!rig.mount.state(axis).await.moving);
        }
    }
}
async fn ready() -> Rig {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    rig.sync().await;
    assert_eq!(session(&rig.app).await["mount"]["connected"], true);
    rig
}
fn rate(axis: &str, speed: f64, duration: f64) -> Value {
    json!({"axis":axis,"deg_per_s":speed,"duration_s":duration})
}
fn position(axis: &str, counts: i32) -> Value {
    json!({"axis":axis,"position":counts})
}
async fn control(rig: &Rig) {
    accepted(tool(&rig.app, "move_axis", rate("altitude", 0.5, 1.0)).await);
    let before = positions(rig).await;
    ticks(rig, 5).await;
    assert!(positions(rig).await[1] > before[1]);
    page(rig, "/api/stop", json!({})).await;
    ticks(rig, 20).await;
}
async fn finish(rig: &Rig, seconds: usize) {
    for _ in 0..seconds * 10 {
        rig.tick().await;
        if session(&rig.app).await["axis_move"]["state"] != "moving" {
            return;
        }
    }
    panic!("move did not end: {}", session(&rig.app).await);
}
fn stop_time(rig: &Rig, after: f64) -> f64 {
    rig.commands
        .lock()
        .unwrap()
        .iter()
        .find(|c| c.at >= after && c.action == MountCommand::StopAll)
        .unwrap()
        .at
}
// The simulator's horizon transform is independent of GoTo's Sun guard.
fn sun(time: f64) -> (f64, f64) {
    let jd = sky::julian_date(time);
    let (ra, dec) = ephemeris::sun(jd);
    let date = math_core::UnitVector::from_radec(ra.to_radians(), dec.to_radians());
    simulator::sky::horizontal(
        simulator::sky::to_j2000(date, jd),
        simulator::Site::default(),
        time,
    )
}
fn crossing(alt: f64, mut low: f64, mut high: f64, rising: bool) -> f64 {
    for _ in 0..45 {
        let mid = (low + high) / 2.0;
        if (sun(mid).0 < alt) == rising {
            low = mid;
        } else {
            high = mid;
        }
    }
    (low + high) / 2.0
}
async fn low(start: f64, az: f64, min: f64) -> Rig {
    let rig = Rig::new(start, 3.0, az, true).await;
    rig.mount.push_to(3.0, az);
    rig.sync().await;
    // Isolate the boundary geometry from the first solve's residual: this
    // qualified sync uses the simulator's independent eyepiece and counts.
    rig.controls.stop_frames();
    ticks(&rig, 10).await;
    rig.state.session().reset_model(false, rig.now()).unwrap();
    let truth = rig.mount.true_direction();
    assert!(rig
        .state
        .session()
        .record_sync(finder::model::Sync {
            time: rig.now(),
            axis: positions(&rig).await.map(f64::from),
            counts_per_revolution: rig.mount.axes().map(|a| a.counts_per_revolution),
            az_deg: truth.az,
            alt_deg: ephemeris::apparent_from_true(truth.alt),
            roll_deg: None,
            rms_arcsec: 1.0,
            epoch: 0,
        })
        .unwrap());
    edit(&rig, json!({"min_alt_deg":min}));
    rig
}

#[tokio::test(start_paused = true)]
async fn sun_five_degrees_up_refuses_all_three_tools_after_a_positive_move() {
    let rig = ready().await;
    control(&rig).await;
    rig.controls.stop_frames();
    ticks(&rig, 10).await;
    let (ra, dec) = ephemeris::sun(sky::julian_date(rig.now()));
    let lon = (ra - sky::gmst_deg(sky::julian_date(rig.now())) + 180.0).rem_euclid(360.0) - 180.0;
    edit(&rig, json!({"site":{"lat_deg":dec+85.0,"lon_deg":lon}}));
    let date = math_core::UnitVector::from_radec(ra.to_radians(), dec.to_radians());
    let truth = simulator::sky::horizontal(
        simulator::sky::to_j2000(date, sky::julian_date(rig.now())),
        simulator::Site {
            lat_deg: dec + 85.0,
            lon_deg: lon,
        },
        rig.now(),
    );
    assert!((truth.0 - 5.0).abs() < 1e-6);
    let before = positions(&rig).await;
    for (name, args) in [
        ("goto_target", json!({})),
        ("goto_axis_position", position("azimuth", 10000)),
        ("move_axis", rate("altitude", 1.0, 5.0)),
    ] {
        error(tool(&rig.app, name, args).await, "Sun is up");
        quiet(&rig, before).await;
    }
}

#[tokio::test(start_paused = true)]
async fn dusk_position_and_top_rate_path_refuse_the_sun_but_safe_turn_runs() {
    let start = crossing(-10.0, START - 8.0 * 3600.0, START, false);
    assert!((sun(start).0 + 10.0).abs() < 1e-6);
    let az = sun(start).1;
    let rig = low(start, az - 40.0, 0.0).await;
    let model = rig.state.session().model(rig.now());
    let counts = rig.mount.axes().map(|a| a.counts_per_revolution);
    let current = positions(&rig).await;
    let p = model.predict(current.map(f64::from), counts).unwrap();
    // Choose the adjacent encoder revolution, so this tests the Sun rather
    // than accidentally requesting a full extra turn through the cable rule.
    let nearest = |value: f64| {
        let turn = f64::from(counts[0]);
        (value + ((f64::from(current[0]) - value) / turn).round() * turn).round() as i32
    };
    let on_sun = nearest(model.inverse(sun(rig.now()).1, p.alt_deg, counts).unwrap()[0]);
    let endpoint = model
        .predict([f64::from(on_sun), f64::from(current[1])], counts)
        .unwrap();
    assert!(
        (separation(
            endpoint.az_deg,
            ephemeris::true_from_apparent(endpoint.alt_deg),
            sun(rig.now()).1,
            sun(rig.now()).0
        ) - 13.0)
            .abs()
            < 0.1,
        "endpoint: {endpoint:?}, sun: {:?}, true: {:?}",
        sun(rig.now()),
        rig.mount.true_direction()
    );
    assert!((separation(sun(rig.now()).1, 3.0, sun(rig.now()).1, -10.0) - 13.0).abs() < 1e-9);
    error(
        tool(&rig.app, "goto_axis_position", position("azimuth", on_sun)).await,
        PATH,
    );
    quiet(&rig, current).await;
    error(
        tool(
            &rig.app,
            "move_axis",
            rate("azimuth", rig.mount.axes()[0].top_rate_deg_per_s, 30.0),
        )
        .await,
        PATH,
    );
    quiet(&rig, current).await;
    let safe = nearest(model.inverse(az - 90.0, p.alt_deg, counts).unwrap()[0]);
    accepted(tool(&rig.app, "goto_axis_position", position("azimuth", safe)).await);
    finish(&rig, 40).await;
    assert_eq!(rig.mount.state(Axis::Azimuth).await.position, safe);
    assert_eq!(session(&rig.app).await["axis_move"]["state"], "done");
    assert!(session(&rig.app).await.get("motion").is_none());
}

#[tokio::test(start_paused = true)]
async fn below_minimum_destination_and_unsynced_rate_are_refused() {
    let rig = ready().await;
    control(&rig).await;
    let model = rig.state.session().model(rig.now());
    let counts = rig.mount.axes().map(|a| a.counts_per_revolution);
    let before = positions(&rig).await;
    let p = model.predict(before.map(f64::from), counts).unwrap();
    let low = model
        .inverse(p.az_deg, ephemeris::apparent_from_true(10.0), counts)
        .unwrap()[1]
        .round() as i32;
    error(
        tool(&rig.app, "goto_axis_position", position("altitude", low)).await,
        "the position is below the minimum altitude",
    );
    quiet(&rig, before).await;
    rig.controls.stop_frames();
    ticks(&rig, 10).await;
    rig.state.session().reset_model(false, rig.now()).unwrap();
    error(
        tool(&rig.app, "move_axis", rate("altitude", 1.0, 5.0)).await,
        "the night has no sync yet",
    );
    quiet(&rig, before).await;
}

#[tokio::test(start_paused = true)]
async fn below_minimum_only_altitude_can_climb_and_duration_is_twenty_seconds() {
    let rig = low(START, 120.0, 15.0).await;
    let before = positions(&rig).await;
    error(
        tool(&rig.app, "move_axis", rate("azimuth", 1.0, 20.0)).await,
        PATH,
    );
    quiet(&rig, before).await;
    let start = rig.now();
    accepted(tool(&rig.app, "move_axis", rate("altitude", 1.0, 20.0)).await);
    let view = session(&rig.app).await;
    assert_eq!(
        view["axis_move"],
        json!({"tool":"move_axis","state":"moving"})
    );
    assert_eq!(view["motion"]["by"], "mcp");
    assert_eq!(view["motion"]["tool"], "move_axis");
    finish(&rig, 22).await;
    assert!((stop_time(&rig, start) - start - 20.0).abs() <= 0.1);
    assert!(rig.mount.true_direction().alt > 21.0);
}

#[tokio::test(start_paused = true)]
async fn cable_wind_refuses_one_point_two_turns_and_accepts_point_eight() {
    let rig = ready().await;
    let before = positions(&rig).await;
    let turn = rig.mount.axes()[0].counts_per_revolution as f64;
    error(
        tool(
            &rig.app,
            "goto_axis_position",
            position("azimuth", (1.2 * turn) as i32),
        )
        .await,
        PATH,
    );
    quiet(&rig, before).await;
    let destination = (0.8 * turn) as i32;
    accepted(
        tool(
            &rig.app,
            "goto_axis_position",
            position("azimuth", destination),
        )
        .await,
    );
    finish(&rig, 110).await;
    assert_eq!(rig.mount.state(Axis::Azimuth).await.position, destination);
}

#[tokio::test(start_paused = true)]
async fn rate_duration_and_acceleration_ramp_match_the_simulator_and_bad_duration_moves_nothing() {
    let rig = ready().await;
    let before = positions(&rig).await;
    let start = rig.now();
    accepted(tool(&rig.app, "move_axis", rate("altitude", 0.5, 10.0)).await);
    while rig.now() < start + 10.0 {
        rig.tick().await;
    }
    let degrees = f64::from(positions(&rig).await[1] - before[1]) * 360.0
        / f64::from(rig.mount.axes()[1].counts_per_revolution);
    // Acceleration ramp loses v²/(2a) relative to a step to full rate.
    assert!(
        (degrees - (5.0 - 0.5 * 0.5 / (2.0 * 3.0))).abs() <= 0.1,
        "{degrees}"
    );
    finish(&rig, 2).await;
    assert!((stop_time(&rig, start) - start - 10.0).abs() <= 0.1);
    ticks(&rig, 10).await;
    let before = positions(&rig).await;
    error(
        tool(&rig.app, "move_axis", rate("altitude", 0.5, 60.0)).await,
        "duration_s must be more than 0 and at most 30",
    );
    quiet(&rig, before).await;
}

#[tokio::test(start_paused = true)]
async fn disconnected_client_does_not_end_heartbeats_or_leave_a_rate_running() {
    let rig = ready().await;
    let client = app(rig.state.clone(), "192.168.1.30");
    let link = rig.state.link().unwrap();
    let heartbeats = link.status().borrow().heartbeats;
    let start = rig.now();
    accepted(tool(&client, "move_axis", rate("azimuth", 0.5, 20.0)).await);
    drop(client);
    for _ in 0..210 {
        rig.tick().await;
        let status = link.status().borrow().clone();
        assert!(status.last_heartbeat_at.unwrap().elapsed().as_secs_f64() <= 1.0);
    }
    assert!(link.status().borrow().heartbeats > heartbeats);
    assert!((stop_time(&rig, start) - start - 20.0).abs() <= 0.1);
    assert!(!rig.mount.state(Axis::Azimuth).await.moving);
    assert_eq!(session(&rig.app).await["axis_move"]["state"], "done");
}

#[tokio::test(start_paused = true)]
async fn sunrise_halts_with_the_page_guard_reason_within_one_second() {
    let sunrise = crossing(-0.83, START, START + 12.0 * 3600.0, true);
    let rig = Rig::new(sunrise - 12.0, 50.0, 120.0, true).await;
    rig.sync().await;
    assert!(rig.now() < sunrise);
    let start = rig.now();
    accepted(tool(&rig.app, "move_axis", rate("altitude", 0.5, 30.0)).await);
    finish(&rig, 32).await;
    let stopped = stop_time(&rig, start);
    assert!(
        (sunrise..=sunrise + 1.0).contains(&stopped),
        "{}",
        stopped - sunrise
    );
    assert_eq!(
        session(&rig.app).await["axis_move"],
        json!({"tool":"move_axis","state":"halted","reason":"halted by the Sun guard"})
    );
}

#[tokio::test(start_paused = true)]
async fn ten_calls_share_two_slots_across_clients_and_stop_is_unlimited() {
    let rig = ready().await;
    let second = app(rig.state.clone(), "192.168.1.30");
    rig.commands.lock().unwrap().clear();
    accepted(tool(&rig.app, "move_axis", rate("altitude", 0.5, 10.0)).await);
    for i in 1..10 {
        error(
            tool(
                if i % 2 == 0 { &rig.app } else { &second },
                "move_axis",
                rate("altitude", 0.5, 10.0),
            )
            .await,
            if i == 1 {
                "the mount is already moving"
            } else {
                LIMITED
            },
        );
    }
    let result = tool(&second, "stop_all", json!({})).await;
    assert_eq!(result["structuredContent"], json!({"stopped":true}));
    ticks(&rig, 10).await;
    let commands = rig.commands.lock().unwrap().clone();
    assert_eq!(
        commands
            .iter()
            .filter(|c| matches!(c.action, MountCommand::Rate { .. }))
            .count(),
        1
    );
    assert_eq!(
        commands
            .iter()
            .filter(|c| matches!(c.action, MountCommand::GoTo { .. }))
            .count(),
        0
    );
    assert!(commands.iter().any(|c| c.action == MountCommand::StopAll));
    for axis in [Axis::Azimuth, Axis::Altitude] {
        assert!(!rig.mount.state(axis).await.moving);
    }
}

#[tokio::test(start_paused = true)]
async fn page_stop_halts_and_goto_cannot_overlap_a_direct_move() {
    let rig = ready().await;
    accepted(tool(&rig.app, "move_axis", rate("altitude", 0.5, 10.0)).await);
    error(
        tool(&rig.app, "goto_target", json!({})).await,
        "the mount is already moving",
    );
    assert!(!rig
        .commands
        .lock()
        .unwrap()
        .iter()
        .any(|c| matches!(c.action, MountCommand::GoTo { .. })));
    ticks(&rig, 5).await;
    let stop = rig.now();
    page(&rig, "/api/stop", json!({})).await;
    rig.tick().await;
    assert!(stop_time(&rig, stop) - stop <= 0.1);
    assert_eq!(
        session(&rig.app).await["axis_move"],
        json!({"tool":"move_axis","state":"halted","reason":"the observer stopped it"})
    );
}

#[tokio::test(start_paused = true)]
async fn direct_tools_are_listed_describe_real_motion_and_never_leak_the_token() {
    let rig = ready().await;
    let log = Log::default();
    let client = app(rig.state.clone().with_call_log(log.clone()), "127.0.0.1");
    let listing = body(
        send(
            &client,
            request(json!({"jsonrpc":"2.0","id":7,"method":"tools/list"})),
        )
        .await,
    )
    .await;
    for name in ["goto_axis_position", "move_axis"] {
        let entry = listing["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["name"] == name)
            .unwrap();
        assert!(entry["description"]
            .as_str()
            .unwrap()
            .contains("real telescope"));
        assert_eq!(entry["outputSchema"]["type"], "object");
    }
    accepted(tool(&client, "goto_axis_position", position("azimuth", 4500)).await);
    finish(&rig, 3).await;
    ticks(&rig, 10).await;
    accepted(tool(&client, "move_axis", rate("altitude", 0.5, 1.0)).await);
    finish(&rig, 3).await;
    assert_eq!(log.text().lines().count(), 2);
    assert!(log.text().contains("mcp goto_axis_position"));
    assert!(log.text().contains("mcp move_axis"));
    assert!(!log.text().contains("test-token"));
}

#[tokio::test(start_paused = true)]
async fn track_through_the_page_blocks_all_three_mcp_moves_and_keeps_tracking() {
    let rig = ready().await;
    page(&rig, "/api/track", json!({"on":true})).await;
    let mut tracking = false;
    for _ in 0..2000 {
        if session(&rig.app).await["track"]["state"] == "tracking" {
            tracking = true;
            break;
        }
        rig.tick().await;
    }
    assert!(tracking, "{}", session(&rig.app).await);
    rig.commands.lock().unwrap().clear();
    for (name, args) in [
        ("move_axis", rate("altitude", 1.0, 5.0)),
        ("goto_axis_position", position("azimuth", 4500)),
        ("goto_target", json!({})),
    ] {
        error(tool(&rig.app, name, args).await, "Track is on");
        ticks(&rig, 10).await;
    }
    ticks(&rig, 50).await;
    assert_eq!(session(&rig.app).await["track"]["state"], "tracking");
    let commands = rig.commands.lock().unwrap();
    assert!(!commands
        .iter()
        .any(|c| matches!(c.action, MountCommand::GoTo { .. })));
    assert!(!commands
        .iter()
        .any(|c| matches!(c.action, MountCommand::Rate {deg_per_s, ..} if deg_per_s == 1.0)));
}

#[tokio::test(start_paused = true)]
async fn changed_minimum_reforecasts_and_brakes_before_the_limit() {
    let rig = ready().await;
    accepted(tool(&rig.app, "move_axis", rate("altitude", -1.0, 20.0)).await);
    ticks(&rig, 20).await;
    let minimum = rig.mount.true_direction().alt - 1.0;
    edit(&rig, json!({"min_alt_deg":minimum}));
    let changed = rig.now();
    for _ in 0..20 {
        rig.tick().await;
        assert!(rig.mount.true_direction().alt >= minimum);
    }
    assert!(stop_time(&rig, changed) - changed <= 0.1);
    assert!(!rig.mount.state(Axis::Altitude).await.moving);
    assert_eq!(
        session(&rig.app).await["axis_move"],
        json!({"tool":"move_axis","state":"halted","reason":"the position is below the minimum altitude"})
    );
}

#[tokio::test(start_paused = true)]
async fn signed_rate_is_capped_and_duration_boundary_refusals_do_not_move() {
    let rig = ready().await;
    accepted(tool(&rig.app, "move_axis", rate("azimuth", -99.0, 1.0)).await);
    assert!(rig.commands.lock().unwrap().iter().any(|c| c.action
        == MountCommand::Rate {
            axis: Axis::Azimuth,
            deg_per_s: -rig.mount.axes()[0].top_rate_deg_per_s,
        }));
    finish(&rig, 3).await;
    ticks(&rig, 20).await;
    assert!(rig.mount.state(Axis::Azimuth).await.position < 0);
    for duration in [0.0, -1.0, 30.1] {
        let before = positions(&rig).await;
        error(
            tool(&rig.app, "move_axis", rate("altitude", 0.5, duration)).await,
            "duration_s must be more than 0 and at most 30",
        );
        quiet(&rig, before).await;
    }
}

#[tokio::test(start_paused = true)]
async fn braking_distance_is_checked_at_the_cable_limit() {
    let rig = ready().await;
    let turn = f64::from(rig.mount.axes()[0].counts_per_revolution);
    accepted(
        tool(
            &rig.app,
            "goto_axis_position",
            position("azimuth", (354.0 / 360.0 * turn) as i32),
        )
        .await,
    );
    finish(&rig, 110).await;
    let before = positions(&rig).await;
    // Cruise alone ends at 359.4 deg; braking adds v²/(2a) = 1.5 deg.
    let degrees = f64::from(before[0]) * 360.0 / turn;
    assert!(degrees + 3.0 * 1.8 < 360.0);
    assert!(degrees + 3.0 * 1.8 + 3.0 * 3.0 / (2.0 * 3.0) > 360.0);
    error(
        tool(&rig.app, "move_axis", rate("azimuth", 3.0, 1.8)).await,
        PATH,
    );
    quiet(&rig, before).await;
}

#[tokio::test(start_paused = true)]
async fn direct_move_state_is_absent_until_first_move_and_never_persisted() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../target/us5-axis-session-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("session.json");
    std::fs::write(
        &path,
        json!({"site":{"lat_deg":40.0,"lon_deg":-75.0},"min_alt_deg":15.0,"target":"NGC 1976"})
            .to_string(),
    )
    .unwrap();
    let rig = support::sim::persistent(&path).await;
    rig.sync().await;
    assert!(session(&rig.app).await.get("axis_move").is_none());
    accepted(tool(&rig.app, "move_axis", rate("altitude", 0.5, 1.0)).await);
    for _ in 0..30 {
        rig.tick().await;
        let stored: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert!(stored.get("axis_move").is_none());
        assert!(stored.get("motion").is_none());
    }
    assert_eq!(session(&rig.app).await["axis_move"]["state"], "done");
    drop(rig);
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test(start_paused = true)]
async fn a_zero_rate_still_reserves_motion_until_its_duration_ends() {
    let rig = ready().await;
    accepted(tool(&rig.app, "move_axis", rate("altitude", 0.0, 5.0)).await);
    error(
        tool(&rig.app, "goto_target", json!({})).await,
        "the mount is already moving",
    );
    assert!(!rig
        .commands
        .lock()
        .unwrap()
        .iter()
        .any(|c| matches!(c.action, MountCommand::GoTo { .. })));
    finish(&rig, 6).await;
}
