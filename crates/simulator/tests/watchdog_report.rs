//! Oracle: watchdog firings counted exactly once per 1.1 s heartbeat silence;
//! explicit stops do not count. Simulated axes come to rest after braking.
mod support;

use scope_link::proto;
use scope_node::mount::{Axis, Mount};
use support::Rig;

async fn readings(rig: &mut Rig, count: u64) {
    for axis in [proto::Axis::Azimuth, proto::Axis::Altitude] {
        let state = rig
            .mount
            .axis_state(proto::AxisStateRequest { axis: axis as i32 })
            .await
            .unwrap()
            .into_inner();
        assert_eq!(state.watchdog_stops, count, "{axis:?}: {state:?}");
    }
}

#[tokio::test(start_paused = true)]
async fn watchdog_count_survives_heartbeats_and_excludes_explicit_stops() {
    let mut rig = Rig::new().await;
    rig.heartbeat().await;
    readings(&mut rig, 0).await;

    for count in 1..=2 {
        rig.rate(proto::Axis::Azimuth, 1.0).await;
        rig.rate(proto::Axis::Altitude, -1.0).await;
        rig.step(0.1, true).await;
        readings(&mut rig, count - 1).await;
        for axis in [Axis::Azimuth, Axis::Altitude] {
            assert!(rig.adapter.mount.state(axis).await.moving);
        }
        rig.step(1.1, false).await;
        readings(&mut rig, count).await;

        // Remaining silent must not count another firing every watchdog tick.
        // The simulator brakes at 3 deg/s², needing at most 1/3 s at this rate.
        rig.step(0.5, false).await;
        rig.at_rest().await;
        readings(&mut rig, count).await;
        assert_eq!(
            rig.adapter.stops.lock().unwrap().len(),
            2 * count as usize - 1
        );
        rig.stop.stop_all(proto::StopAllRequest {}).await.unwrap();
        readings(&mut rig, count).await;
        assert_eq!(rig.adapter.stops.lock().unwrap().len(), 2 * count as usize);
        rig.heartbeat().await;
        readings(&mut rig, count).await;
    }
}
