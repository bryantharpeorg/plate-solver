//! The imaging routes (031): the view, a settings change, one frame, its
//! abort and its preview.

use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Deserialize;

use crate::{
    api::{sim_json, ApiError, AppState, SessionWrite},
    imaging::{
        align::AlignAsk,
        calibration::service::{CalibrationView, DarksRequest, FlatsRequest},
        focus::run::FocusAsk,
        run::{By, RunRequest, Runner},
        stack::service::StartRequest,
        steering::{rate_share_checked, smear_checked, step_share_checked},
        Imaging, ImagingView, SettingsEdit,
    },
};

/// The imaging routes, merged into the console's router.
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/imaging", get(get_imaging))
        .route("/api/imaging/settings", put(put_settings))
        .route("/api/imaging/frame", post(take_frame))
        .route("/api/imaging/preview", post(take_preview))
        .route("/api/imaging/abort", post(abort_frame))
        .route("/api/imaging/solve", post(post_solve))
        .route("/api/imaging/focus", post(post_focus).delete(delete_focus))
        .route("/api/imaging/align", post(post_align).delete(delete_align))
        .route("/api/imaging/calibration", get(get_calibration))
        .route("/api/imaging/calibration/darks", post(take_darks))
        .route("/api/imaging/calibration/flats", post(take_flats))
        .route("/api/imaging/calibration/{id}", delete(delete_calibration))
        .route("/api/imaging/frames/{id}/preview.jpg", get(frame_preview))
        .route("/api/imaging/stack", post(start_stack).delete(stop_stack))
        .route("/api/imaging/stack/preview.jpg", get(stack_preview))
        .route("/api/imaging/steering", put(put_steering))
        .route("/api/imaging/run", post(start_run).delete(stop_run))
        .route("/api/imaging/run/continue", post(continue_run))
}

/// A partial edit of the run-only steering settings. Present nulls are refused.
#[derive(Debug, Default)]
pub struct SteeringEdit {
    pub on: Option<bool>,
    pub smear_px: Option<f64>,
    pub step_share: Option<f64>,
    pub rate_share: Option<f64>,
}

impl<'de> Deserialize<'de> for SteeringEdit {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            #[serde(default, deserialize_with = "super::present_value")]
            on: Option<bool>,
            #[serde(default, deserialize_with = "super::present_value")]
            smear_px: Option<f64>,
            #[serde(default, deserialize_with = "super::present_value")]
            step_share: Option<f64>,
            #[serde(default, deserialize_with = "super::present_value")]
            rate_share: Option<f64>,
        }
        // A derived struct also accepts sequences. Require an object before
        // applying its field/type checks, so even an empty array is refused.
        let object = serde_json::Map::<String, serde_json::Value>::deserialize(deserializer)?;
        let fields: Fields = serde_json::from_value(serde_json::Value::Object(object))
            .map_err(serde::de::Error::custom)?;
        Ok(Self {
            on: fields.on,
            smear_px: fields.smear_px,
            step_share: fields.step_share,
            rate_share: fields.rate_share,
        })
    }
}

/// Validate the whole edit in a fixed order before changing any setting.
pub(crate) fn apply_steering(imaging: &Imaging, edit: SteeringEdit) -> Result<ImagingView, String> {
    let smear = edit.smear_px.map(smear_checked).transpose()?;
    let step = edit.step_share.map(step_share_checked).transpose()?;
    let rate = edit.rate_share.map(rate_share_checked).transpose()?;
    let steering = imaging.steering();
    if let Some(smear) = smear {
        steering.set_smear_px(smear)?;
    }
    if let Some(step) = step {
        steering.set_step_share(step)?;
    }
    if let Some(rate) = rate {
        steering.set_rate_share(rate)?;
    }
    if let Some(on) = edit.on {
        steering.set_on(on);
    }
    Ok(imaging.view())
}

async fn put_steering(
    State(state): State<AppState>,
    _write: SessionWrite,
    edit: Result<Json<SteeringEdit>, JsonRejection>,
) -> Result<Json<ImagingView>, ApiError> {
    let edit = sim_json(edit)?;
    let imaging = state.imaging().ok_or_else(|| {
        ApiError::new(StatusCode::CONFLICT, "this console takes no imaging frames")
    })?;
    apply_steering(&imaging, edit)
        .map(Json)
        .map_err(|error| ApiError::new(StatusCode::BAD_REQUEST, error))
}

/// `GET /api/imaging`: the imaging view, or `{"state":"off"}` without imaging.
async fn get_imaging(State(state): State<AppState>) -> Json<ImagingView> {
    Json(match state.imaging() {
        Some(imaging) => imaging.view(),
        None => ImagingView::off(),
    })
}

/// The calibration library in every imaging state, even without imaging.
async fn get_calibration(State(state): State<AppState>) -> Json<CalibrationView> {
    Json(match state.calibration() {
        None => CalibrationView::empty(),
        Some(calibration) => state
            .imaging()
            .and_then(|imaging| imaging.view().calibration)
            .unwrap_or_else(|| calibration.view(None)),
    })
}

/// Take darks: origin, body shape and imaging availability precede the
/// service's count, camera-state, running-set, exposure and lease checks.
async fn take_darks(
    State(state): State<AppState>,
    _write: SessionWrite,
    request: Result<Json<DarksRequest>, JsonRejection>,
) -> Result<Json<CalibrationView>, ApiError> {
    let request = sim_json(request)?;
    let calibration = state.calibration().ok_or_else(|| {
        ApiError::new(StatusCode::CONFLICT, "this console takes no imaging frames")
    })?;
    let view = calibration
        .take_darks(request)
        .await
        .map_err(|refusal| ApiError::new(refusal.status, refusal.message))?;
    Ok(Json(view))
}

/// Find and take flats, or take their darks, under the same check order.
async fn take_flats(
    State(state): State<AppState>,
    _write: SessionWrite,
    request: Result<Json<FlatsRequest>, JsonRejection>,
) -> Result<Json<CalibrationView>, ApiError> {
    let request = sim_json(request)?;
    let calibration = state.calibration().ok_or_else(|| {
        ApiError::new(StatusCode::CONFLICT, "this console takes no imaging frames")
    })?;
    let view = calibration
        .take_flats(request)
        .await
        .map_err(|refusal| ApiError::new(refusal.status, refusal.message))?;
    Ok(Json(view))
}

/// Remove a master under the same-origin rule, judged before imaging or the id.
async fn delete_calibration(
    State(state): State<AppState>,
    _write: SessionWrite,
    Path(id): Path<String>,
) -> Result<Json<CalibrationView>, ApiError> {
    let calibration = state.calibration().ok_or_else(|| {
        ApiError::new(StatusCode::CONFLICT, "this console takes no imaging frames")
    })?;
    let view = calibration
        .remove(&id)
        .await
        .map_err(|refusal| ApiError::new(refusal.status, refusal.message))?;
    Ok(Json(view))
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

/// `POST /api/imaging/preview` (058 FR-001): no body; one preview frame,
/// shown and never saved, with `POST /api/imaging/frame`'s refusals and
/// words.
async fn take_preview(
    State(state): State<AppState>,
    _write: SessionWrite,
) -> Result<Json<ImagingView>, ApiError> {
    let imaging = state.imaging().ok_or_else(|| {
        ApiError::new(StatusCode::CONFLICT, "this console takes no imaging frames")
    })?;
    let view = imaging
        .take_preview()
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

/// `POST /api/imaging/solve` (032 FR-009): no body; solve the latest saved
/// imaging frame and write its solve sidecar, answering the view at once.
async fn post_solve(
    State(state): State<AppState>,
    _write: SessionWrite,
) -> Result<Json<ImagingView>, ApiError> {
    let imaging = state.imaging().ok_or_else(|| {
        ApiError::new(StatusCode::CONFLICT, "this console takes no imaging frames")
    })?;
    let view = imaging
        .solve()
        .await
        .map_err(|refusal| ApiError::new(refusal.status, refusal.message))?;
    Ok(Json(view))
}

/// `POST /api/imaging/focus` (033 FR-020, FR-026): a JSON body of any of
/// `check`, `find`, `tile` and `exposure_ms`; start the focus aid on the focus
/// region, or with `check` a focus check. The body is read before anything
/// else is looked at: a content type that is not JSON answers 415, a body with
/// an unknown key or a value of the wrong kind 400.
async fn post_focus(
    State(state): State<AppState>,
    _write: SessionWrite,
    ask: Result<Json<FocusAsk>, JsonRejection>,
) -> Result<Json<ImagingView>, ApiError> {
    let ask = sim_json(ask)?;
    let imaging = state.imaging().ok_or_else(|| {
        ApiError::new(StatusCode::CONFLICT, "this console takes no imaging frames")
    })?;
    let view = imaging
        .focus_start(ask)
        .await
        .map_err(|refusal| ApiError::new(refusal.status, refusal.message))?;
    Ok(Json(view))
}

/// Stop the finder alignment (050), waiting for the settings to be restored
/// and the camera's lease to be released.
async fn delete_align(
    State(state): State<AppState>,
    _write: SessionWrite,
) -> Result<Json<ImagingView>, ApiError> {
    let imaging = state.imaging().ok_or_else(|| {
        ApiError::new(StatusCode::CONFLICT, "this console takes no imaging frames")
    })?;
    Ok(Json(imaging.align_stop().await))
}

/// Start the finder alignment (050), reading its JSON body as the focus aid
/// does before checking imaging availability and the start's refusals.
async fn post_align(
    State(state): State<AppState>,
    _write: SessionWrite,
    ask: Result<Json<AlignAsk>, JsonRejection>,
) -> Result<Json<ImagingView>, ApiError> {
    let ask = sim_json(ask)?;
    let imaging = state.imaging().ok_or_else(|| {
        ApiError::new(StatusCode::CONFLICT, "this console takes no imaging frames")
    })?;
    let view = imaging
        .align_start(ask)
        .await
        .map_err(|refusal| ApiError::new(refusal.status, refusal.message))?;
    Ok(Json(view))
}

/// `DELETE /api/imaging/focus` (033 FR-028): no body; stop the focus aid, wait
/// until it has put the imaging settings back, and answer the view. With
/// nothing running it answers the view and changes nothing.
async fn delete_focus(
    State(state): State<AppState>,
    _write: SessionWrite,
) -> Result<Json<ImagingView>, ApiError> {
    let imaging = state.imaging().ok_or_else(|| {
        ApiError::new(StatusCode::CONFLICT, "this console takes no imaging frames")
    })?;
    Ok(Json(imaging.focus_stop().await))
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

/// Origin and body shape precede imaging availability and the stack's checks.
async fn start_stack(
    State(state): State<AppState>,
    _write: SessionWrite,
    request: Result<Json<StartRequest>, JsonRejection>,
) -> Result<Json<ImagingView>, ApiError> {
    let request = sim_json(request)?;
    let stacker = state.stack().ok_or_else(|| {
        ApiError::new(StatusCode::CONFLICT, "this console takes no imaging frames")
    })?;
    stacker
        .start(request, state.target_label().as_deref())
        .await
        .map_err(|refusal| ApiError::new(refusal.status, refusal.message))?;
    Ok(Json(
        state.imaging().expect("the stack's imaging state").view(),
    ))
}

/// Wait for the stack to end and save its files before answering the view.
async fn stop_stack(
    State(state): State<AppState>,
    _write: SessionWrite,
) -> Result<Json<ImagingView>, ApiError> {
    let stacker = state.stack().ok_or_else(|| {
        ApiError::new(StatusCode::CONFLICT, "this console takes no imaging frames")
    })?;
    stacker.stop().await;
    Ok(Json(
        state.imaging().expect("the stack's imaging state").view(),
    ))
}

/// The latest kept stack picture; the query does not select a picture.
async fn stack_preview(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let bytes = state
        .stack()
        .and_then(|stacker| stacker.preview())
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "the console holds no stack picture")
        })?;
    Ok(([(header::CONTENT_TYPE, "image/jpeg")], bytes))
}

/// Deserialize through a map so serde's struct sequence form is refused too.
fn run_body<T: serde::de::DeserializeOwned>(
    body: Result<Json<serde_json::Map<String, serde_json::Value>>, JsonRejection>,
) -> Result<T, ApiError> {
    let object = sim_json(body)?;
    serde_json::from_value(serde_json::Value::Object(object))
        .map_err(|error| ApiError::new(StatusCode::BAD_REQUEST, error.to_string()))
}

fn imaging_runner(state: &AppState) -> Result<std::sync::Arc<Runner>, ApiError> {
    state
        .imaging_runner()
        .ok_or_else(|| ApiError::new(StatusCode::CONFLICT, "this console runs no imaging runs"))
}

async fn start_run(
    State(state): State<AppState>,
    _write: SessionWrite,
    request: Result<Json<serde_json::Map<String, serde_json::Value>>, JsonRejection>,
) -> Result<Json<ImagingView>, ApiError> {
    let request: RunRequest = run_body(request)?;
    let runner = imaging_runner(&state)?;
    crate::api::start_motion(&state, "page", "imaging_run", || async {
        runner
            .start(request, By::Page)
            .await
            .map(|_| ())
            .map_err(|refusal| ApiError::new(refusal.status, refusal.message))
    })
    .await?;
    Ok(Json(state.imaging().unwrap().view()))
}

async fn stop_run(
    State(state): State<AppState>,
    _write: SessionWrite,
) -> Result<Json<ImagingView>, ApiError> {
    imaging_runner(&state)?.stop().await;
    Ok(Json(state.imaging().unwrap().view()))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ContinueEdit {
    anyway: bool,
}

async fn continue_run(
    State(state): State<AppState>,
    _write: SessionWrite,
    request: Result<Json<serde_json::Map<String, serde_json::Value>>, JsonRejection>,
) -> Result<Json<ImagingView>, ApiError> {
    let edit: ContinueEdit = run_body(request)?;
    imaging_runner(&state)?
        .resume(edit.anyway)
        .map_err(|refusal| ApiError::new(refusal.status, refusal.message))?;
    Ok(Json(state.imaging().unwrap().view()))
}
