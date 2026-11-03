//! Thin tools over the same functions the page's routes call.
use std::sync::Arc;

use axum::Json;
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

impl Console {
    pub(crate) fn new(state: AppState) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl Console {
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
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for Console {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("finder", env!("CARGO_PKG_VERSION")))
    }
}
