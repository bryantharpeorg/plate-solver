//! Oracles: simulator horizontal fields (0.1 degree), the databases' built ranges,
//! and US2's exact database, reason and three-failure sequences. Time is paused.

use super::{databases, MID, NARROW, WIDE};
use crate::support::live::{self as support, event, request, ConsoleClock, DB, START};
use axum::{http::StatusCode, Router};
use finder::{live::LiveReport, samples::Samples, session::Session, solve::Status, AppState};
use pattern_database::PatternDatabase;
use serde_json::{json, Value};
use simulator::{Optics, SimNode, Simulator};
use std::{sync::Arc, time::Duration};
use tokio::sync::broadcast;

fn set_field(sim: &Simulator, fov_deg: f64) {
    sim.set_optics(Optics {
        fov_deg,
        ..Default::default()
    });
}

fn rig(
    fov_deg: f64,
    list: &[(&str, &Arc<PatternDatabase>)],
) -> (Simulator, Router, broadcast::Receiver<LiveReport>) {
    let sim = support::sim();
    set_field(&sim, fov_deg);
    let session = Session::default();
    session
        .edit(
            serde_json::from_value(json!({
                "node": {"address": "scope.local:50051", "token": "test-token"},
                "camera_settings": {"exposure_ms": 200, "gain": 1}
            }))
            .unwrap(),
            START,
        )
        .unwrap();
    let node = Arc::new(
        SimNode::new(sim.clone(), "test-token", Default::default())
            .unwrap()
            .with_frame_interval(Duration::from_millis(500)),
    );
    let state = AppState::shared(DB.clone(), Samples::default())
        .with_databases(databases(list))
        .with_session(session)
        .with_clock(Arc::new(ConsoleClock(sim.clock().clone())))
        .with_link(support::transport(node))
        .with_live();
    let events = state.live().unwrap().subscribe();
    (sim, finder::router(state), events)
}

async fn next(app: &Router, events: &mut broadcast::Receiver<LiveReport>) -> (LiveReport, Value) {
    let report = event(events).await;
    let (status, session) = request(app, "GET", "/api/session", Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    (report, session)
}

fn solved(report: &LiveReport, session: &Value, database: &str, fov_deg: f64) {
    assert_eq!(report.report.status, Status::MatchFound, "{report:?}");
    assert_eq!(report.report.database.as_deref(), Some(database));
    let actual = report.report.pointing.as_ref().unwrap().fov_deg;
    assert!((actual - fov_deg).abs() < 0.1, "{actual} vs {fov_deg}");
    let actual = session["field"]["fov_deg"].as_f64().unwrap();
    assert!((actual - fov_deg).abs() < 0.1, "{actual} vs {fov_deg}");
    assert_eq!(session["pointing"]["frame_id"], report.frame_id);
}

fn in_use(session: &Value, name: &str, min: f64, max: f64, reason: &str) {
    assert_eq!(
        session["field"]["database"],
        json!({"name": name, "min_fov_deg": min, "max_fov_deg": max, "reason": reason})
    );
    assert_eq!(session["field"]["db_min_fov_deg"], min);
    assert_eq!(session["field"]["db_max_fov_deg"], max);
}

#[tokio::test(start_paused = true)]
async fn one_database_is_named_after_its_catalog_with_reason_only() {
    let sim = support::sim();
    set_field(&sim, 12.0);
    let state = support::state(&sim, None, false);
    let mut events = state.live().unwrap().subscribe();
    let app = finder::router(state);
    let (report, session) = next(&app, &mut events).await;
    solved(&report, &session, "hip_main", 12.0);
    in_use(&session, "hip_main", 10.0, 30.0, "only");
    assert_eq!(
        session["field"]["databases"],
        json!([{"name": "hip_main", "min_fov_deg": 10.0, "max_fov_deg": 30.0}])
    );
}

#[tokio::test(start_paused = true)]
async fn twelve_degrees_uses_cedar_and_lists_every_range_in_given_order() {
    let (_sim, app, mut events) = rig(12.0, &[("cedar", &DB), ("narrow", &NARROW)]);
    let (report, session) = next(&app, &mut events).await;
    solved(&report, &session, "cedar", 12.0);
    in_use(&session, "cedar", 10.0, 30.0, "field");
    assert_eq!(
        session["field"]["databases"],
        json!([
            {"name": "cedar", "min_fov_deg": 10.0, "max_fov_deg": 30.0},
            {"name": "narrow", "min_fov_deg": 4.5, "max_fov_deg": 7.0}
        ])
    );
}

#[tokio::test(start_paused = true)]
async fn five_degrees_tries_narrow_and_uses_its_range() {
    let (_sim, app, mut events) = rig(5.0, &[("cedar", &DB), ("narrow", &NARROW)]);
    let (report, session) = next(&app, &mut events).await;
    solved(&report, &session, "narrow", 5.0);
    in_use(&session, "narrow", 4.5, 7.0, "field");
}

#[tokio::test(start_paused = true)]
async fn no_match_reports_last_tried_but_keeps_first_in_use() {
    let (_sim, app, mut events) = rig(12.0, &[("mid", &MID), ("wide", &WIDE)]);
    let (report, session) = next(&app, &mut events).await;
    assert_eq!(report.report.status, Status::NoMatch);
    assert_eq!(report.report.database.as_deref(), Some("wide"));
    assert!(session["field"]["fov_deg"].is_null());
    in_use(&session, "mid", 20.0, 30.0, "first");
}

#[tokio::test(start_paused = true)]
async fn lens_change_to_five_degrees_clears_estimate_after_three_failed_frames() {
    let (sim, app, mut events) = rig(12.0, &[("cedar", &DB), ("narrow", &NARROW)]);
    for number in 1..=16 {
        let (report, session) = next(&app, &mut events).await;
        match number {
            1..=6 => {
                solved(&report, &session, "cedar", 12.0);
                in_use(&session, "cedar", 10.0, 30.0, "field");
            }
            7..=9 => {
                assert_ne!(report.report.status, Status::MatchFound, "report {number}");
                assert_eq!(report.report.database.as_deref(), Some("cedar"));
                in_use(&session, "cedar", 10.0, 30.0, "field");
            }
            10..=16 => {
                solved(&report, &session, "narrow", 5.0);
                in_use(&session, "narrow", 4.5, 7.0, "field");
            }
            _ => unreachable!(),
        }
        if number == 6 {
            set_field(&sim, 5.0);
        }
    }
}

#[tokio::test(start_paused = true)]
async fn lens_change_to_twelve_degrees_switches_after_narrow_solves_without_estimate() {
    let (sim, app, mut events) = rig(5.0, &[("cedar", &DB), ("narrow", &NARROW)]);
    for number in 1..=16 {
        let (report, session) = next(&app, &mut events).await;
        match number {
            1..=6 => {
                solved(&report, &session, "narrow", 5.0);
                in_use(&session, "narrow", 4.5, 7.0, "field");
            }
            7..=9 => {
                assert_ne!(report.report.status, Status::MatchFound, "report {number}");
                assert_eq!(report.report.database.as_deref(), Some("narrow"));
                in_use(&session, "narrow", 4.5, 7.0, "field");
            }
            10..=16 => {
                solved(
                    &report,
                    &session,
                    if number == 10 { "narrow" } else { "cedar" },
                    12.0,
                );
                in_use(&session, "cedar", 10.0, 30.0, "field");
            }
            _ => unreachable!(),
        }
        if number == 6 {
            set_field(&sim, 12.0);
        }
    }
}

#[tokio::test(start_paused = true)]
async fn cold_start_narrow_solves_outside_its_range_then_cedar_takes_over() {
    let (_sim, app, mut events) = rig(12.0, &[("narrow", &NARROW), ("cedar", &DB)]);
    for number in 1..=16 {
        let (report, session) = next(&app, &mut events).await;
        solved(
            &report,
            &session,
            if number == 1 { "narrow" } else { "cedar" },
            12.0,
        );
        in_use(&session, "cedar", 10.0, 30.0, "field");
    }
}

#[tokio::test(start_paused = true)]
async fn field_outside_every_range_keeps_the_database_that_solved() {
    let (_sim, app, mut events) = rig(12.0, &[("narrow", &NARROW), ("wide", &WIDE)]);
    let (report, session) = next(&app, &mut events).await;
    solved(&report, &session, "narrow", 12.0);
    in_use(&session, "narrow", 4.5, 7.0, "solved");
}

#[tokio::test(start_paused = true)]
async fn thirty_seconds_without_any_solve_keeps_first_and_no_field() {
    let (_sim, app, mut events) = rig(12.0, &[("mid", &MID), ("wide", &WIDE)]);
    let mut unsolved_s = 0.0;
    for _ in 0..62 {
        let (report, session) = next(&app, &mut events).await;
        assert_ne!(report.report.status, Status::MatchFound);
        assert!(report.stars >= 10, "rich frames start the unsolved clock");
        assert!(session["field"]["fov_deg"].is_null());
        in_use(&session, "mid", 20.0, 30.0, "first");
        unsolved_s = session["field"]["unsolved_s"].as_f64().unwrap();
        if unsolved_s >= 30.0 {
            break;
        }
    }
    assert!(
        unsolved_s >= 30.0,
        "only {unsolved_s} seconds in 62 reports"
    );
}

#[tokio::test(start_paused = true)]
async fn overlapping_ranges_keep_the_database_that_solved_even_when_listed_second() {
    let (sim, app, mut events) = rig(8.0, &[("mid", &MID), ("cedar", &DB)]);
    for number in 1..=8 {
        let (report, session) = next(&app, &mut events).await;
        match number {
            1 => {
                solved(&report, &session, "cedar", 8.0);
                in_use(&session, "cedar", 10.0, 30.0, "solved");
                set_field(&sim, 25.0);
            }
            2..=4 => {
                assert_ne!(report.report.status, Status::MatchFound, "report {number}");
            }
            5..=8 => {
                solved(&report, &session, "cedar", 25.0);
                in_use(&session, "cedar", 10.0, 30.0, "field");
            }
            _ => unreachable!(),
        }
    }
}
