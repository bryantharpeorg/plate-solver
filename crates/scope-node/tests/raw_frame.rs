//! Oracles: the independently received star frame and cedar's detector on the
//! returned pixels. Dimensions, ids and centroid f64 bits must agree exactly.
use scope_link::{memory, proto, BearerToken};
use scope_node::{
    clock::SystemClock,
    service::{NodeControls, NodeService},
    synthetic::SyntheticCamera,
    Settings,
};
use std::{sync::Arc, time::Duration};
use tokio::{task::JoinHandle, time};
use tonic::{service::interceptor::InterceptedService, transport::Channel, Code};

type RawClient =
    proto::scope_node_raw_client::ScopeNodeRawClient<InterceptedService<Channel, BearerToken>>;
struct Rig {
    channel: Channel,
    frames: tonic::Streaming<proto::StarFrame>,
    controls: NodeControls,
    server: JoinHandle<()>,
}
impl Drop for Rig {
    fn drop(&mut self) {
        self.server.abort();
    }
}
impl Rig {
    async fn new() -> Self {
        let clock = Arc::new(SystemClock);
        let service = NodeService::new(
            SyntheticCamera::new(clock.clone()),
            clock,
            "test-token",
            Settings::default(),
        )
        .unwrap();
        let controls = service.controls();
        let (connect, io) = memory::channel_pair();
        let routes = service.routes();
        let server = tokio::spawn(async move {
            memory::serve_routes_on(io, routes).await.unwrap();
        });
        let channel = connect.await.unwrap();
        let frames = proto::scope_node_client::ScopeNodeClient::with_interceptor(
            channel.clone(),
            BearerToken::new("test-token").unwrap(),
        )
        .frames(proto::FramesRequest {})
        .await
        .unwrap()
        .into_inner();
        Self {
            channel,
            frames,
            controls,
            server,
        }
    }
    fn raw(&self, token: &str) -> RawClient {
        proto::scope_node_raw_client::ScopeNodeRawClient::with_interceptor(
            self.channel.clone(),
            BearerToken::new(token).unwrap(),
        )
    }
}

#[tokio::test(start_paused = true)]
async fn raw_pixels_are_exactly_what_detected_the_streamed_stars() {
    let mut rig = Rig::new().await;
    let raw = time::timeout(
        Duration::from_secs(10),
        rig.raw("test-token").raw_frame(proto::RawFrameRequest {}),
    )
    .await
    .unwrap()
    .unwrap()
    .into_inner();
    assert_eq!(raw.pixels.len(), raw.width as usize * raw.height as usize);
    let frame = time::timeout(Duration::from_secs(10), async {
        loop {
            let frame = rig.frames.message().await.unwrap().unwrap();
            if frame.id == raw.id {
                break frame;
            }
            assert!(frame.id < raw.id);
        }
    })
    .await
    .unwrap();
    assert_eq!(
        (frame.id, frame.width, frame.height),
        (raw.id, raw.width, raw.height)
    );
    let d = frame.settings.unwrap().detection.unwrap();
    let detected = star_detection::detect_stars(
        &raw.pixels,
        raw.width as usize,
        raw.height as usize,
        d.sigma,
        d.binning as usize,
        d.normalize_rows,
        d.detect_hot_pixels,
    );
    let expected: Vec<_> = detected
        .iter()
        .take(200)
        .map(|c| [c.x.to_bits(), c.y.to_bits(), c.brightness.to_bits()])
        .collect();
    let actual: Vec<_> = frame
        .centroids
        .iter()
        .map(|c| [c.x.to_bits(), c.y.to_bits(), c.brightness.to_bits()])
        .collect();
    assert!(!actual.is_empty());
    assert_eq!(actual, expected);
}

#[tokio::test(start_paused = true)]
async fn token_is_required_and_all_waiters_receive_the_same_next_frame() {
    let rig = Rig::new().await;
    let error = rig
        .raw("wrong-token")
        .raw_frame(proto::RawFrameRequest {})
        .await
        .unwrap_err();
    assert_eq!(error.code(), Code::Unauthenticated);
    rig.controls.stop_frames();
    time::sleep(Duration::from_secs(2)).await;
    let mut first = rig.raw("test-token");
    let mut second = first.clone();
    let controls = rig.controls.clone();
    let resume = async move {
        time::sleep(Duration::from_millis(300)).await;
        controls.start_frames();
    };
    let (a, b, ()) = time::timeout(Duration::from_secs(10), async {
        tokio::join!(
            first.raw_frame(proto::RawFrameRequest {}),
            second.raw_frame(proto::RawFrameRequest {}),
            resume
        )
    })
    .await
    .unwrap();
    let a = a.unwrap().into_inner();
    let b = b.unwrap().into_inner();
    assert_eq!(a.id, b.id);
    assert_eq!(a, b);
}
