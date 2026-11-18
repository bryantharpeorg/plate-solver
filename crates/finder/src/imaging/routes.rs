//! The imaging routes (031): the view, a settings change, one frame, its
//! abort and its preview.

use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};

use crate::{
    api::{sim_json, ApiError, AppState, SessionWrite},
    imaging::{ImagingView, SettingsEdit},
};

/// The imaging routes, merged into the console's router.
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/imaging", get(get_imaging))
        .route("/api/imaging/settings", put(put_settings))
        .route("/api/imaging/frame", post(take_frame))
        .route("/api/imaging/abort", post(abort_frame))
        .route("/api/imaging/frames/{id}/preview.jpg", get(frame_preview))
}

/// `GET /api/imaging`: the imaging view, or `{"state":"off"}` without imaging.
async fn get_imaging(State(state): State<AppState>) -> Json<ImagingView> {
    Json(match state.imaging() {
        Some(imaging) => imaging.view(),
        None => ImagingView::off(),
    })
}

/// `POST /api/imaging/frame` (FR-008): no body; one frame, saved as FITS.
async fn take_frame(
    State(state): State<AppState>,
    _write: SessionWrite,
) -> Result<Json<ImagingView>, ApiError> {
    let imaging = state.imaging().ok_or_else(|| {
        ApiError::new(StatusCode::CONFLICT, "this console takes no imaging frames")
    })?;
    let view = imaging
        .take("frame")
        .await
        .map_err(|refusal| ApiError::new(refusal.status, refusal.message))?;
    Ok(Json(view))
}

/// `GET /api/imaging/frames/{id}/preview.jpg` (FR-018): the colour JPEG of
/// one of the last eight saved frames, `image/jpeg`; an id the console no
/// longer holds answers 404, an id that is not a number 400.
async fn frame_preview(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<impl IntoResponse, ApiError> {
    let preview = state
        .imaging()
        .and_then(|imaging| imaging.preview(id))
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "the console no longer holds that preview",
            )
        })?;
    Ok(([(header::CONTENT_TYPE, "image/jpeg")], preview))
}

/// `POST /api/imaging/abort` (FR-013): no body; abort the exposure or the
/// frame in flight, or change nothing when there is none.
async fn abort_frame(
    State(state): State<AppState>,
    _write: SessionWrite,
) -> Result<Json<ImagingView>, ApiError> {
    let imaging = state.imaging().ok_or_else(|| {
        ApiError::new(StatusCode::CONFLICT, "this console takes no imaging frames")
    })?;
    Ok(Json(imaging.abort().await))
}

/// `PUT /api/imaging/settings`: a partial settings change under the console's
/// same-origin rule.
async fn put_settings(
    State(state): State<AppState>,
    _write: SessionWrite,
    edit: Result<Json<SettingsEdit>, JsonRejection>,
) -> Result<Json<ImagingView>, ApiError> {
    let edit = sim_json(edit)?;
    let imaging = state.imaging().ok_or_else(|| {
        ApiError::new(StatusCode::CONFLICT, "this console takes no imaging frames")
    })?;
    let view = imaging
        .set(edit)
        .await
        .map_err(|refusal| ApiError::new(refusal.status, refusal.message))?;
    Ok(Json(view))
}
