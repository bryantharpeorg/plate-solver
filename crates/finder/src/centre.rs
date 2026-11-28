//! Centring from an imaging solve (057): the session's target is brought to
//! the imaging camera's set point, the sensor's centre, with Track's own step
//! (055). The place Track holds becomes that place, and the telescope moves by
//! what the latest frame's solve says lies between it and the sky at the set
//! point. Nothing here needs a live pointing or writes the pointing model.

use crate::{
    api::{sim_json, ApiError, AppState, SessionWrite},
    imaging::{solver::ImagingSolve, steering::set_point_in_frame},
    model::separation,
};
use axum::{
    extract::{rejection::JsonRejection, State},
    http::StatusCode,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};

/// How long a solve holds, in seconds after its frame ended: a centring
/// refuses an older one, and the finder camera's first solve within it after a
/// centring takes the aim point from the place Track holds.
pub const AGE_S: f64 = 60.0;

/// The body of a centring: none of the three for the session's target, or
/// `frame_id`, `x` and `y` together for a pixel of that imaging frame (FR-006).
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CentreAsk {
    pub frame_id: Option<u64>,
    pub x: Option<f64>,
    pub y: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct CentreView {
    pub frame_id: u64,
    pub offset_arcsec: f64,
    pub moved: &'static str,
}

/// The words for a malformed ask (FR-006).
pub const PARTS: &str = "centre takes nothing, to centre the target, or frame_id, x and y together";
const NOT_SOLVED: &str = "the imaging camera has not solved a frame yet";
const NOT_TRACKING: &str = "Track is not tracking yet";

/// `POST /api/imaging/centre`: the page's same-origin write.
pub fn routes() -> Router<AppState> {
    Router::new().route("/api/imaging/centre", post(press))
}

async fn press(
    State(state): State<AppState>,
    _write: SessionWrite,
    ask: Result<Json<CentreAsk>, JsonRejection>,
) -> Result<Json<CentreView>, ApiError> {
    let ask = sim_json(ask)?;
    Ok(Json(centre(&state, "page", &ask).await?))
}

/// Centre the session's target, or the point `ask` names on the latest imaging
/// solve, from that solve alone, by `by`. A malformed ask answers 400; other
/// refusals answer 409, checked in FR-002's order, with a point's own checks
/// (FR-007) where the target's are.
pub async fn centre(
    state: &AppState,
    by: &'static str,
    ask: &CentreAsk,
) -> Result<CentreView, ApiError> {
    let point = match (ask.frame_id, ask.x, ask.y) {
        (None, None, None) => None,
        (Some(frame_id), Some(x), Some(y)) if x.is_finite() && y.is_finite() => {
            Some((frame_id, x, y))
        }
        _ => return Err(ApiError::new(StatusCode::BAD_REQUEST, PARTS)),
    };
    let now = state.clock.now();
    let session = state.session().snapshot(now);
    let conflict = |words| ApiError::new(StatusCode::CONFLICT, words);
    let view = state
        .imaging()
        .map(|imaging| imaging.view())
        .ok_or_else(|| conflict(NOT_SOLVED.to_owned()))?;
    let solve = view
        .solve
        .as_ref()
        .ok_or_else(|| conflict(NOT_SOLVED.to_owned()))?;
    let frame = view
        .frame
        .as_ref()
        .filter(|frame| frame.id == solve.frame_id)
        .ok_or_else(|| {
            conflict(format!(
                "a newer imaging frame was taken after frame {}",
                solve.frame_id
            ))
        })?;
    match solve.state {
        "solving" => {
            return Err(conflict(
                "the imaging camera's solve is still running".into(),
            ));
        }
        "failed" => {
            return Err(conflict(
                "the imaging camera's latest frame did not solve".into(),
            ));
        }
        _ => {}
    }
    let (Some(ra), Some(dec), Some(cd), Some(width), Some(height), Some(start), Some(camera)) = (
        solve.ra_deg,
        solve.dec_deg,
        solve.cd_deg,
        frame.width,
        frame.height,
        frame.start,
        view.camera.as_ref(),
    ) else {
        return Err(conflict(NOT_SOLVED.into()));
    };
    let to = match point {
        Some((frame_id, x, y)) => {
            if frame_id != solve.frame_id {
                return Err(conflict(format!(
                    "imaging frame {frame_id} is not the latest solved frame"
                )));
            }
            if !(0.0..=f64::from(width)).contains(&x) || !(0.0..=f64::from(height)).contains(&y) {
                return Err(conflict(format!(
                    "the point is outside imaging frame {frame_id}"
                )));
            }
            ImagingSolve::from_cd(ra, dec, cd, width, height).sky_at((x, y))
        }
        None => {
            let target = session
                .target
                .ok_or_else(|| conflict("no target is selected".into()))?;
            (target.ra_deg, target.dec_deg)
        }
    };
    if !session.track.as_ref().is_some_and(|track| track.wanted) {
        return Err(conflict("Track is off: turn Track on to centre".into()));
    }
    let tracker = state
        .session()
        .tracker()
        .filter(|tracker| tracker.state().state == "tracking")
        .ok_or_else(|| conflict(NOT_TRACKING.into()))?;
    if !tracker.steady_between(start, now) {
        return Err(conflict(format!(
            "the telescope moved after imaging frame {}: take another and solve it",
            solve.frame_id
        )));
    }
    if now - (start + frame.exposure_ms / 1000.0) > AGE_S {
        return Err(conflict(format!(
            "imaging frame {} is more than a minute old: take another and solve it",
            solve.frame_id
        )));
    }
    let set_point = set_point_in_frame(
        (camera.width, camera.height),
        None,
        frame.roi.into(),
        frame.binning.unwrap_or(1),
    );
    let from = ImagingSolve::from_cd(ra, dec, cd, width, height).sky_at(set_point);
    let offset_arcsec = separation(from.0, from.1, to.0, to.1) * 3600.0;
    crate::nudge::track_step(state, by, "centre", now, |tracker, model| async move {
        tracker.centre(from, to, model).await
    })
    .await?;
    state.session().centred(now + AGE_S);
    Ok(CentreView {
        frame_id: solve.frame_id,
        offset_arcsec,
        moved: "track",
    })
}
