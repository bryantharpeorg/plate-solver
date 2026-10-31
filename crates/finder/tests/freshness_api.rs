//! Freshness oracle: console FixedClock minus exposure time, strictly greater
//! than five seconds; link state has no effect. Uploaded pointings never expire.
#[path = "live/support.rs"] mod support;
use support::*;
use finder::{samples::Samples, session::{Clock, FixedClock, Session}, sky::Frame, AppState};
use serde_json::json;
use std::{sync::Arc,time::Duration};
fn frame() -> Frame {
    Frame::from_solution(&plate_solver::Solution { ra:Some(1.0),dec:Some(0.1),roll:Some(0.0),camera:Some(math_core::PinholeCamera::new(1456.0,1088.0,12_f64.to_radians())),..Default::default() }).unwrap()
}
#[tokio::test(start_paused = true)]
async fn connected_and_disconnected_live_pointings_expire_and_a_new_solve_restores_guidance() {
    let sim=sim(); let node=Arc::new(simulator::SimNode::new(sim.clone(),"test-token",Default::default()).unwrap());
    let clock=Arc::new(FixedClock(START+10.0)); let session=Session::default();
    session.edit(serde_json::from_value(json!({"node":{"address":"scope.local:50051","token":"test-token"},"site":{"lat_deg":40,"lon_deg":-75},"target":"M78"})).unwrap(),clock.now()).unwrap();
    let state=AppState::shared(DB.clone(),Samples::default()).with_session(session).with_clock(clock.clone()).with_link(transport(node));
    let link=state.link().unwrap(); let session=state.session(); let app=finder::router(state);
    for _ in 0..80 { if link.status().borrow().connected { break; } tick().await; }
    assert!(link.status().borrow().connected);
    session.record_live_pointing(frame(),clock.now()-5.0,1).unwrap();
    let (_,read)=request(&app,"GET","/api/session",json!(null)).await; assert!(read["guidance"].get("turn_deg").is_some()); assert!(read["guidance"].get("stale").is_none());
    session.record_live_pointing(frame(),clock.now()-5.1,2).unwrap();
    let (_,read)=request(&app,"GET","/api/session",json!(null)).await; assert_eq!(read["node"]["connected"],true); assert_stale(&read);
    sim.set_unresponsive(true); tokio::time::sleep(Duration::from_millis(1100)).await;
    let (_,read)=request(&app,"GET","/api/session",json!(null)).await; assert_eq!(read["node"]["connected"],false); assert_stale(&read);
    session.record_live_pointing(frame(),clock.now(),3).unwrap();
    let (_,read)=request(&app,"GET","/api/session",json!(null)).await; assert!(read["guidance"].get("stale").is_none()); assert!(read["guidance"]["turn_deg"].is_number()); assert!(read["guidance"]["tilt_deg"].is_number());
    session.record_pointing(frame(),30.0,10.0,clock.now()-1000.0);
    let (_,read)=request(&app,"GET","/api/session",json!(null)).await; assert_eq!(read["pointing"]["source"],"upload"); assert!(read["pointing"].get("frame_id").is_none()); assert!(read["guidance"].get("stale").is_none()); assert!(read["guidance"]["turn_deg"].is_number());
}
fn assert_stale(read:&serde_json::Value) { let g=&read["guidance"]; assert_eq!(g["stale"],true); assert!(g.get("turn_deg").is_none()); assert!(g.get("tilt_deg").is_none()); }
