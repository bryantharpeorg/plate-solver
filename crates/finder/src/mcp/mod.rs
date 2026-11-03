//! Stateless MCP on the console's port, behind its Origin, Host and LAN checks.

pub(crate) mod door;
mod tools;

use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::sync::Arc;

use rmcp::transport::streamable_http_server::{
    session::never::NeverSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use serde::Serialize;

use crate::AppState;

/// Maximum JSON body, including a 32 MiB image encoded as base64.
pub const BODY_LIMIT: usize = 44 * 1024 * 1024;

/// Host addresses advertised by Setup. Tests inject a fixed list.
pub trait AddressSource: Send + Sync {
    /// Report the host's addresses without sending network traffic.
    fn addresses(&self) -> Vec<IpAddr>;
}

struct OutwardAddress;
impl AddressSource for OutwardAddress {
    fn addresses(&self) -> Vec<IpAddr> {
        // UDP connect selects an interface but sends no datagram. TEST-NET-1 is
        // documentation space; no remote service or DNS lookup is involved.
        let address = (|| {
            let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
            socket.connect("192.0.2.1:9").ok()?;
            Some(socket.local_addr().ok()?.ip())
        })();
        address.into_iter().collect()
    }
}

/// MCP is on by default. Host names remain AppState's shared `allowed_hosts` list.
#[derive(Clone)]
pub struct McpConfig {
    /// Mount `/mcp` when true; otherwise let the page fallback handle it.
    pub enabled: bool,
    /// The actual address and port bound by the console.
    pub bound_addr: SocketAddr,
    /// Address enumeration for the read-only Setup response.
    pub address_source: Arc<dyn AddressSource>,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            bound_addr: SocketAddr::from(([127, 0, 0, 1], 8080)),
            address_source: Arc::new(OutwardAddress),
        }
    }
}

#[derive(Serialize)]
pub(crate) struct McpStatus {
    enabled: bool,
    local_only: bool,
    urls: Vec<String>,
    allowed_hosts: Vec<String>,
}

impl McpConfig {
    pub(crate) fn status(&self, allowed_hosts: &[String]) -> McpStatus {
        let local_only = self.bound_addr.ip().is_loopback();
        let port = self.bound_addr.port();
        let mut urls = vec![format!("http://localhost:{port}/mcp")];
        if !local_only {
            let mut addresses = self.address_source.addresses();
            addresses.retain(|ip| {
                !ip.is_loopback()
                    && match ip {
                        IpAddr::V4(_) => true,
                        IpAddr::V6(ip) => ip.is_unicast_link_local() || ip.is_unique_local(),
                    }
            });
            // Keep the source's order within IPv4, link-local IPv6 and ULA groups.
            addresses.sort_by_key(|ip| match ip {
                IpAddr::V4(_) => 0,
                IpAddr::V6(ip) if ip.is_unicast_link_local() => 1,
                IpAddr::V6(_) => 2,
            });
            for ip in addresses {
                let url = format!("http://{}/mcp", SocketAddr::new(ip, port));
                if !urls.contains(&url) {
                    urls.push(url);
                }
            }
        }
        McpStatus {
            enabled: self.enabled,
            local_only,
            urls,
            allowed_hosts: allowed_hosts.to_vec(),
        }
    }
}

pub(crate) fn service(
    state: AppState,
) -> StreamableHttpService<tools::Console, NeverSessionManager> {
    let config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true)
        .disable_allowed_hosts()
        .disable_allowed_origins()
        .with_max_request_body_bytes(BODY_LIMIT);
    StreamableHttpService::new(
        move || Ok(tools::Console::new(state.clone())),
        Arc::new(NeverSessionManager::default()),
        config,
    )
}
