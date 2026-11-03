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
            .is_some_and(|parts| {
                parts
                    .extensions
                    .get::<super::door::StopsCarriedOut>()
                    .is_some()
            });
        let result = if carried_out {
            json!({"stopped":true})
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

    #[tool(description = "Stop all telescope motion, including GoTo and tracking, in every state.",
        output_schema = object_schema())]
    async fn stop(
        &self,
        arguments: JsonObject,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> CallToolResult {
        self.stop_result("stop", arguments, context).await
    }

    #[tool(description = "Stop both axes and end any console GoTo or tracking in every state.",
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
            } = &edit;
            if imaging_camera.is_some()
                || site.is_some()
                || min_alt_deg.is_some()
                || node.is_some()
                || camera_settings.is_some()
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
            } = &edit;
            if imaging_camera.is_some() || node.is_some() || camera_settings.is_some() {
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
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for Console {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("finder", env!("CARGO_PKG_VERSION")))
    }
}
