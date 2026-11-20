//! Thin tools over the same functions the page's routes call.
use std::sync::Arc;

use axum::Json;
use base64::prelude::{Engine as _, BASE64_STANDARD};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, ContentBlock, Implementation, JsonObject, ServerCapabilities, ServerConfig,
};
use rmcp::{schemars, tool, tool_handler, tool_router, ServerHandler};
use serde_json::{json, Value};

use crate::api;
use crate::imaging::calibration::service::{CalibrationView, DarksRequest, FlatsRequest};
use crate::imaging::routes::SteeringEdit;
use crate::imaging::stack::{display, service::StartRequest};
use crate::session::SessionEdit;
use crate::AppState;

#[derive(Clone)]
pub(crate) struct Console {
    state: AppState,
    tool_router: ToolRouter<Self>,
}

// Only used to describe the tool. Parsing deliberately uses SessionEdit so the
// route and tool keep identical rejection reasons for invalid/unknown fields.
#[derive(rmcp::schemars::JsonSchema)]
#[expect(
    dead_code,
    reason = "schema only; SessionEdit parses the actual arguments"
)]
struct TargetArguments {
    target: Option<String>,
}

#[derive(rmcp::schemars::JsonSchema)]
#[expect(
    dead_code,
    reason = "schema only; StartRequest parses the actual arguments"
)]
struct StackingArguments {
    on: bool,
    reject: Option<bool>,
    focus_gate: Option<bool>,
    gains: Option<Vec<f64>>,
    stretch: Option<f64>,
}

#[derive(rmcp::schemars::JsonSchema)]
#[expect(
    dead_code,
    reason = "schema only; SteeringEdit parses the actual arguments"
)]
struct SteeringArguments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    on: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "f64")]
    smear_px: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "f64")]
    step_share: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "f64")]
    rate_share: Option<f64>,
}

#[derive(rmcp::schemars::JsonSchema)]
#[expect(
    dead_code,
    reason = "schema only; DarksRequest parses the actual arguments"
)]
struct DarksArguments {
    count: u32,
    anyway: Option<bool>,
}

#[derive(rmcp::schemars::JsonSchema)]
#[expect(
    dead_code,
    reason = "schema only; FlatsRequest parses the actual arguments"
)]
struct FlatsArguments {
    count: Option<u32>,
    label: Option<String>,
    darks: Option<bool>,
    anyway: Option<bool>,
}

// Only used to describe the tool. Parsing deliberately uses SettingsEdit so the
// route and tool keep identical rejection reasons for invalid/unknown fields.
// The seven keys that refuse null carry `with` so the schema lists them as
// optional and not nullable; `roi` and `forward_kib_per_s`, which null clears,
// stay plain `Option`s and nullable.
#[derive(rmcp::schemars::JsonSchema)]
#[expect(
    dead_code,
    reason = "schema only; SettingsEdit parses the actual arguments"
)]
struct ImagingSettingsArguments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "f64")]
    exposure_ms: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "f64")]
    gain: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "f64")]
    offset: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u32")]
    binning: Option<u32>,
    roi: Option<ImagingRoiArguments>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    cooler_on: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "f64")]
    set_temp_c: Option<f64>,
    forward_kib_per_s: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    save_at_node: Option<bool>,
}

#[derive(rmcp::schemars::JsonSchema)]
#[expect(
    dead_code,
    reason = "schema only; SettingsEdit parses the actual arguments"
)]
struct ImagingRoiArguments {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[derive(rmcp::schemars::JsonSchema)]
#[expect(
    dead_code,
    reason = "schema only; SessionEdit parses the actual arguments"
)]
struct SessionArguments {
    site: Option<SiteArguments>,
    min_alt_deg: Option<f64>,
    target: Option<String>,
}

#[derive(rmcp::schemars::JsonSchema)]
#[expect(
    dead_code,
    reason = "schema only; SessionEdit parses the actual arguments"
)]
struct SiteArguments {
    lat_deg: f64,
    lon_deg: f64,
}

#[derive(serde::Deserialize, rmcp::schemars::JsonSchema)]
struct SearchArguments {
    q: String,
    limit: Option<usize>,
}

#[derive(serde::Deserialize, rmcp::schemars::JsonSchema)]
struct FrameArguments {
    /// Image file encoded with standard base64, at most 32 MiB once decoded.
    image: String,
    /// Optional file name to include in the solve report.
    name: Option<String>,
}

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize, rmcp::schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
enum AxisArgument {
    Azimuth,
    Altitude,
}
impl From<AxisArgument> for scope_link::proto::Axis {
    fn from(axis: AxisArgument) -> Self {
        match axis {
            AxisArgument::Azimuth => Self::Azimuth,
            AxisArgument::Altitude => Self::Altitude,
        }
    }
}
#[derive(serde::Deserialize, serde::Serialize, rmcp::schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct PositionArguments {
    axis: AxisArgument,
    position: i32,
}
#[derive(serde::Deserialize, serde::Serialize, rmcp::schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct RateArguments {
    axis: AxisArgument,
    deg_per_s: f64,
    duration_s: f64,
}

#[derive(serde::Deserialize, serde::Serialize, rmcp::schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct TrackingArguments {
    on: bool,
}

// Only used to describe the tool. Parsing deliberately uses FocusAsk so the
// route and tool keep identical rejection reasons for invalid/unknown fields.
#[derive(rmcp::schemars::JsonSchema)]
#[expect(
    dead_code,
    reason = "schema only; FocusAsk parses the actual arguments"
)]
struct FocusAidArguments {
    on: bool,
    check: Option<bool>,
    find: Option<bool>,
    tile: Option<FocusTileArguments>,
    exposure_ms: Option<f64>,
}

#[derive(rmcp::schemars::JsonSchema)]
#[expect(
    dead_code,
    reason = "schema only; FocusAsk parses the actual arguments"
)]
struct FocusTileArguments {
    col: u32,
    row: u32,
}

fn object_schema() -> Arc<JsonObject> {
    Arc::new(json!({"type":"object"}).as_object().unwrap().clone())
}

// MCP needs an object for structured content; text keeps the underlying route
// array or session field unchanged, including a field whose value is null.
fn field_result(field: &str, value: Value) -> CallToolResult {
    let mut result = CallToolResult::structured(json!({field: value}));
    result.content = vec![ContentBlock::text(value.to_string())];
    result
}

fn tool_result(result: Result<Value, String>) -> CallToolResult {
    match result {
        Ok(value) => CallToolResult::structured(value),
        Err(reason) => CallToolResult::error(vec![ContentBlock::text(reason)]),
    }
}

/// The words of the error a focus start would be refused with, by the imaging
/// view's state: 031's words for a camera that is not `ready`, which the
/// imaging module keeps private to its own modules (FR-035). Both focus tools
/// use them where the view has no `focus`.
fn focus_state_error(state: &str, error: Option<&str>) -> String {
    match state {
        "ready" => "this console has no imaging solver".to_owned(),
        "no_service" => "this scope node has no imaging service".to_owned(),
        "no_camera" => "this scope node has no imaging camera".to_owned(),
        "error" => format!(
            "the imaging camera is not answering: {}",
            error.unwrap_or_default()
        ),
        _ => "the scope node is not connected".to_owned(),
    }
}

/// The focus view as a tool answer, or the error a start would be refused with
/// when the view has no `focus` (FR-035, FR-036).
fn focus_answer(view: crate::imaging::ImagingView) -> Result<Value, String> {
    match view.focus {
        Some(focus) => Ok(serde_json::to_value(focus).expect("focus view JSON")),
        None => Err(focus_state_error(view.state, view.error.as_deref())),
    }
}

impl Console {
    fn mount(&self) -> Result<crate::link::MountClient, String> {
        self.state
            .link()
            .and_then(|link| link.mount())
            .ok_or_else(|| "no mount is connected".to_owned())
    }

    async fn stop_result(
        &self,
        name: &str,
        arguments: JsonObject,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> CallToolResult {
        let carried_out = context
            .extensions
            .get::<axum::http::request::Parts>()
            .and_then(|parts| {
                parts
                    .extensions
                    .get::<super::door::StopsCarriedOut>()
                    .map(|stops| stops.0.clone())
            });
        let result = if let Some(result) = carried_out {
            result
        } else {
            let result = api::stop(&self.state).await;
            self.state
                .log_call(name, &Value::Object(arguments), Ok(&result));
            result
        };
        CallToolResult::structured(result)
    }

    pub(crate) fn new(state: AppState) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl Console {
    #[tool(description = "Describe the connected mount's two axes.",
        output_schema = object_schema(), annotations(read_only_hint = true))]
    async fn describe_axes(&self) -> CallToolResult {
        let result = async {
            let mut mount = self.mount()?;
            let description = mount
                .describe_mount()
                .await
                .map_err(|error| error.message().to_owned())?;
            let axes: Vec<_> = description
                .axes
                .into_iter()
                .zip(["azimuth", "altitude"])
                .map(|(axis, name)| {
                    json!({
                        "axis":name, "counts_per_revolution":axis.counts_per_revolution,
                        "top_rate_deg_per_s":axis.top_rate_deg_per_s, "can_goto":axis.can_goto
                    })
                })
                .collect();
            Ok(json!({"axes":axes}))
        }
        .await;
        tool_result(result)
    }

    #[tool(description = "Read each mount axis's position, rate and moving flag.",
        output_schema = object_schema(), annotations(read_only_hint = true))]
    async fn get_axis_state(&self) -> CallToolResult {
        let result = async {
            let mut mount = self.mount()?;
            let mut axes = Vec::new();
            for (axis, name) in [
                (scope_link::proto::Axis::Azimuth, "azimuth"),
                (scope_link::proto::Axis::Altitude, "altitude"),
            ] {
                let reading = mount
                    .axis_state(axis)
                    .await
                    .map_err(|error| error.message().to_owned())?;
                axes.push(json!({"axis":name, "position":reading.position,
                    "rate_deg_per_s":reading.rate_deg_per_s, "moving":reading.moving}));
            }
            Ok(json!({"axes":axes}))
        }
        .await;
        tool_result(result)
    }

    #[tool(description = "Move a real telescope to the session's target immediately, through the console's guarded GoTo.",
        output_schema = object_schema())]
    async fn goto_target(&self, arguments: JsonObject) -> CallToolResult {
        let result = async {
            self.state.limit_move()?;
            api::start_motion(&self.state, "mcp", "goto_target", || {
                api::goto_target(&self.state)
            })
            .await
            .map_err(|error| error.message)?;
            Ok(json!({"started":true}))
        }
        .await;
        self.state.log_call(
            "goto_target",
            &Value::Object(arguments),
            result.as_ref().map_err(String::as_str),
        );
        tool_result(result)
    }

    #[tool(description = "Move a real telescope immediately to an absolute axis position in counts, through all console guards.",
        output_schema = object_schema())]
    async fn goto_axis_position(
        &self,
        Parameters(args): Parameters<PositionArguments>,
    ) -> CallToolResult {
        let result = async {
            self.state.limit_move()?;
            super::guard::start(
                &self.state,
                "goto_axis_position",
                crate::goto::AxisMove::Position {
                    axis: args.axis.into(),
                    position: args.position,
                },
            )
            .await
            .map_err(|error| error.message)?;
            Ok(json!({"started":true}))
        }
        .await;
        self.state.log_call(
            "goto_axis_position",
            &json!(args),
            result.as_ref().map_err(String::as_str),
        );
        tool_result(result)
    }

    #[tool(description = "Move a real telescope immediately at a signed axis rate for at most 30 seconds, capped at its top rate and held to all console guards.",
        output_schema = object_schema())]
    async fn move_axis(&self, Parameters(args): Parameters<RateArguments>) -> CallToolResult {
        let result = async {
            self.state.limit_move()?;
            if !args.duration_s.is_finite() || args.duration_s <= 0.0 || args.duration_s > 30.0 {
                return Err("duration_s must be more than 0 and at most 30".into());
            }
            super::guard::start(
                &self.state,
                "move_axis",
                crate::goto::AxisMove::Rate {
                    axis: args.axis.into(),
                    deg_per_s: args.deg_per_s,
                    duration_s: args.duration_s,
                },
            )
            .await
            .map_err(|error| error.message)?;
            Ok(json!({"started":true}))
        }
        .await;
        self.state.log_call(
            "move_axis",
            &json!(args),
            result.as_ref().map_err(String::as_str),
        );
        tool_result(result)
    }

    #[tool(description = "Move a real telescope by turning Track on for the session's target, or stop all motion by turning it off, as the page's Track toggle does. Turning Track on while it follows a push by hand sends the telescope back to the target.",
        output_schema = object_schema())]
    async fn set_tracking(
        &self,
        Parameters(args): Parameters<TrackingArguments>,
    ) -> CallToolResult {
        let result = async {
            if args.on {
                self.state.limit_move()?;
                api::start_motion(&self.state, "mcp", "set_tracking", || {
                    api::start_tracking(&self.state)
                })
                .await
                .map_err(|error| error.message)?;
            } else {
                api::stop(&self.state).await;
            }
            Ok(
                serde_json::to_value(api::session_response(&self.state).await)
                    .expect("session JSON"),
            )
        }
        .await;
        self.state.log_call(
            "set_tracking",
            &json!(args),
            result.as_ref().map_err(String::as_str),
        );
        tool_result(result)
    }

    #[tool(description = "Stop all telescope motion, including GoTo and tracking, in every state. A reply with acknowledged: false means the scope node did not answer and the mount may still be moving; call it again.",
        output_schema = object_schema())]
    async fn stop(
        &self,
        arguments: JsonObject,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> CallToolResult {
        self.stop_result("stop", arguments, context).await
    }

    #[tool(description = "Stop both axes and end any console GoTo or tracking in every state. A reply with acknowledged: false means the scope node did not answer and the mount may still be moving; call it again.",
        output_schema = object_schema())]
    async fn stop_all(
        &self,
        arguments: JsonObject,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> CallToolResult {
        self.stop_result("stop_all", arguments, context).await
    }

    #[tool(description = "Read the shared observing session.", output_schema = object_schema(), annotations(read_only_hint = true))]
    async fn get_session(&self) -> CallToolResult {
        CallToolResult::structured(
            serde_json::to_value(api::session_response(&self.state).await).expect("session JSON"),
        )
    }

    #[tool(description = "Set or clear only the observing session's target.",
        input_schema = rmcp::handler::server::common::schema_for_input::<TargetArguments>().expect("target schema"),
        output_schema = object_schema())]
    async fn set_target(&self, arguments: JsonObject) -> CallToolResult {
        let result = (|| {
            let bytes = Value::Object(arguments).to_string();
            let Json(edit) = Json::<SessionEdit>::from_bytes(bytes.as_bytes())
                .map_err(api::session_edit_error)
                .map_err(|error| error.message)?;
            // Exhaustive: a new session field must be considered here explicitly.
            let SessionEdit {
                imaging_camera,
                site,
                min_alt_deg,
                target: _,
                node,
                camera_settings,
                chart_rotation_deg,
            } = &edit;
            if imaging_camera.is_some()
                || site.is_some()
                || min_alt_deg.is_some()
                || node.is_some()
                || camera_settings.is_some()
                || chart_rotation_deg.is_some()
            {
                return Err("set_target changes only target".to_owned());
            }
            api::edit_session(&self.state, edit).map_err(|error| error.message)
        })();
        match result {
            Ok(session) => {
                CallToolResult::structured(serde_json::to_value(session).expect("session JSON"))
            }
            Err(reason) => CallToolResult::error(vec![ContentBlock::text(reason)]),
        }
    }

    #[tool(description = "Change only the observing session's site, minimum altitude and target. The site cannot change while a mount is connected.",
        input_schema = rmcp::handler::server::common::schema_for_input::<SessionArguments>().expect("session schema"),
        output_schema = object_schema())]
    async fn set_session(&self, arguments: JsonObject) -> CallToolResult {
        let result = async {
            let bytes = Value::Object(arguments).to_string();
            let Json(edit) = Json::<SessionEdit>::from_bytes(bytes.as_bytes())
                .map_err(api::session_edit_error)
                .map_err(|error| error.message)?;
            // Exhaustive: a new session field must be considered here explicitly.
            let SessionEdit {
                imaging_camera,
                site,
                min_alt_deg: _,
                target: _,
                node,
                camera_settings,
                chart_rotation_deg,
            } = &edit;
            if imaging_camera.is_some()
                || node.is_some()
                || camera_settings.is_some()
                || chart_rotation_deg.is_some()
            {
                return Err("set_session changes only site, min_alt_deg and target".to_owned());
            }
            if site.is_some()
                && api::session_response(&self.state)
                    .await
                    .session
                    .mount
                    .is_some_and(|mount| mount.connected)
            {
                return Err("the site cannot change while a mount is connected".to_owned());
            }
            api::edit_session(&self.state, edit).map_err(|error| error.message)
        }
        .await;
        match result {
            Ok(session) => {
                CallToolResult::structured(serde_json::to_value(session).expect("session JSON"))
            }
            Err(reason) => CallToolResult::error(vec![ContentBlock::text(reason)]),
        }
    }

    #[tool(description = "Search the target catalog by name or catalog id.",
        output_schema = object_schema(), annotations(read_only_hint = true))]
    async fn search_targets(
        &self,
        Parameters(args): Parameters<SearchArguments>,
    ) -> CallToolResult {
        match api::search(&self.state, &args.q, args.limit) {
            Ok(targets) => field_result(
                "targets",
                serde_json::to_value(targets).expect("targets JSON"),
            ),
            Err(error) => CallToolResult::error(vec![ContentBlock::text(error.message)]),
        }
    }

    #[tool(description = "Read the target's pointability and push-to turn and tilt.",
        output_schema = object_schema(), annotations(read_only_hint = true))]
    async fn get_guidance(&self) -> CallToolResult {
        let guidance = api::session_response(&self.state).await.session.guidance;
        field_result(
            "guidance",
            serde_json::to_value(guidance).expect("guidance JSON"),
        )
    }

    #[tool(description = "Solve a saved image encoded as standard base64 and set the latest pointing, as POST /api/solve does. The decoded image limit is 32 MiB.",
        output_schema = object_schema())]
    async fn solve_frame(&self, Parameters(args): Parameters<FrameArguments>) -> CallToolResult {
        let result = async {
            let bytes = BASE64_STANDARD
                .decode(args.image)
                .map_err(|error| format!("the image is not valid base64: {error}"))?;
            if bytes.len() > api::SOLVE_BODY_LIMIT {
                return Err("the image is larger than POST /api/solve's 32 MiB limit".to_owned());
            }
            let permit = self
                .state
                .solve_permit()
                .await
                .map_err(|error| error.message)?;
            api::solve_upload(
                &self.state,
                permit,
                bytes,
                args.name,
                crate::solve::SolveOptions::default(),
            )
            .await
            .map_err(|error| error.message)
        }
        .await;
        match result {
            Ok(report) => {
                CallToolResult::structured(serde_json::to_value(report).expect("solve report JSON"))
            }
            Err(reason) => CallToolResult::error(vec![ContentBlock::text(reason)]),
        }
    }

    #[tool(description = "Read the latest pointing, updated by live solves while the live loop runs.",
        output_schema = object_schema(), annotations(read_only_hint = true))]
    async fn get_pointing(&self) -> CallToolResult {
        let pointing = api::session_response(&self.state).await.session.pointing;
        field_result(
            "pointing",
            serde_json::to_value(pointing).expect("pointing JSON"),
        )
    }

    #[tool(description = "Read the scope link's health and live loop state as reported by the session's node.",
        output_schema = object_schema(), annotations(read_only_hint = true))]
    async fn get_link(&self) -> CallToolResult {
        let node = api::session_response(&self.state).await.session.node;
        field_result("node", serde_json::to_value(node).expect("node JSON"))
    }

    #[tool(description = "Read the imaging camera's state, description, status, settings and latest imaging frame.",
        output_schema = object_schema(), annotations(read_only_hint = true))]
    async fn get_imaging_camera(&self) -> CallToolResult {
        let view = match self.state.imaging() {
            Some(imaging) => imaging.view(),
            None => crate::imaging::ImagingView::off(),
        };
        CallToolResult::structured(serde_json::to_value(view).expect("imaging view JSON"))
    }

    #[tool(description = "Change the imaging camera's exposure, gain, offset, binning, region, cooler and forwarding settings, as PUT /api/imaging/settings does.",
        input_schema = rmcp::handler::server::common::schema_for_input::<ImagingSettingsArguments>().expect("imaging settings schema"),
        output_schema = object_schema())]
    async fn set_imaging_camera(&self, arguments: JsonObject) -> CallToolResult {
        let result = async {
            let bytes = Value::Object(arguments).to_string();
            let edit = api::sim_json(Json::<crate::imaging::SettingsEdit>::from_bytes(
                bytes.as_bytes(),
            ))
            .map_err(|error| error.message)?;
            let imaging = self
                .state
                .imaging()
                .ok_or_else(|| "this console takes no imaging frames".to_owned())?;
            imaging.set(edit).await.map_err(|refusal| refusal.message)
        }
        .await;
        match result {
            Ok(view) => {
                CallToolResult::structured(serde_json::to_value(view).expect("imaging view JSON"))
            }
            Err(reason) => CallToolResult::error(vec![ContentBlock::text(reason)]),
        }
    }

    #[tool(description = "Take one imaging frame, as POST /api/imaging/frame does. It waits until the imaging frame is saved and returns it with a colour JPEG preview.",
        output_schema = object_schema())]
    async fn take_imaging_frame(&self, arguments: JsonObject) -> CallToolResult {
        if !arguments.is_empty() {
            return CallToolResult::error(vec![ContentBlock::text(
                "take_imaging_frame takes no arguments",
            )]);
        }
        let Some(imaging) = self.state.imaging() else {
            return CallToolResult::error(vec![ContentBlock::text(
                "this console takes no imaging frames",
            )]);
        };
        let result = async {
            let view = imaging
                .take("frame")
                .await
                .map_err(|refusal| refusal.message)?;
            let id = view.frame.as_ref().expect("a frame in the view").id;
            Ok::<_, String>(imaging.ended(id).await)
        }
        .await;
        match result {
            Err(reason) => CallToolResult::error(vec![ContentBlock::text(reason)]),
            Ok(frame) if frame.state == "saved" => {
                let id = frame.id;
                let mut view = imaging.view();
                view.frame = Some(frame);
                let mut result = CallToolResult::structured(
                    serde_json::to_value(view).expect("imaging view JSON"),
                );
                if let Some(preview) = imaging.preview(id) {
                    result.content.push(ContentBlock::image(
                        BASE64_STANDARD.encode(preview),
                        "image/jpeg",
                    ));
                }
                result
            }
            Ok(frame) if frame.state == "aborted" => {
                CallToolResult::error(vec![ContentBlock::text("the exposure was aborted")])
            }
            Ok(frame) => CallToolResult::error(vec![ContentBlock::text(
                frame.error.expect("a failed frame carries its error"),
            )]),
        }
    }

    #[tool(description = "Solve the latest imaging frame, as POST /api/imaging/solve does. It waits until the solve ends and returns the imaging camera's state with the solve.",
        output_schema = object_schema())]
    async fn solve_imaging_frame(&self, arguments: JsonObject) -> CallToolResult {
        if !arguments.is_empty() {
            return CallToolResult::error(vec![ContentBlock::text(
                "solve_imaging_frame takes no arguments",
            )]);
        }
        let Some(imaging) = self.state.imaging() else {
            return CallToolResult::error(vec![ContentBlock::text(
                "this console takes no imaging frames",
            )]);
        };
        match imaging.solve().await {
            Err(refusal) => CallToolResult::error(vec![ContentBlock::text(refusal.message)]),
            Ok(view) => {
                let id = view.solve.as_ref().expect("a solve in the view").frame_id;
                let solve = imaging.solved(id).await;
                if solve.state == "solved" {
                    CallToolResult::structured(
                        serde_json::to_value(imaging.view()).expect("imaging view JSON"),
                    )
                } else {
                    CallToolResult::error(vec![ContentBlock::text(
                        solve.error.expect("a failed solve carries its error"),
                    )])
                }
            }
        }
    }

    #[tool(description = "Read the focus reading: the focus region, the focus record, and the reading, best, trend and verdict of the focus aid or the last focus check.",
        output_schema = object_schema(), annotations(read_only_hint = true))]
    async fn get_focus_reading(&self, arguments: JsonObject) -> CallToolResult {
        let result = async {
            if !arguments.is_empty() {
                return Err("get_focus_reading takes no arguments".to_owned());
            }
            let imaging = self
                .state
                .imaging()
                .ok_or_else(|| "this console takes no imaging frames".to_owned())?;
            focus_answer(imaging.view())
        };
        tool_result(result.await)
    }

    #[tool(description = "Start or stop the focus aid, as POST and DELETE /api/imaging/focus do. With check true it takes a focus check, waits until it ends and returns its verdict against the focus record.",
        input_schema = rmcp::handler::server::common::schema_for_input::<FocusAidArguments>().expect("focus aid schema"),
        output_schema = object_schema())]
    async fn set_focus_aid(&self, arguments: JsonObject) -> CallToolResult {
        let result = async {
            let Some(on) = arguments.get("on").and_then(Value::as_bool) else {
                return Err("on must be true or false".to_owned());
            };
            if !on {
                if arguments.len() > 1 {
                    return Err("set_focus_aid with on false takes no other argument".to_owned());
                }
                let imaging = self
                    .state
                    .imaging()
                    .ok_or_else(|| "this console takes no imaging frames".to_owned())?;
                return focus_answer(imaging.focus_stop().await);
            }
            // With `on` true the other keys are the route's own body, so the
            // tool refuses with the route's words wherever the words are the
            // console's own (FR-036).
            let mut rest = arguments.clone();
            rest.remove("on");
            let ask: crate::imaging::focus::run::FocusAsk =
                serde_json::from_value(Value::Object(rest)).map_err(|error| error.to_string())?;
            let imaging = self
                .state
                .imaging()
                .ok_or_else(|| "this console takes no imaging frames".to_owned())?;
            let check = ask.check;
            let view = imaging
                .focus_start(ask)
                .await
                .map_err(|refusal| refusal.message)?;
            if !check {
                return focus_answer(view);
            }
            // A focus check: the answer is its verdict, once it has ended.
            match imaging.focus_ended().await {
                Some(focus) => Ok(serde_json::to_value(focus).expect("focus view JSON")),
                None => focus_answer(imaging.view()),
            }
        };
        tool_result(result.await)
    }

    #[tool(description = "List the calibration library's master darks and master flats, the set being taken, and what matches the latest imaging frame.",
        output_schema = object_schema(), annotations(read_only_hint = true))]
    async fn list_calibration(&self) -> CallToolResult {
        let view = match self.state.calibration() {
            None => CalibrationView::empty(),
            Some(calibration) => self
                .state
                .imaging()
                .and_then(|imaging| imaging.view().calibration)
                .unwrap_or_else(|| calibration.view(None)),
        };
        CallToolResult::structured(serde_json::to_value(view).expect("calibration view JSON"))
    }

    #[tool(description = "Take a set of darks with the telescope covered and add their master dark to the calibration library, as POST /api/imaging/calibration/darks does; anyway: true passes the dark test. It answers once the set has started; read its progress with list_calibration.",
        input_schema = rmcp::handler::server::common::schema_for_input::<DarksArguments>().expect("darks schema"),
        output_schema = object_schema())]
    async fn take_darks(&self, arguments: JsonObject) -> CallToolResult {
        let result = async {
            let calibration = self
                .state
                .calibration()
                .ok_or_else(|| "this console takes no imaging frames".to_owned())?;
            let bytes = Value::Object(arguments).to_string();
            let request = api::sim_json(Json::<DarksRequest>::from_bytes(bytes.as_bytes()))
                .map_err(|error| error.message)?;
            let view = calibration
                .take_darks(request)
                .await
                .map_err(|refusal| refusal.message)?;
            Ok(serde_json::to_value(view).expect("calibration view JSON"))
        };
        tool_result(result.await)
    }

    #[tool(description = "Take a set of flats of a lit flat panel, or with darks: true the darks that finish them, as POST /api/imaging/calibration/flats does; anyway: true passes the dark test of those darks. It answers once the step has started; read its progress with list_calibration.",
        input_schema = rmcp::handler::server::common::schema_for_input::<FlatsArguments>().expect("flats schema"),
        output_schema = object_schema())]
    async fn take_flats(&self, arguments: JsonObject) -> CallToolResult {
        let result = async {
            let calibration = self
                .state
                .calibration()
                .ok_or_else(|| "this console takes no imaging frames".to_owned())?;
            let bytes = Value::Object(arguments).to_string();
            let request = api::sim_json(Json::<FlatsRequest>::from_bytes(bytes.as_bytes()))
                .map_err(|error| error.message)?;
            let view = calibration
                .take_flats(request)
                .await
                .map_err(|refusal| refusal.message)?;
            Ok(serde_json::to_value(view).expect("calibration view JSON"))
        };
        tool_result(result.await)
    }

    #[tool(description = "Start or stop the live stack of subs, as POST and DELETE /api/imaging/stack do. With on true it also takes reject, focus_gate, gains and stretch; on a running stack it changes gains and stretch.",
        input_schema = rmcp::handler::server::common::schema_for_input::<StackingArguments>().expect("stacking schema"),
        output_schema = object_schema())]
    async fn set_stacking(&self, mut arguments: JsonObject) -> CallToolResult {
        let result = async {
            let on = arguments
                .get("on")
                .and_then(Value::as_bool)
                .ok_or_else(|| "on must be true or false".to_owned())?;
            if !on && arguments.len() > 1 {
                return Err("on false takes no other argument".to_owned());
            }
            // Judge the route body's shape before checking for imaging, and
            // remove the tool-only key before StartRequest's unknown-key check.
            arguments.remove("on");
            let request = if on {
                serde_json::from_value::<StartRequest>(Value::Object(arguments))
                    .map_err(|error| error.to_string())?
            } else {
                StartRequest::default()
            };
            let stacker = self
                .state
                .stack()
                .ok_or_else(|| "this console takes no imaging frames".to_owned())?;
            let view = if on {
                Some(
                    stacker
                        .start(request, self.state.target_label().as_deref())
                        .await
                        .map_err(|refusal| refusal.message)?,
                )
            } else {
                stacker.stop().await
            };
            Ok(view.map_or_else(
                || json!({"state":"none"}),
                |view| serde_json::to_value(view).expect("stack view JSON"),
            ))
        };
        tool_result(result.await)
    }

    #[tool(description = "Read the live stack: its state, counts, dropped subs by reason and, when it has one, its picture as a colour JPEG.",
        output_schema = object_schema(), annotations(read_only_hint = true))]
    async fn get_stack(&self, arguments: JsonObject) -> CallToolResult {
        if !arguments.is_empty() {
            return CallToolResult::error(vec![ContentBlock::text("get_stack takes no arguments")]);
        }
        let stacker = self.state.stack();
        let view = stacker.as_ref().and_then(|stacker| stacker.view());
        let mut result = CallToolResult::structured(view.map_or_else(
            || json!({"state":"none"}),
            |view| serde_json::to_value(view).expect("stack view JSON"),
        ));
        if let Some(stacker) = stacker {
            if let Some(preview) =
                tokio::task::spawn_blocking(move || stacker.preview_within(display::MCP_WIDTH))
                    .await
                    .expect("stack preview task")
            {
                result.content.push(ContentBlock::image(
                    BASE64_STANDARD.encode(preview),
                    "image/jpeg",
                ));
            }
        }
        result
    }

    #[tool(description = "Set what steers the tracker during a live stack, as PUT /api/imaging/steering does: on, smear_px, step_share and rate_share, each optional. Read the result in the steering part of get_imaging_camera.",
        input_schema = rmcp::handler::server::common::schema_for_input::<SteeringArguments>().expect("steering schema"),
        output_schema = object_schema())]
    async fn set_steering(&self, arguments: JsonObject) -> CallToolResult {
        let result = (|| {
            let edit = serde_json::from_value::<SteeringEdit>(Value::Object(arguments))
                .map_err(|error| error.to_string())?;
            let imaging = self
                .state
                .imaging()
                .ok_or_else(|| "this console takes no imaging frames".to_owned())?;
            let view = crate::imaging::routes::apply_steering(&imaging, edit)?;
            Ok(serde_json::to_value(view).expect("imaging view JSON"))
        })();
        tool_result(result)
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for Console {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("finder", env!("CARGO_PKG_VERSION")))
    }
}
