//! Console-owned direct moves. Client lifetime never owns the stop timer.
use crate::{
    api::{self, ApiError},
    goto::{AxisMove, GoTo, TICK},
    AppState,
};
use axum::http::StatusCode;
use serde::Serialize;
use std::{sync::Arc, time::Duration};

/// Latest direct move; run-only, retained after completion until the next move.
#[derive(Clone, Debug, Serialize)]
pub(crate) struct Progress {
    pub tool: &'static str,
    pub state: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

pub(crate) async fn start(
    state: &AppState,
    tool: &'static str,
    request: AxisMove,
) -> Result<(), ApiError> {
    api::start_motion(state, "mcp", tool, || begin(state, tool, request)).await
}

async fn begin(state: &AppState, tool: &'static str, request: AxisMove) -> Result<(), ApiError> {
    let conflict = |reason| ApiError::new(StatusCode::CONFLICT, reason);
    let goto = state
        .goto()
        .ok_or_else(|| conflict("no mount is connected".to_owned()))?;
    let mut stops = goto.stops();
    let (mut client, request) = goto.check_axis_move(request).await.map_err(conflict)?;
    if stops.has_changed().unwrap_or(true) {
        return Err(conflict("the observer stopped it".into()));
    }
    let started = state.clock.now();
    let command = async {
        match request {
            AxisMove::Position { axis, position } => client.goto_axis(axis, position).await,
            AxisMove::Rate {
                axis, deg_per_s, ..
            } => client.rate_axis(axis, deg_per_s).await,
        }
    };
    let result = tokio::select! {
        biased;
        _ = stops.changed() => Err("the observer stopped it".to_owned()),
        result = tokio::time::timeout(Duration::from_secs(1), command) =>
            result.map_err(|_| "mount did not answer".to_owned())
                .and_then(|reply| reply.map_err(|error| error.message().to_owned())),
    };
    if let Err(reason) = result {
        let _ = tokio::time::timeout(Duration::from_millis(200), client.stop_all()).await;
        return Err(conflict(reason));
    }
    *state.axis_move.lock().unwrap() = Some(Progress {
        tool,
        state: "moving",
        reason: None,
    });
    let state = state.clone();
    tokio::spawn(async move {
        let result = tokio::select! {
            biased;
            _ = stops.changed() => Err("the observer stopped it".to_owned()),
            reason = halt(goto.clone()) => Err(reason),
            _ = duration(&state, request, started) => Ok(()),
            result = path(&state, &goto, request, started) => result,
        };
        let _ = tokio::time::timeout(Duration::from_millis(200), client.stop_all()).await;
        *state.axis_move.lock().unwrap() = Some(Progress {
            tool,
            state: if result.is_ok() { "done" } else { "halted" },
            reason: result.err(),
        });
    });
    Ok(())
}

async fn halt(goto: Arc<GoTo>) -> String {
    loop {
        if let Some(reason) = goto.halt_reason() {
            return reason;
        }
        tokio::time::sleep(TICK).await;
    }
}
async fn duration(state: &AppState, request: AxisMove, started: f64) {
    let AxisMove::Rate { duration_s, .. } = request else {
        return std::future::pending().await;
    };
    loop {
        let remaining = duration_s - (state.clock.now() - started);
        if remaining <= 1e-6 {
            return;
        }
        tokio::time::sleep(Duration::from_secs_f64(remaining).min(TICK)).await;
    }
}
async fn path(
    state: &AppState,
    goto: &GoTo,
    request: AxisMove,
    started: f64,
) -> Result<(), String> {
    let mut client = state
        .link()
        .and_then(|link| link.mount())
        .ok_or("no mount is connected")?;
    loop {
        let remaining = match request {
            AxisMove::Rate {
                axis,
                deg_per_s,
                duration_s,
            } => {
                let remaining = duration_s - (state.clock.now() - started);
                if remaining <= 1e-6 {
                    return Ok(());
                }
                AxisMove::Rate {
                    axis,
                    deg_per_s,
                    duration_s: remaining + TICK.as_secs_f64(),
                }
            }
            AxisMove::Position { axis, .. } => {
                let reading = client
                    .axis_state(axis)
                    .await
                    .map_err(|_| "mount did not answer")?;
                if !reading.moving {
                    return Ok(());
                }
                request
            }
        };
        goto.recheck_axis_move(remaining).await?;
        tokio::time::sleep(TICK).await;
    }
}
