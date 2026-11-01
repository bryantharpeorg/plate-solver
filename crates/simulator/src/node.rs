//! The production scope-node service over the catalog-rendered camera.
use crate::{SimCamera, Simulator, VirtualClock};
use scope_link::memory;
use scope_node::{clock::NodeClock, mount::Mount, service::NodeService, RangeError, Settings};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{net::TcpListener, task::JoinHandle};
use tonic::transport::{Channel, Server};

struct Clock(VirtualClock);
impl NodeClock for Clock {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
}

/// Owns capture and transport lifetimes. Dropping it aborts each listening server.
pub struct SimNode {
    service: NodeService,
    servers: Mutex<Vec<JoinHandle<()>>>,
}
impl SimNode {
    pub fn new(
        simulator: Simulator,
        token: impl AsRef<[u8]>,
        initial: Settings,
    ) -> Result<Self, RangeError> {
        let camera = SimCamera::new(simulator.clone());
        Self::from_camera(simulator, camera, token, initial)
    }
    pub fn from_camera(
        simulator: Simulator,
        camera: SimCamera,
        token: impl AsRef<[u8]>,
        initial: Settings,
    ) -> Result<Self, RangeError> {
        let service = NodeService::with_mount(
            camera,
            Arc::new(Clock(simulator.clock().clone())),
            token,
            initial,
            simulator.controls.clone(),
            simulator.mount().map(|m| Arc::new(m) as Arc<dyn Mount>),
        )?;
        Ok(Self {
            service,
            servers: Mutex::new(Vec::new()),
        })
    }
    pub fn with_frame_interval(self, interval: Duration) -> Self {
        self.service.controls().set_frame_interval(interval);
        self
    }
    pub fn service(&self) -> NodeService {
        self.service.clone()
    }
    pub async fn memory_channel(&self) -> Result<Channel, tonic::transport::Error> {
        let (connect, io) = memory::channel_pair();
        let service = self.service.clone().into_routes();
        self.servers.lock().unwrap().push(tokio::spawn(async move {
            if let Err(error) = memory::serve_routes_on(io, service).await {
                eprintln!("simulator memory transport: {error}");
            }
        }));
        connect.await
    }
    pub async fn listen_loopback(&self) -> Result<SocketAddr, std::io::Error> {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await?;
        let address = listener.local_addr()?;
        let service = self.service.clone().into_routes();
        self.servers.lock().unwrap().push(tokio::spawn(async move {
            if let Err(error) = Server::builder()
                .add_routes(service)
                .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
                .await
            {
                eprintln!("simulator loopback transport: {error}");
            }
        }));
        Ok(address)
    }
}
impl Drop for SimNode {
    fn drop(&mut self) {
        for server in self.servers.get_mut().unwrap().drain(..) {
            server.abort();
        }
    }
}
