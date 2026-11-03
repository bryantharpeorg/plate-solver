//! The console checks requests before the SDK or a tool can run.
use std::net::{IpAddr, SocketAddr};

use axum::body::{to_bytes, Body};
use axum::extract::{ConnectInfo, FromRequestParts, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use crate::{api, AppState};

pub(crate) async fn door(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let (mut parts, body) = request.into_parts();
    let bytes = match to_bytes(body, super::BODY_LIMIT).await {
        Ok(bytes) => bytes,
        Err(_) => return StatusCode::PAYLOAD_TOO_LARGE.into_response(),
    };
    if api::check_origin(&parts, &state.allowed_hosts).is_err()
        || !api::host_allowed(&parts, &state.allowed_hosts)
    {
        return denied();
    }
    let peer_allowed = ConnectInfo::<SocketAddr>::from_request_parts(&mut parts, &state)
        .await
        .is_ok_and(|peer| lan_peer(peer.0.ip()));
    if !peer_allowed {
        return denied();
    }

    next.run(Request::from_parts(parts, Body::from(bytes)))
        .await
}

fn denied() -> Response {
    (StatusCode::FORBIDDEN, Json(json!({"jsonrpc":"2.0", "error":{
        "code":-32000, "message":"MCP requires the console's own origin, an allowed host and a LAN peer"
    }}))).into_response()
}

fn lan_peer(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => ip.is_loopback() || ip.is_private() || ip.is_link_local(),
        IpAddr::V6(ip) => match ip.to_ipv4_mapped() {
            Some(ip) => lan_peer(IpAddr::V4(ip)),
            None => ip.is_loopback() || ip.is_unicast_link_local() || ip.is_unique_local(),
        },
    }
}
