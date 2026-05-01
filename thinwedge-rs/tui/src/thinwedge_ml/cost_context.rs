#[path = "cost_context_infra.rs"]
mod infra;
#[path = "cost_context_llm.rs"]
mod llm;

use serde_json::Value as JsonValue;
use thinwedge_app_server_protocol::DynamicToolSpec;

pub(super) fn dynamic_tool_specs() -> Vec<DynamicToolSpec> {
    let mut specs = llm::dynamic_tool_specs();
    specs.extend(infra::dynamic_tool_specs());
    specs
}

pub(super) async fn handle_dynamic_tool_call(
    namespace: &str,
    tool: &str,
    arguments: JsonValue,
) -> Result<String, String> {
    match namespace {
        "llmcosts" => llm::handle_dynamic_tool_call(tool, arguments).await,
        "infracosts" => infra::handle_dynamic_tool_call(tool, arguments).await,
        _ => Err(format!(
            "Unsupported ThinWedge cost namespace `{namespace}`"
        )),
    }
}
