//! Real node + simulated mount over one duplex, with independent adapter readings.
#[path = "../../simulator/tests/support/mod.rs"]
mod support;
use finder::link::MountClient;
use scope_link::{
    proto::{self, scope_node_mount_client::ScopeNodeMountClient},
    BearerToken,
};
use scope_node::mount::{Axis, Mount};
use simulator::{mount::SimMount, VirtualClock};
use std::sync::Arc;
use support::{advance, beat, rig};

#[tokio::test(start_paused = true)]
async fn only_stop_is_open_and_the_console_reads_exact_mount_capabilities_and_state() {
    let clock = VirtualClock::new(1_768_528_800.0);
    let mount = Arc::new(SimMount::new(clock.clone()));
    let mut rig = rig(clock, Some(mount.clone())).await;
    let mut anonymous = ScopeNodeMountClient::new(rig.channel.clone());
    assert_eq!(
        anonymous
            .rate_axis(proto::RateAxisRequest {
                axis: 1,
                deg_per_s: 1.0
            })
            .await
            .unwrap_err()
            .code(),
        tonic::Code::Unauthenticated
    );
    assert_eq!(
        anonymous
            .goto_axis(proto::GotoAxisRequest {
                axis: 2,
                position: 10000
            })
            .await
            .unwrap_err()
            .code(),
        tonic::Code::Unauthenticated
    );
    assert_eq!(
        anonymous
            .describe_mount(proto::DescribeMountRequest {})
            .await
            .unwrap_err()
            .code(),
        tonic::Code::Unauthenticated
    );
    assert_eq!(
        anonymous
            .axis_state(proto::AxisStateRequest { axis: 1 })
            .await
            .unwrap_err()
            .code(),
        tonic::Code::Unauthenticated
    );
    advance(0.2).await;
    for axis in [Axis::Azimuth, Axis::Altitude] {
        let state = mount.state(axis).await;
        assert_eq!(state.position, 0);
        assert_eq!(state.rate_deg_per_s, 0.0);
        assert!(!state.moving);
    }
    let console = MountClient::new(rig.channel.clone(), BearerToken::new("test-token").unwrap());
    let info = console.describe_mount().await.unwrap();
    for (wire, axis) in info.axes.iter().zip(mount.axes()) {
        assert_eq!(wire.counts_per_revolution, axis.counts_per_revolution);
        assert_eq!(wire.top_rate_deg_per_s, axis.top_rate_deg_per_s);
        assert_eq!(wire.can_goto, axis.can_goto);
    }
    assert_eq!(info.axes.len(), 2);
    for (axis, rate) in [(proto::Axis::Azimuth, 0.6), (proto::Axis::Altitude, -0.3)] {
        console.rate_axis(axis, rate).await.unwrap();
    }
    advance(0.3).await;
    beat(&mut rig).await;
    for (wire_axis, axis) in [
        (proto::Axis::Azimuth, Axis::Azimuth),
        (proto::Axis::Altitude, Axis::Altitude),
    ] {
        let wire = console.axis_state(wire_axis).await.unwrap();
        let state = mount.state(axis).await;
        assert_eq!(wire.position, state.position);
        assert_eq!(wire.rate_deg_per_s, state.rate_deg_per_s);
        assert_eq!(wire.moving, state.moving);
        assert!(wire.moving);
        assert_ne!(wire.position, 0);
    }
    // No token on this separate service, on the same channel as the protected ones.
    rig.stop.stop_all(proto::StopAllRequest {}).await.unwrap();
    advance(0.3).await;
    for axis in [Axis::Azimuth, Axis::Altitude] {
        assert!(!mount.state(axis).await.moving);
        assert_eq!(mount.state(axis).await.rate_deg_per_s, 0.0);
    }
    console.goto_axis(proto::Axis::Azimuth, 1000).await.unwrap();
    advance(0.6).await;
    assert_eq!(mount.state(Axis::Azimuth).await.position, 1000);
    console.stop_all().await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn malformed_motion_is_refused_without_moving_the_connected_mount() {
    let clock = VirtualClock::new(1_768_528_800.0);
    let mount = Arc::new(SimMount::new(clock.clone()));
    let mut rig = rig(clock, Some(mount.clone())).await;
    for axis in [0, 99] {
        assert_eq!(
            rig.mount
                .axis_state(proto::AxisStateRequest { axis })
                .await
                .unwrap_err()
                .code(),
            tonic::Code::InvalidArgument
        );
        assert_eq!(
            rig.mount
                .goto_axis(proto::GotoAxisRequest { axis, position: 1 })
                .await
                .unwrap_err()
                .code(),
            tonic::Code::InvalidArgument
        );
    }
    for rate in [f64::NAN, f64::INFINITY, -f64::INFINITY] {
        assert_eq!(
            rig.mount
                .rate_axis(proto::RateAxisRequest {
                    axis: 1,
                    deg_per_s: rate
                })
                .await
                .unwrap_err()
                .code(),
            tonic::Code::InvalidArgument
        );
    }
    assert!(!mount.state(Axis::Azimuth).await.moving);
    rig.mount
        .rate_axis(proto::RateAxisRequest {
            axis: 1,
            deg_per_s: 0.3,
        })
        .await
        .unwrap();
    advance(0.2).await;
    assert!(mount.state(Axis::Azimuth).await.moving);
}
