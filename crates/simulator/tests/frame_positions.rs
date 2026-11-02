//! Oracle: rate × 0.200 s × counts/revolution / 360, within two counts.
mod support;
use scope_link::proto;
use scope_node::mount::{Axis, Mount};
use support::*;

#[tokio::test(start_paused = true)]
async fn exposure_positions_follow_the_node_clock_and_steady_simulated_rate() {
    let mut rig = Rig::new().await;
    rig.rate(proto::Axis::Azimuth, 1.5).await;
    rig.step(0.6, true).await;
    assert_eq!(
        rig.adapter.mount.state(Axis::Azimuth).await.rate_deg_per_s,
        1.5
    );
    let mut frames = rig
        .camera
        .frames(proto::FramesRequest {})
        .await
        .unwrap()
        .into_inner();
    let after = rig.sim.clock().node_now();
    let frame = loop {
        let frame = frames.message().await.unwrap().unwrap();
        if frame.start_s >= after {
            break frame;
        }
    };
    assert!((frame.end_s - frame.start_s - 0.2).abs() < 1e-9);
    assert!(frame.start_s >= 1000.0 && frame.end_s - rig.sim.clock().node_now() <= 1e-9);
    assert!(frame.start_s < rig.sim.clock().now() - 1_000_000.0);
    let start = frame.axis_start.unwrap();
    let end = frame.axis_end.unwrap();
    let expected = 1.5 * 0.2 * f64::from(rig.adapter.axes()[0].counts_per_revolution) / 360.0;
    assert!((f64::from(end.azimuth - start.azimuth) - expected).abs() <= 2.0);
    assert_eq!(start.altitude, 0);
    assert_eq!(end.altitude, 0);
}
