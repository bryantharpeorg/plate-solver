//! Oracles: the simulated mount's own capabilities and readings, exact equality;
//! unauthenticated movement changes neither axis; stopping decelerates to zero.
use finder::{
    link::{LinkManager, MountClient, Transport},
    session::{Clock, Session},
};
use pattern_database::PatternDatabase;
use scope_link::{memory, proto};
use scope_node::{
    mount::{Axis, Mount},
    Settings,
};
use simulator::{mount::SimMount, SimNode, Simulator, Site, VirtualClock};
use std::{
    path::Path,
    sync::{Arc, LazyLock},
    time::Duration,
};

static DB: LazyLock<Arc<PatternDatabase>> = LazyLock::new(|| {
    Arc::new(
        pattern_database::load_from_path(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../reference-solutions/cedar-solve/tetra3/data/default_database.npz"),
        )
        .unwrap(),
    )
});
struct ConsoleClock(VirtualClock);
impl Clock for ConsoleClock {
    fn now(&self) -> f64 {
        self.0.now()
    }
}
fn setup() -> (Simulator, SimMount, SimNode) {
    let clock = VirtualClock::new(1_768_528_800.0);
    let sim = Simulator::new(DB.clone(), Site::default(), clock.clone());
    let mount = SimMount::new(clock);
    sim.connect_mount(mount.clone());
    let node = SimNode::new(sim.clone(), "test-token", Settings::default()).unwrap();
    (sim, mount, node)
}
async fn step(seconds: f64) {
    for _ in 0..(seconds * 100.0).round() as usize {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test(start_paused = true)]
async fn missing_and_wrong_tokens_cannot_move_either_axis_but_anyone_can_stop() {
    let (_sim, mount, node) = setup();
    let channel = node.memory_channel().await.unwrap();
    let mut unauth = proto::scope_node_mount_client::ScopeNodeMountClient::new(channel.clone());
    let mut wrong = MountClient::new(channel.clone(), "wrong-token").unwrap();
    assert_eq!(
        wrong
            .rate_axis(proto::Axis::Azimuth, 1.0)
            .await
            .unwrap_err()
            .code(),
        tonic::Code::Unauthenticated
    );
    for axis in [proto::Axis::Azimuth, proto::Axis::Altitude] {
        assert_eq!(
            unauth
                .rate_axis(proto::RateAxisRequest {
                    axis: axis as i32,
                    deg_per_s: 1.0
                })
                .await
                .unwrap_err()
                .code(),
            tonic::Code::Unauthenticated
        );
        assert_eq!(
            unauth
                .goto_axis(proto::GotoAxisRequest {
                    axis: axis as i32,
                    position: 4500
                })
                .await
                .unwrap_err()
                .code(),
            tonic::Code::Unauthenticated
        );
        assert_eq!(
            unauth
                .axis_state(proto::AxisStateRequest { axis: axis as i32 })
                .await
                .unwrap_err()
                .code(),
            tonic::Code::Unauthenticated
        );
    }
    assert_eq!(
        unauth
            .describe_mount(proto::DescribeMountRequest {})
            .await
            .unwrap_err()
            .code(),
        tonic::Code::Unauthenticated
    );
    step(0.3).await;
    for axis in [Axis::Azimuth, Axis::Altitude] {
        let state = mount.state(axis).await;
        assert_eq!(state.position, 0);
        assert_eq!(state.rate_deg_per_s, 0.0);
        assert!(!state.moving);
    }
    // Positive control: this node has a working mount and authenticated moves.
    let mut accepted = MountClient::new(channel.clone(), "test-token").unwrap();
    accepted.rate_axis(proto::Axis::Azimuth, 1.0).await.unwrap();
    accepted
        .rate_axis(proto::Axis::Altitude, -1.0)
        .await
        .unwrap();
    step(0.4).await;
    for axis in [Axis::Azimuth, Axis::Altitude] {
        assert!(mount.state(axis).await.moving);
        assert_ne!(mount.state(axis).await.position, 0);
    }
    proto::scope_node_stop_client::ScopeNodeStopClient::new(channel)
        .stop_all(proto::StopAllRequest {})
        .await
        .unwrap();
    step(0.4).await;
    for axis in [Axis::Azimuth, Axis::Altitude] {
        assert!(!mount.state(axis).await.moving);
        assert_eq!(mount.state(axis).await.rate_deg_per_s, 0.0);
    }
}

#[tokio::test(start_paused = true)]
async fn console_mount_calls_share_the_paired_channel_and_match_simulator_exactly() {
    let (sim, mount, node) = setup();
    let node = Arc::new(node);
    let transport = Transport::connector(move || {
        let service = node.service();
        async move {
            let (client, server) = tokio::io::duplex(64 * 1024);
            tokio::spawn(async move {
                memory::serve_routes_on(server, service.routes())
                    .await
                    .unwrap();
            });
            Ok::<_, std::io::Error>(client)
        }
    });
    let session = Arc::new(Session::default());
    session
        .edit(
            serde_json::from_value(
                serde_json::json!({"node":{"address":"scope.local:50051","token":"test-token"}}),
            )
            .unwrap(),
            0.0,
        )
        .unwrap();
    let link = LinkManager::start(
        session,
        Arc::new(ConsoleClock(sim.clock().clone())),
        transport,
    );
    for _ in 0..100 {
        if link.status().borrow().connected {
            break;
        }
        step(0.01).await;
    }
    assert!(link.status().borrow().connected);
    let mut client = link.mount().expect("mount client on the paired channel");
    let info = client.describe_mount().await.unwrap();
    assert_eq!(info.axes.len(), 2);
    for (actual, expected) in info.axes.iter().zip(mount.axes()) {
        assert_eq!(actual.counts_per_revolution, expected.counts_per_revolution);
        assert_eq!(actual.top_rate_deg_per_s, expected.top_rate_deg_per_s);
        assert_eq!(actual.can_goto, expected.can_goto);
    }
    client.rate_axis(proto::Axis::Azimuth, 1.2).await.unwrap();
    client.rate_axis(proto::Axis::Altitude, -0.6).await.unwrap();
    step(0.5).await;
    for (wire, axis) in [
        (proto::Axis::Azimuth, Axis::Azimuth),
        (proto::Axis::Altitude, Axis::Altitude),
    ] {
        let actual = client.axis_state(wire).await.unwrap();
        let expected = mount.state(axis).await;
        assert_eq!(actual.position, expected.position);
        assert_eq!(actual.rate_deg_per_s, expected.rate_deg_per_s);
        assert_eq!(actual.moving, expected.moving);
        assert!(actual.moving && actual.position != 0);
    }
    client.stop_all().await.unwrap();
    step(0.5).await;
    client.goto_axis(proto::Axis::Azimuth, 4500).await.unwrap();
    step(2.0).await;
    assert_eq!(
        client
            .axis_state(proto::Axis::Azimuth)
            .await
            .unwrap()
            .position,
        4500
    );
}

#[tokio::test(start_paused = true)]
async fn invalid_axis_and_rate_are_refused_and_a_node_without_a_mount_can_be_stopped() {
    let (sim, mount, node) = setup();
    let channel = node.memory_channel().await.unwrap();
    let mut client = MountClient::new(channel, "test-token").unwrap();
    assert_eq!(
        client
            .axis_state(proto::Axis::Unspecified)
            .await
            .unwrap_err()
            .code(),
        tonic::Code::InvalidArgument
    );
    assert_eq!(
        client
            .goto_axis(proto::Axis::Unspecified, 4500)
            .await
            .unwrap_err()
            .code(),
        tonic::Code::InvalidArgument
    );
    for rate in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            client
                .rate_axis(proto::Axis::Azimuth, rate)
                .await
                .unwrap_err()
                .code(),
            tonic::Code::InvalidArgument
        );
    }
    assert!(!mount.state(Axis::Azimuth).await.moving);
    sim.disconnect_mount();
    let node = SimNode::new(sim, "test-token", Settings::default()).unwrap();
    let mut client = MountClient::new(node.memory_channel().await.unwrap(), "test-token").unwrap();
    assert!(client.describe_mount().await.unwrap().axes.is_empty());
    assert_eq!(
        client
            .axis_state(proto::Axis::Azimuth)
            .await
            .unwrap_err()
            .code(),
        tonic::Code::FailedPrecondition
    );
    assert_eq!(
        client
            .goto_axis(proto::Axis::Azimuth, 4500)
            .await
            .unwrap_err()
            .code(),
        tonic::Code::FailedPrecondition
    );
    client.stop_all().await.unwrap();
    let mut camera = proto::scope_node_client::ScopeNodeClient::with_interceptor(
        node.memory_channel().await.unwrap(),
        scope_link::BearerToken::new("test-token").unwrap(),
    );
    let frame = camera
        .frames(proto::FramesRequest {})
        .await
        .unwrap()
        .into_inner()
        .message()
        .await
        .unwrap()
        .unwrap();
    assert!(frame.axis_start.is_none() && frame.axis_end.is_none());
}
