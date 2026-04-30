use serde::Deserialize;
use serde::Serialize;
use serde_json::Value as JsonValue;

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ListModelsArgs {
    pub(super) creator_id: Option<String>,
    pub(super) search: Option<String>,
    pub(super) sort_by: Option<LlmSortBy>,
    pub(super) limit: Option<usize>,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ModelLookup {
    pub(super) model_id: Option<String>,
    pub(super) slug: Option<String>,
    pub(super) name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct CompareModelsArgs {
    pub(super) models: Vec<ModelLookup>,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub(super) enum LlmSortBy {
    BlendedPrice,
    InputPrice,
    OutputPrice,
    Speed,
    Latency,
    Intelligence,
    Coding,
}

#[derive(Debug, Deserialize)]
pub(super) struct ArtificialAnalysisResponse {
    pub(super) data: Vec<ArtificialAnalysisModel>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(super) struct ArtificialAnalysisModel {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) slug: String,
    pub(super) model_creator: ArtificialAnalysisCreator,
    #[serde(default)]
    pub(super) pricing: Option<ArtificialAnalysisPricing>,
    #[serde(default)]
    pub(super) evaluations: JsonValue,
    #[serde(default)]
    pub(super) median_output_tokens_per_second: Option<f64>,
    #[serde(default)]
    pub(super) median_time_to_first_token_seconds: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(super) struct ArtificialAnalysisCreator {
    pub(super) id: String,
    pub(super) name: String,
    #[serde(default)]
    pub(super) slug: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(super) struct ArtificialAnalysisPricing {
    #[serde(default)]
    pub(super) price_1m_blended_3_to_1: Option<f64>,
    #[serde(default)]
    pub(super) price_1m_input_tokens: Option<f64>,
    #[serde(default)]
    pub(super) price_1m_output_tokens: Option<f64>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(super) struct LlmCostSnapshot {
    pub(super) blended_price_per_1_m_tokens_usd: Option<f64>,
    pub(super) input_price_per_1_m_tokens_usd: Option<f64>,
    pub(super) output_price_per_1_m_tokens_usd: Option<f64>,
    pub(super) median_output_tokens_per_second: Option<f64>,
    pub(super) median_time_to_first_token_seconds: Option<f64>,
    pub(super) intelligence_index: Option<f64>,
    pub(super) coding_index: Option<f64>,
}
