//! In-memory tonic transport shared by scope node, simulator and console tests.
//!
//! Start [`serve_on`] on the server half before awaiting the connection future
//! returned by [`channel_pair`]. The caller owns the server task's lifetime.

use std::convert::Infallible;
use std::future::Future;
use std::io;
use std::sync::Arc;

use hyper_util::rt::TokioIo;
use tokio::io::DuplexStream;
use tokio::sync::Mutex;
use tonic::body::Body;
use tonic::codegen::{http, Body as HttpBody, Bytes, StdError};
use tonic::server::NamedService;
use tonic::transport::{Channel, Endpoint, Error, Server};
use tower::{service_fn, Service};

/// Creates a client connection future and its server's duplex endpoint.
///
/// The future performs a normal HTTP/2 handshake, so serve the other half
/// concurrently. This single-connection transport opens no sockets. A reconnect
/// attempt returns an I/O error instead of reusing or panicking on a spent stream.
pub fn channel_pair() -> (
    impl Future<Output = Result<Channel, Error>> + Send,
    DuplexStream,
) {
    let (client_io, server_io) = tokio::io::duplex(64 * 1024);
    let connect = async move {
        let client_io = Arc::new(Mutex::new(Some(client_io)));
        Endpoint::from_static("http://scope-node.memory")
            .connect_with_connector(service_fn(move |_| {
                let client_io = client_io.clone();
                async move {
                    let stream = client_io.lock().await.take().ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::ConnectionAborted,
                            "duplex connection already used",
                        )
                    })?;
                    Ok::<_, io::Error>(TokioIo::new(stream))
                }
            }))
            .await
    };
    (connect, server_io)
}

/// Serves a generated tonic service, including its interceptor, on one duplex.
pub async fn serve_on<S, B>(server_io: DuplexStream, service: S) -> Result<(), Error>
where
    S: Service<http::Request<Body>, Response = http::Response<B>, Error = Infallible>
        + NamedService
        + Clone
        + Send
        + Sync
        + 'static,
    S::Future: Send + 'static,
    B: HttpBody<Data = Bytes> + Send + 'static,
    B::Error: Into<StdError>,
{
    Server::builder()
        .add_service(service)
        .serve_with_incoming(tokio_stream::iter(vec![Ok::<_, io::Error>(server_io)]))
        .await
}

/// Serves several generated services with their individual interceptors on one duplex.
pub async fn serve_routes_on(
    server_io: DuplexStream,
    routes: tonic::service::Routes,
) -> Result<(), Error> {
    Server::builder()
        .add_routes(routes)
        .serve_with_incoming(tokio_stream::iter(vec![Ok::<_, io::Error>(server_io)]))
        .await
}
