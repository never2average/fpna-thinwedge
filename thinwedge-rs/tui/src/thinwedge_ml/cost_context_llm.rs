use thinwedge_app_server_protocol::DynamicToolSpec;
use serde::Serialize;
use serde_json::Value as JsonValue;
use serde_json::json;

#[path = "cost_context_llm_logic.rs"]
mod logic;
#[path = "cost_context_llm_types.rs"]
mod types;

#[cfg(test)]
use logic::coding_index;
use logic::filter_models;
#[cfg(test)]
use logic::intelligence_index;
use logic::resolve_lookup;
use logic::snapshot_for_model;
use logic::sort_models;
use types::ArtificialAnalysisModel;
use types::ArtificialAnalysisResponse;
use types::CompareModelsArgs;
use types::ListModelsArgs;
use types::LlmCostSnapshot;
use types::LlmSortBy;
use types::ModelLookup;

const ARTIFICIAL_ANALYSIS_MODELS_URL: &str =
    "https://artificialanalysis.ai/api/v2/data/llms/models";
const ARTIFICIAL_ANALYSIS_API_KEY_ENV: &str = "ARTIFICIAL_ANALYSIS_API_KEY";

pub(super) fn dynamic_tool_specs() -> Vec<DynamicToolSpec> {
    vec![
        DynamicToolSpec {
            namespace: Some("llmcosts".to_string()),
            name: "listModels".to_string(),
            description:
                "List LLM market context from Artificial Analysis, including price and speed."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "creatorId": { "type": "string" },
                    "search": { "type": "string" },
                    "sortBy": {
                        "type": "string",
                        "enum": [
                            "blendedPrice",
                            "inputPrice",
                            "outputPrice",
                            "speed",
                            "latency",
                            "intelligence",
                            "coding"
                        ]
                    },
                    "limit": { "type": "integer", "minimum": 1 }
                },
                "additionalProperties": false
            }),
            defer_loading: false,
        },
        DynamicToolSpec {
            namespace: Some("llmcosts".to_string()),
            name: "getModel".to_string(),
            description: "Inspect one LLM market context entry from Artificial Analysis."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "modelId": { "type": "string" },
                    "slug": { "type": "string" },
                    "name": { "type": "string" }
                },
                "additionalProperties": false
            }),
            defer_loading: false,
        },
        DynamicToolSpec {
            namespace: Some("llmcosts".to_string()),
            name: "compareModels".to_string(),
            description: "Compare multiple LLMs across price, latency, speed, and benchmarks."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "models": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "modelId": { "type": "string" },
                                "slug": { "type": "string" },
                                "name": { "type": "string" }
                            },
                            "additionalProperties": false
                        },
                        "minItems": 2
                    }
                },
                "required": ["models"],
                "additionalProperties": false
            }),
            defer_loading: false,
        },
    ]
}

pub(super) async fn handle_dynamic_tool_call(
    tool: &str,
    arguments: JsonValue,
) -> Result<String, String> {
    match tool {
        "listModels" => list_models(arguments).await,
        "getModel" => get_model(arguments).await,
        "compareModels" => compare_models(arguments).await,
        _ => Err(format!("Unsupported ThinWedge cost tool `llmcosts.{tool}`")),
    }
}

async fn list_models(arguments: JsonValue) -> Result<String, String> {
    let args: ListModelsArgs = parse_arguments(arguments)?;
    let mut models = fetch_models().await?;
    filter_models(
        &mut models,
        args.creator_id.as_deref(),
        args.search.as_deref(),
    );
    sort_models(&mut models, args.sort_by);
    if let Some(limit) = args.limit {
        models.truncate(limit);
    }
    pretty_json(&json!({
        "source": "Artificial Analysis",
        "models": models,
    }))
}

async fn get_model(arguments: JsonValue) -> Result<String, String> {
    let args: ModelLookup = parse_arguments(arguments)?;
    let model = resolve_lookup(&fetch_models().await?, &args)?;
    pretty_json(&json!({
        "source": "Artificial Analysis",
        "model": model,
        "costContext": snapshot_for_model(&model),
    }))
}

async fn compare_models(arguments: JsonValue) -> Result<String, String> {
    let args: CompareModelsArgs = parse_arguments(arguments)?;
    let models = fetch_models().await?;
    let selected = args
        .models
        .iter()
        .map(|lookup| resolve_lookup(&models, lookup))
        .collect::<Result<Vec<_>, _>>()?;
    let comparisons = selected
        .iter()
        .map(|model| {
            json!({
                "model": model,
                "costContext": snapshot_for_model(model),
            })
        })
        .collect::<Vec<_>>();
    pretty_json(&json!({
        "source": "Artificial Analysis",
        "comparisons": comparisons,
    }))
}

async fn fetch_models() -> Result<Vec<ArtificialAnalysisModel>, String> {
    let api_key = std::env::var(ARTIFICIAL_ANALYSIS_API_KEY_ENV).map_err(|_| {
        format!("ThinWedge LLM cost tool requires `{ARTIFICIAL_ANALYSIS_API_KEY_ENV}`")
    })?;
    let response = reqwest::Client::new()
        .get(ARTIFICIAL_ANALYSIS_MODELS_URL)
        .header("x-api-key", api_key)
        .send()
        .await
        .map_err(|err| format!("failed to call Artificial Analysis API: {err}"))?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<body unavailable>".to_string());
        return Err(format!(
            "Artificial Analysis API request failed with status {status}: {body}"
        ));
    }
    response
        .json::<ArtificialAnalysisResponse>()
        .await
        .map(|response| response.data)
        .map_err(|err| format!("failed to decode Artificial Analysis API response: {err}"))
}

fn parse_arguments<T>(arguments: JsonValue) -> Result<T, String>
where
    T: for<'de> serde::Deserialize<'de>,
{
    serde_json::from_value(arguments)
        .map_err(|err| format!("invalid ThinWedge LLM cost tool arguments: {err}"))
}

fn pretty_json<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string_pretty(value)
        .map_err(|err| format!("failed to encode ThinWedge LLM cost response: {err}"))
}

#[cfg(test)]
#[path = "cost_context_llm_tests.rs"]
mod tests;
