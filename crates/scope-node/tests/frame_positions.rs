//! Oracle: commanded steady rate × exposure × the simulator's counts/revolution.
mod support;
use scope_link::proto;
use scope_node::mount::{Axis, Mount};
use simulator::{mount::SimMount, VirtualClock};
use std::sync::Arc;
use support::*;

#[tokio::test(start_paused = true)]
async fn frame_positions_bracket_a_200_ms_exposure_on_the_node_clock() {
    let clock = VirtualClock::new(1_768_528_800.0);
    let mount = Arc::new(SimMount::new(clock.clone()));
    let mut rig = rig(clock.clone(), Some(mount.clone())).await;
    rig.mount
        .rate_axis(proto::RateAxisRequest {
            axis: 1,
            deg_per_s: 0.6,
        })
        .await
        .unwrap();
    rig.mount
        .rate_axis(proto::RateAxisRequest {
            axis: 2,
            deg_per_s: -0.3,
        })
        .await
        .unwrap();
    advance(0.4).await;
    assert_eq!(mount.state(Axis::Azimuth).await.rate_deg_per_s, 0.6);
    assert_eq!(mount.state(Axis::Altitude).await.rate_deg_per_s, -0.3);
    beat(&mut rig).await;
    let mut stream = rig
        .client
        .frames(proto::FramesRequest {})
        .await
        .unwrap()
        .into_inner();
    let frame = stream.message().await.unwrap().unwrap();
    assert!((frame.end_s - frame.start_s - 0.2).abs() < 1e-9);
    assert!(frame.start_s >= 1000.4 && frame.end_s <= clock.node_now() + 1e-9);
    let start = frame.axis_start.unwrap();
    let end = frame.axis_end.unwrap();
    let expected = 0.6 * 0.2 * f64::from(mount.axes()[0].counts_per_revolution) / 360.0;
    assert!((f64::from(end.azimuth - start.azimuth) - expected).abs() <= 2.0);
    // The simulator accelerates at 3 degrees/s², then holds the requested rate.
    // Check absolute counts against each exposure timestamp as well as travel:
    // equal travel alone would also accept two readings shifted in time.
    for (axis, rate, first, last) in [
        (Axis::Azimuth, 0.6_f64, start.azimuth, end.azimuth),
        (Axis::Altitude, -0.3_f64, start.altitude, end.altitude),
    ] {
        for (time, position) in [(frame.start_s, first), (frame.end_s, last)] {
            let degrees = rate * (time - 1000.0 - rate.abs() / (2.0 * 3.0));
            let counts =
                degrees * f64::from(mount.axes()[axis.index()].counts_per_revolution) / 360.0;
            assert!((f64::from(position) - counts).abs() <= 2.0);
        }
    }
    assert!(end.azimuth > start.azimuth);
    assert!(end.altitude < start.altitude);
}

#[tokio::test(start_paused = true)]
async fn frames_without_a_mount_omit_both_readings() {
    let mut rig = rig(VirtualClock::new(1_768_528_800.0), None).await;
    let mut frames = rig
        .client
        .frames(proto::FramesRequest {})
        .await
        .unwrap()
        .into_inner();
    let frame = frames.message().await.unwrap().unwrap();
    assert!(frame.axis_start.is_none() && frame.axis_end.is_none());
    assert_eq!(
        rig.mount
            .describe_mount(proto::DescribeMountRequest {})
            .await
            .unwrap_err()
            .code(),
        tonic::Code::FailedPrecondition
    );
    rig.stop.stop_all(proto::StopAllRequest {}).await.unwrap();
}
