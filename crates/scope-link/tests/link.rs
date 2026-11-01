//! Scope-link acceptance tests: numbered frames are the independent stream oracle.

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use scope_link::proto::{
    scope_node_client::ScopeNodeClient,
    scope_node_server::{ScopeNode, ScopeNodeServer},
    CameraInfo, Centroid, DescribeRequest, Detection, FramesRequest, HeartbeatReply,
    HeartbeatRequest, PreviewAck, PreviewRequest, Settings, StarFrame,
};
use scope_link::{memory, BearerToken, TokenInterceptor};
use tokio_stream::StreamExt;
use tonic::{Code, Request, Response, Status};

#[derive(Clone)]
struct TestNode {
    frames_sent: Arc<AtomicUsize>,
    calls: Arc<AtomicUsize>,
}

fn star_frame(id: u64) -> StarFrame {
    StarFrame {
        id,
        start_s: 200.0,
        end_s: 200.1,
        width: 1456,
        height: 1088,
        exposure_ms: 100.0,
        gain: 1.0,
        settings: Some(Settings {
            exposure_ms: None,
            gain: Some(1.0),
            detection: Some(Detection {
                sigma: 8.0,
                binning: 2,
                normalize_rows: true,
                detect_hot_pixels: true,
            }),
        }),
        centroids: vec![Centroid {
            x: 700.0,
            y: 420.0,
            brightness: 100.0,
        }],
        preview_jpeg: Some(vec![0xff, 0xd8, 0xff, 0xd9]),
        axis_start: None,
        axis_end: None,
    }
}

#[tonic::async_trait]
impl ScopeNode for TestNode {
    async fn describe(&self, _: Request<DescribeRequest>) -> Result<Response<CameraInfo>, Status> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(Response::new(CameraInfo {
            model: "test camera".into(),
            ..Default::default()
        }))
    }

    type FramesStream = tokio_stream::Iter<std::vec::IntoIter<Result<StarFrame, Status>>>;

    async fn frames(
        &self,
        _: Request<FramesRequest>,
    ) -> Result<Response<Self::FramesStream>, Status> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.frames_sent.fetch_add(2, Ordering::SeqCst);
        Ok(Response::new(tokio_stream::iter(vec![
            Ok(star_frame(1)),
            Ok(star_frame(2)),
        ])))
    }

    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatReply>, Status> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(Response::new(HeartbeatReply {
            t0: request.into_inner().t0,
            t1: 100.070,
            t2: 100.071,
        }))
    }

    async fn configure(&self, request: Request<Settings>) -> Result<Response<Settings>, Status> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(Response::new(request.into_inner()))
    }

    async fn request_preview(
        &self,
        _: Request<PreviewRequest>,
    ) -> Result<Response<PreviewAck>, Status> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(Response::new(PreviewAck {}))
    }
}

#[tokio::test]
async fn missing_and_wrong_tokens_serve_nothing_then_paired_frames_arrive() {
    let frames_sent = Arc::new(AtomicUsize::new(0));
    let calls = Arc::new(AtomicUsize::new(0));
    let node = TestNode {
        frames_sent: frames_sent.clone(),
        calls: calls.clone(),
    };
    let (channel, server_io) = memory::channel_pair();
    let server = tokio::spawn(memory::serve_on(
        server_io,
        ScopeNodeServer::with_interceptor(node, TokenInterceptor::new("test-token")),
    ));
    let channel = channel.await.unwrap();
    let mut client = ScopeNodeClient::new(channel.clone());

    let status = client.frames(FramesRequest {}).await.unwrap_err();
    assert_eq!(status.code(), Code::Unauthenticated);
    assert_eq!(frames_sent.load(Ordering::SeqCst), 0);

    let mut wrong = ScopeNodeClient::with_interceptor(
        channel.clone(),
        BearerToken::new("wrong-token").unwrap(),
    );
    let status = wrong.frames(FramesRequest {}).await.unwrap_err();
    assert_eq!(status.code(), Code::Unauthenticated);
    assert_eq!(frames_sent.load(Ordering::SeqCst), 0);
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    // Every RPC is protected by the same service interceptor.
    assert_eq!(
        wrong.describe(DescribeRequest {}).await.unwrap_err().code(),
        Code::Unauthenticated
    );
    assert_eq!(
        wrong
            .heartbeat(HeartbeatRequest { t0: 100.0 })
            .await
            .unwrap_err()
            .code(),
        Code::Unauthenticated
    );
    assert_eq!(
        wrong
            .configure(Settings::default())
            .await
            .unwrap_err()
            .code(),
        Code::Unauthenticated
    );
    assert_eq!(
        wrong
            .request_preview(PreviewRequest {})
            .await
            .unwrap_err()
            .code(),
        Code::Unauthenticated
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    for header in [
        "test-token",
        "Basic test-token",
        "Bearer wrong-token",
        "Bearer ",
    ] {
        let mut request = Request::new(FramesRequest {});
        request
            .metadata_mut()
            .insert("authorization", header.parse().unwrap());
        assert_eq!(
            client.frames(request).await.unwrap_err().code(),
            Code::Unauthenticated
        );
    }
    assert_eq!(frames_sent.load(Ordering::SeqCst), 0);

    let mut paired =
        ScopeNodeClient::with_interceptor(channel, BearerToken::new("test-token").unwrap());
    let mut stream = paired.frames(FramesRequest {}).await.unwrap().into_inner();
    assert_eq!(stream.next().await.unwrap().unwrap(), star_frame(1));
    assert_eq!(stream.next().await.unwrap().unwrap(), star_frame(2));
    assert!(stream.next().await.is_none());
    assert_eq!(frames_sent.load(Ordering::SeqCst), 2);
    assert_eq!(
        paired
            .describe(DescribeRequest {})
            .await
            .unwrap()
            .into_inner()
            .model,
        "test camera"
    );
    let heartbeat = paired
        .heartbeat(HeartbeatRequest { t0: 100.0 })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        (heartbeat.t0, heartbeat.t1, heartbeat.t2),
        (100.0, 100.070, 100.071)
    );
    let settings = star_frame(1).settings.unwrap();
    assert_eq!(
        paired.configure(settings).await.unwrap().into_inner(),
        settings
    );
    paired.request_preview(PreviewRequest {}).await.unwrap();
    server.abort();
}
