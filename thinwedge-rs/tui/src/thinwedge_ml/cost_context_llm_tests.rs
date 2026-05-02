use super::ArtificialAnalysisModel;
use super::LlmCostSnapshot;
use super::LlmSortBy;
use super::coding_index;
use super::intelligence_index;
use super::snapshot_for_model;
use super::sort_models;
use super::types::ArtificialAnalysisCreator;
use super::types::ArtificialAnalysisPricing;
use pretty_assertions::assert_eq;
use serde_json::json;

fn sample_model(
    id: &str,
    name: &str,
    price: f64,
    intelligence: f64,
    coding: f64,
    speed: f64,
    latency: f64,
) -> ArtificialAnalysisModel {
    ArtificialAnalysisModel {
        id: id.to_string(),
        name: name.to_string(),
        slug: id.to_string(),
        model_creator: ArtificialAnalysisCreator {
            id: "creator-1".to_string(),
            name: "ThinWedge".to_string(),
            slug: Some("thinwedge".to_string()),
        },
        pricing: Some(ArtificialAnalysisPricing {
            price_1m_blended_3_to_1: Some(price),
            price_1m_input_tokens: Some(price / 2.0),
            price_1m_output_tokens: Some(price * 2.0),
        }),
        evaluations: json!({
            "artificial_analysis_intelligence_index": intelligence,
            "artificial_analysis_coding_index": coding
        }),
        median_output_tokens_per_second: Some(speed),
        median_time_to_first_token_seconds: Some(latency),
    }
}

#[test]
fn snapshot_extracts_cost_and_benchmark_data() {
    let model = sample_model("model-a", "Model A", 2.0, 40.0, 35.0, 120.0, 3.0);

    assert_eq!(
        snapshot_for_model(&model),
        LlmCostSnapshot {
            blended_price_per_1_m_tokens_usd: Some(2.0),
            input_price_per_1_m_tokens_usd: Some(1.0),
            output_price_per_1_m_tokens_usd: Some(4.0),
            median_output_tokens_per_second: Some(120.0),
            median_time_to_first_token_seconds: Some(3.0),
            intelligence_index: Some(40.0),
            coding_index: Some(35.0),
        }
    );
    assert_eq!(intelligence_index(&model), Some(40.0));
    assert_eq!(coding_index(&model), Some(35.0));
}

#[test]
fn sort_models_supports_benchmark_and_price_modes() {
    let mut models = vec![
        sample_model("cheap", "Cheap", 1.0, 30.0, 20.0, 50.0, 4.0),
        sample_model("best", "Best", 5.0, 60.0, 55.0, 80.0, 5.0),
        sample_model("coder", "Coder", 2.0, 40.0, 70.0, 90.0, 3.0),
    ];

    sort_models(&mut models, Some(LlmSortBy::BlendedPrice));
    assert_eq!(models[0].id, "cheap");

    sort_models(&mut models, Some(LlmSortBy::Intelligence));
    assert_eq!(models[0].id, "best");

    sort_models(&mut models, Some(LlmSortBy::Coding));
    assert_eq!(models[0].id, "coder");
}
