//! Nudges (055): one press of an arrow moves the telescope one step on the sky, the
//! way the arrow points in the view the observer picked. With Track off, a press is a
//! direct move of both axes through every guard a GoTo passes
//! (`goto::AxisMove::Offset`): each axis the step moves goes 0.5° past its end and
//! comes back, as a GoTo's approach does. A press writes nothing to the pointing model.

use crate::{
    api::{sim_json, ApiError, AppState, SessionWrite},
    goto::AxisMove,
    mcp::guard,
};
use axum::{
    extract::{rejection::JsonRejection, State},
    http::StatusCode,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};

/// A fine step on the sky, arcseconds (055 FR-002).
pub const FINE_ARCSEC: f64 = 30.0;
/// A coarse step on the sky, arcseconds (055 FR-002).
pub const COARSE_ARCSEC: f64 = 240.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Up,
    Right,
    Down,
    Left,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    Fine,
    Coarse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum View {
    Mount,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NudgeAsk {
    pub direction: Direction,
    pub step: Step,
    pub view: View,
}

/// The step in the mount's view: `up_arcsec` the altitude, `right_arcsec` a clockwise
/// turn of azimuth (055 FR-001, FR-003).
#[derive(Debug, Serialize)]
pub struct NudgeView {
    pub up_arcsec: f64,
    pub right_arcsec: f64,
    pub moved: &'static str,
}

/// `[up, right]` arcseconds in the mount's view. The mount view's units are exact:
/// up is the step and right is 0, or the reverse, signed.
pub fn offset(ask: &NudgeAsk) -> Result<[f64; 2], String> {
    let size = match ask.step {
        Step::Fine => FINE_ARCSEC,
        Step::Coarse => COARSE_ARCSEC,
    };
    match ask.view {
        View::Mount => Ok(match ask.direction {
            Direction::Up => [size, 0.0],
            Direction::Right => [0.0, size],
            Direction::Down => [-size, 0.0],
            Direction::Left => [0.0, -size],
        }),
    }
}

/// `POST /api/mount/nudge`: a same-origin write, answered by `nudge` for the page.
pub fn routes() -> Router<AppState> {
    Router::new().route("/api/mount/nudge", post(press))
}

async fn press(
    State(state): State<AppState>,
    _write: SessionWrite,
    ask: Result<Json<NudgeAsk>, JsonRejection>,
) -> Result<Json<NudgeView>, ApiError> {
    let ask = sim_json(ask)?;
    Ok(Json(nudge(&state, "page", &ask).await?))
}

/// One press, by `by` (`page` or `mcp`). Its refusals answer 409 with their words.
pub async fn nudge(
    state: &AppState,
    by: &'static str,
    ask: &NudgeAsk,
) -> Result<NudgeView, ApiError> {
    let [up, right] = offset(ask).map_err(|reason| ApiError::new(StatusCode::CONFLICT, reason))?;
    guard::start_by(
        state,
        by,
        "nudge",
        AxisMove::Offset {
            up_arcsec: up,
            right_arcsec: right,
        },
    )
    .await?;
    Ok(NudgeView {
        up_arcsec: up,
        right_arcsec: right,
        moved: "axes",
    })
}
