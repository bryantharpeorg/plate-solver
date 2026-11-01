#![allow(dead_code)]
use scope_link::{memory, proto, BearerToken};
use scope_node::{
    clock::NodeClock,
    mount::Mount,
    service::{NodeControls, NodeService},
    synthetic::SyntheticCamera,
    Settings,
};
use simulator::VirtualClock;
use std::{sync::Arc, time::Duration};
use tonic::{service::interceptor::InterceptedService, transport::Channel};

pub struct Clock(pub VirtualClock);
impl NodeClock for Clock {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
}
pub type Client =
    proto::scope_node_client::ScopeNodeClient<InterceptedService<Channel, BearerToken>>;
pub type MountClient =
    proto::scope_node_mount_client::ScopeNodeMountClient<InterceptedService<Channel, BearerToken>>;
pub type StopClient = proto::scope_node_stop_client::ScopeNodeStopClient<Channel>;

pub struct Rig {
    pub service: NodeService,
    pub client: Client,
    pub mount: MountClient,
    pub stop: StopClient,
    pub channel: Channel,
    server: tokio::task::JoinHandle<Result<(), tonic::transport::Error>>,
}
impl Drop for Rig {
    fn drop(&mut self) {
        self.server.abort();
    }
}
pub async fn rig(clock: VirtualClock, mount: Option<Arc<dyn Mount>>) -> Rig {
    let clock = Arc::new(Clock(clock));
    let service = NodeService::with_mount(
        SyntheticCamera::new(clock.clone()),
        clock,
        "test-token",
        Settings {
            exposure_ms: Some(200.0),
            gain: Some(1.0),
            ..Default::default()
        },
        NodeControls::default(),
        mount,
    )
    .unwrap();
    let (channel, io) = memory::channel_pair();
    let server = tokio::spawn(memory::serve_routes_on(io, service.clone().into_routes()));
    let channel = channel.await.unwrap();
    Rig {
        service,
        client: Client::with_interceptor(channel.clone(), BearerToken::new("test-token").unwrap()),
        mount: MountClient::with_interceptor(
            channel.clone(),
            BearerToken::new("test-token").unwrap(),
        ),
        stop: StopClient::new(channel.clone()),
        channel,
        server,
    }
}
pub async fn advance(seconds: f64) {
    // Small external steps let the 50 ms watchdog tick run at its actual deadline.
    for _ in 0..(seconds * 1000.0 / 10.0).round() as usize {
        tokio::time::advance(Duration::from_millis(10)).await;
        tokio::task::yield_now().await;
    }
}
pub async fn beat(rig: &mut Rig) -> proto::HeartbeatReply {
    rig.client
        .heartbeat(proto::HeartbeatRequest { t0: 0.0 })
        .await
        .unwrap()
        .into_inner()
}
