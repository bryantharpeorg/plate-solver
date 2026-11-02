//! One paired client for TCP nodes and injected transports, with bounded reconnects.

use std::{future::Future, io, pin::Pin, sync::Arc, time::Duration};

use hyper_util::rt::TokioIo;
use scope_link::{
    proto::{
        self, scope_node_client::ScopeNodeClient, scope_node_mount_client::ScopeNodeMountClient,
        scope_node_stop_client::ScopeNodeStopClient,
    },
    BearerToken, ClockMap, Exchange,
};
use serde::Serialize;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::watch,
    task::JoinHandle,
    time::{self, Instant},
};
use tonic::{
    service::interceptor::InterceptedService,
    transport::{Channel, Endpoint},
    Status,
};
use tower::service_fn;

use crate::session::{CameraSettings, Clock, LinkConfig, Session};

const HEARTBEAT: Duration = Duration::from_millis(250);
const SILENCE: Duration = Duration::from_secs(1);
type Connect = Pin<Box<dyn Future<Output = Result<Channel, tonic::transport::Error>> + Send>>;
type Client = ScopeNodeClient<InterceptedService<Channel, BearerToken>>;
type Configure =
    Pin<Box<dyn Future<Output = Result<tonic::Response<proto::Settings>, Status>> + Send>>;

/// Mount calls on the console's paired channel. Stop uses the token-free service.
#[derive(Clone)]
pub struct MountClient {
    client: ScopeNodeMountClient<InterceptedService<Channel, BearerToken>>,
    stop: ScopeNodeStopClient<Channel>,
}
impl MountClient {
    /// Reuse a scope link channel and the session's bearer token.
    pub fn new(channel: Channel, token: &str) -> Result<Self, Status> {
        let bearer = BearerToken::new(token)
            .map_err(|_| Status::invalid_argument("invalid scope node token"))?;
        Ok(Self {
            client: ScopeNodeMountClient::with_interceptor(channel.clone(), bearer),
            stop: ScopeNodeStopClient::new(channel),
        })
    }
    /// Describe the two axes, in azimuth then altitude order.
    pub async fn describe_mount(&mut self) -> Result<proto::DescribeMountReply, Status> {
        Ok(self
            .client
            .describe_mount(proto::DescribeMountRequest {})
            .await?
            .into_inner())
    }
    /// Read an axis's counts, signed rate and moving flag.
    pub async fn axis_state(&mut self, axis: proto::Axis) -> Result<proto::AxisState, Status> {
        Ok(self
            .client
            .axis_state(proto::AxisStateRequest { axis: axis as i32 })
            .await?
            .into_inner())
    }
    /// Request an absolute axis position, in counts.
    pub async fn goto_axis(&mut self, axis: proto::Axis, position: i32) -> Result<(), Status> {
        self.client
            .goto_axis(proto::GotoAxisRequest {
                axis: axis as i32,
                position,
            })
            .await?;
        Ok(())
    }
    /// Request a signed angular rate, in degrees per second.
    pub async fn rate_axis(&mut self, axis: proto::Axis, deg_per_s: f64) -> Result<(), Status> {
        self.client
            .rate_axis(proto::RateAxisRequest {
                axis: axis as i32,
                deg_per_s,
            })
            .await?;
        Ok(())
    }
    /// Stop both axes, regardless of the pairing token's validity.
    pub async fn stop_all(&mut self) -> Result<(), Status> {
        self.stop.stop_all(proto::StopAllRequest {}).await?;
        Ok(())
    }
}

/// A real address, or a reusable connector (one new stream for every connection).
#[derive(Clone, Default)]
pub enum Transport {
    /// Connect to the observing session's host and port over TCP.
    #[default]
    Tcp,
    /// An injected connection, using the same tonic client and protocol as TCP.
    Connector(Arc<dyn Fn() -> Connect + Send + Sync>),
}
impl Transport {
    /// Adapt an asynchronous stream connector. Tests normally return a fresh duplex
    /// and serve its other half through `scope_link::memory::serve_on`.
    pub fn connector<F, Fut, T>(connector: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = io::Result<T>> + Send + 'static,
        T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let connector = Arc::new(connector);
        Self::Connector(Arc::new(move || {
            let connector = connector.clone();
            Box::pin(async move {
                Endpoint::from_static("http://scope-node.memory")
                    .connect_with_connector(service_fn(move |_| {
                        let future = connector();
                        async move { future.await.map(TokioIo::new) }
                    }))
                    .await
            })
        }))
    }
    async fn connect(&self, address: &str) -> Result<Channel, Status> {
        let result = match self {
            Self::Tcp => {
                Endpoint::from_shared(format!("http://{address}"))
                    .map_err(|_| Status::invalid_argument("invalid scope node address"))?
                    .connect()
                    .await
            }
            Self::Connector(connect) => connect().await,
        };
        // Transport errors can contain a URI; keep credentials and node replies out of logs.
        result.map_err(|_| Status::unavailable("scope node transport unavailable"))
    }
}

/// A stored control the connected camera cannot accept. Its stored value is retained.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OutOfRange {
    /// Either `exposure_ms` or `gain`.
    pub setting: &'static str,
    /// The observer's stored value.
    pub stored: f64,
    /// The connected camera's minimum.
    pub min: f64,
    /// The connected camera's maximum.
    pub max: f64,
}

/// Health and telemetry of the current scope link; clocks are supplied by the caller.
#[derive(Debug, Clone, Default)]
pub struct LinkStatus {
    /// True only after an answered heartbeat, until one second of silence or stream end.
    pub connected: bool,
    /// Latest frame's arrival on the console clock (Unix seconds).
    pub last_frame_at: Option<f64>,
    /// Node time less console time, from the least-delay exchange in eight seconds.
    pub offset_s: Option<f64>,
    /// The last described camera, retained during a drop.
    pub camera: Option<proto::CameraInfo>,
    /// Latest received centroid count.
    pub stars: Option<usize>,
    /// Stored controls sent as automatic for this camera.
    pub out_of_range: Vec<OutOfRange>,
    /// Total heartbeat replies recorded by this manager.
    pub heartbeats: u64,
    /// Latest accepted reply on tokio's monotonic clock, for health monitoring.
    pub last_heartbeat_at: Option<Instant>,
}

/// Owns the scope link task. Dropping the manager cancels all in-flight calls.
pub struct LinkManager {
    task: JoinHandle<()>,
    status: watch::Receiver<LinkStatus>,
    frames: watch::Receiver<Option<proto::StarFrame>>,
    mount: watch::Receiver<Option<MountClient>>,
}
impl Drop for LinkManager {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl LinkManager {
    /// Watch the session's pairing and settings; every connection uses this client.
    pub fn start(session: Arc<Session>, clock: Arc<dyn Clock>, transport: Transport) -> Self {
        let (status_tx, status) = watch::channel(LinkStatus::default());
        let (frame_tx, frames) = watch::channel(None);
        let (mount_tx, mount) = watch::channel(None);
        let task = tokio::spawn(run(
            session, clock, transport, status_tx, frame_tx, mount_tx,
        ));
        Self {
            task,
            status,
            frames,
            mount,
        }
    }
    /// Subscribe to link health changes.
    pub fn status(&self) -> watch::Receiver<LinkStatus> {
        self.status.clone()
    }
    /// Subscribe to the newest frame. Intermediate frames are overwritten, never queued.
    pub fn frames(&self) -> watch::Receiver<Option<proto::StarFrame>> {
        self.frames.clone()
    }
    /// Mount calls on the current connection; absent while unpaired or disconnected.
    pub fn mount(&self) -> Option<MountClient> {
        self.mount.borrow().clone()
    }
}

fn publish(session: &Session, tx: &watch::Sender<LinkStatus>, status: &LinkStatus) {
    session.set_link_status(status.clone());
    tx.send_replace(status.clone());
}

/// Build wire settings, retaining cedar's detection defaults except the chosen sigma.
pub(crate) fn camera_settings(
    settings: &CameraSettings,
    camera: &proto::CameraInfo,
) -> (proto::Settings, Vec<OutOfRange>) {
    let mut outside = Vec::new();
    let mut control = |setting, stored: Option<f64>, min, max| {
        stored.filter(|value| {
            if (min..=max).contains(value) {
                true
            } else {
                outside.push(OutOfRange {
                    setting,
                    stored: *value,
                    min,
                    max,
                });
                false
            }
        })
    };
    let exposure_ms = control(
        "exposure_ms",
        settings.exposure_ms.number(),
        camera.exposure_min_ms,
        camera.exposure_max_ms,
    );
    let gain = control(
        "gain",
        settings.gain.number(),
        camera.gain_min,
        camera.gain_max,
    );
    (
        proto::Settings {
            exposure_ms,
            gain,
            detection: Some(proto::Detection {
                sigma: settings.sigma,
                binning: 2,
                normalize_rows: true,
                detect_hot_pixels: true,
            }),
        },
        outside,
    )
}

async fn pair(
    transport: &Transport,
    config: &LinkConfig,
) -> Result<
    (
        Client,
        tonic::Streaming<proto::StarFrame>,
        proto::CameraInfo,
        MountClient,
    ),
    Status,
> {
    let node = config.node.as_ref().expect("paired node");
    let channel = transport
        .connect(node.address.as_deref().expect("node address"))
        .await?;
    let bearer = BearerToken::new(node.token.as_deref().expect("node token"))
        .map_err(|_| Status::invalid_argument("invalid scope node token"))?;
    let mount = MountClient::new(channel.clone(), node.token.as_deref().expect("node token"))?;
    let mut client = ScopeNodeClient::with_interceptor(channel, bearer);
    let camera = client
        .describe(proto::DescribeRequest {})
        .await?
        .into_inner();
    let (settings, _) = camera_settings(&config.settings, &camera);
    client.configure(settings).await?;
    let frames = client.frames(proto::FramesRequest {}).await?.into_inner();
    Ok((client, frames, camera, mount))
}

async fn run(
    session: Arc<Session>,
    clock: Arc<dyn Clock>,
    transport: Transport,
    status_tx: watch::Sender<LinkStatus>,
    frame_tx: watch::Sender<Option<proto::StarFrame>>,
    mount_tx: watch::Sender<Option<MountClient>>,
) {
    let mut changes = session.link_changes();
    let mut status = LinkStatus::default();
    let mut retries = 0;
    let mut previous_node = None;
    loop {
        let config = changes.borrow_and_update().clone();
        if config.node != previous_node {
            previous_node = config.node.clone();
            status = LinkStatus::default();
            frame_tx.send_replace(None);
            retries = 0;
            publish(&session, &status_tx, &status);
        }
        if !config.paired() {
            if changes.changed().await.is_err() {
                return;
            }
            continue;
        }
        let paired = tokio::select! {
            biased;
            result = changes.changed() => { if result.is_err() { return; } continue; }
            result = time::timeout(SILENCE, pair(&transport, &config)) => result,
        };
        if let Ok(Ok((client, stream, camera, mount))) = paired {
            mount_tx.send_replace(Some(mount));
            status.out_of_range = camera_settings(&config.settings, &camera).1;
            status.camera = Some(camera);
            publish(&session, &status_tx, &status);
            let heartbeats_before = status.heartbeats;
            let reconfigured = active(
                &session,
                &clock,
                &mut changes,
                &config,
                client,
                stream,
                &mut status,
                &status_tx,
                &frame_tx,
            )
            .await;
            mount_tx.send_replace(None);
            status.connected = false;
            publish(&session, &status_tx, &status);
            if reconfigured {
                retries = 0;
                continue;
            }
            // An established link starts a new backoff sequence after each drop.
            if status.heartbeats > heartbeats_before {
                retries = 0;
            }
        }
        let delay = [0.5, 1.0, 2.0, 4.0, 5.0][retries.min(4)];
        retries += 1;
        tokio::select! {
            _ = time::sleep(Duration::from_secs_f64(delay)) => {},
            result = changes.changed() => { if result.is_err() { return; } retries = 0; },
        }
    }
}

async fn heartbeat(
    mut client: Client,
    clock: Arc<dyn Clock>,
    at: Instant,
) -> Result<Exchange, Status> {
    time::sleep_until(at).await;
    let t0 = clock.now();
    let reply = client
        .heartbeat(proto::HeartbeatRequest { t0 })
        .await?
        .into_inner();
    Ok(Exchange {
        t0,
        t1: reply.t1,
        t2: reply.t2,
        t3: clock.now(),
    })
}

// Return true on a pairing change, which reconnects immediately instead of backing off.
#[allow(clippy::too_many_arguments)]
async fn active(
    session: &Session,
    clock: &Arc<dyn Clock>,
    changes: &mut watch::Receiver<LinkConfig>,
    config: &LinkConfig,
    client: Client,
    mut stream: tonic::Streaming<proto::StarFrame>,
    status: &mut LinkStatus,
    status_tx: &watch::Sender<LinkStatus>,
    frame_tx: &watch::Sender<Option<proto::StarFrame>>,
) -> bool {
    let mut map = ClockMap::new();
    let mut deadline = Instant::now() + SILENCE;
    let mut next_beat = Instant::now();
    let mut beat = Box::pin(heartbeat(client.clone(), clock.clone(), next_beat));
    let mut configure: Option<Configure> = None;
    loop {
        tokio::select! {
            biased;
            // Discard a late answer: a timed-out connection can only return via pairing.
            _ = time::sleep_until(deadline) => return false,
            result = changes.changed() => {
                if result.is_err() { return true; }
                let next = changes.borrow_and_update().clone();
                if next.node != config.node { return true; }
                let (settings, outside) = camera_settings(&next.settings, status.camera.as_ref().expect("described camera"));
                status.out_of_range = outside;
                let mut settings_client = client.clone();
                configure = Some(Box::pin(async move { settings_client.configure(settings).await }));
                publish(session, status_tx, status);
            }
            result = &mut beat => {
                let Ok(exchange) = result else { return false; };
                map.push(exchange);
                status.offset_s = map.offset();
                status.connected = true;
                status.heartbeats += 1;
                status.last_heartbeat_at = Some(Instant::now());
                deadline = Instant::now() + SILENCE;
                publish(session, status_tx, status);
                next_beat += HEARTBEAT;
                beat = Box::pin(heartbeat(client.clone(), clock.clone(), next_beat));
            }
            result = async { configure.as_mut().expect("pending settings").await }, if configure.is_some() => {
                if result.is_err() { return false; }
                configure = None;
            }
            frame = stream.message() => {
                let Ok(Some(frame)) = frame else { return false; };
                status.last_frame_at = Some(clock.now());
                status.stars = Some(frame.centroids.len());
                frame_tx.send_replace(Some(frame));
                publish(session, status_tx, status);
            }
        }
    }
}
