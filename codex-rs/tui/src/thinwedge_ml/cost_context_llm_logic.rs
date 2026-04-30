use super::ArtificialAnalysisModel;
use super::LlmCostSnapshot;
use super::LlmSortBy;
use super::ModelLookup;

pub(super) fn filter_models(
    models: &mut Vec<ArtificialAnalysisModel>,
    creator_id: Option<&str>,
    search: Option<&str>,
) {
    if let Some(creator_id) = creator_id {
        models.retain(|model| {
            model.model_creator.id.eq_ignore_ascii_case(creator_id)
                || model
                    .model_creator
                    .slug
                    .as_deref()
                    .is_some_and(|slug| slug.eq_ignore_ascii_case(creator_id))
        });
    }
    if let Some(search) = search {
        let query = search.to_ascii_lowercase();
        models.retain(|model| {
            model.id.to_ascii_lowercase().contains(&query)
                || model.slug.to_ascii_lowercase().contains(&query)
                || model.name.to_ascii_lowercase().contains(&query)
                || model
                    .model_creator
                    .name
                    .to_ascii_lowercase()
                    .contains(&query)
        });
    }
}

pub(super) fn sort_models(models: &mut [ArtificialAnalysisModel], sort_by: Option<LlmSortBy>) {
    match sort_by {
        Some(LlmSortBy::BlendedPrice) => models.sort_by_key(|model| {
            sort_key_ascending(
                model
                    .pricing
                    .as_ref()
                    .and_then(|pricing| pricing.price_1m_blended_3_to_1),
            )
        }),
        Some(LlmSortBy::InputPrice) => models.sort_by_key(|model| {
            sort_key_ascending(
                model
                    .pricing
                    .as_ref()
                    .and_then(|pricing| pricing.price_1m_input_tokens),
            )
        }),
        Some(LlmSortBy::OutputPrice) => models.sort_by_key(|model| {
            sort_key_ascending(
                model
                    .pricing
                    .as_ref()
                    .and_then(|pricing| pricing.price_1m_output_tokens),
            )
        }),
        Some(LlmSortBy::Latency) => {
            models.sort_by_key(|model| sort_key_ascending(model.median_time_to_first_token_seconds))
        }
        Some(LlmSortBy::Speed) => models.sort_by_key(|model| {
            std::cmp::Reverse(sort_key_ascending(model.median_output_tokens_per_second))
        }),
        Some(LlmSortBy::Intelligence) => models
            .sort_by_key(|model| std::cmp::Reverse(sort_key_ascending(intelligence_index(model)))),
        Some(LlmSortBy::Coding) => {
            models.sort_by_key(|model| std::cmp::Reverse(sort_key_ascending(coding_index(model))))
        }
        None => {}
    }
}

pub(super) fn resolve_lookup(
    models: &[ArtificialAnalysisModel],
    lookup: &ModelLookup,
) -> Result<ArtificialAnalysisModel, String> {
    let identifier = lookup
        .model_id
        .as_deref()
        .or(lookup.slug.as_deref())
        .or(lookup.name.as_deref())
        .ok_or_else(|| {
            "one of `modelId`, `slug`, or `name` is required for llmcosts lookups".to_string()
        })?;
    models
        .iter()
        .find(|model| {
            lookup
                .model_id
                .as_deref()
                .is_some_and(|model_id| model.id.eq_ignore_ascii_case(model_id))
                || lookup
                    .slug
                    .as_deref()
                    .is_some_and(|slug| model.slug.eq_ignore_ascii_case(slug))
                || lookup
                    .name
                    .as_deref()
                    .is_some_and(|name| model.name.eq_ignore_ascii_case(name))
        })
        .cloned()
        .ok_or_else(|| {
            format!("Artificial Analysis did not return an LLM entry matching `{identifier}`")
        })
}

pub(super) fn snapshot_for_model(model: &ArtificialAnalysisModel) -> LlmCostSnapshot {
    LlmCostSnapshot {
        blended_price_per_1_m_tokens_usd: model
            .pricing
            .as_ref()
            .and_then(|pricing| pricing.price_1m_blended_3_to_1),
        input_price_per_1_m_tokens_usd: model
            .pricing
            .as_ref()
            .and_then(|pricing| pricing.price_1m_input_tokens),
        output_price_per_1_m_tokens_usd: model
            .pricing
            .as_ref()
            .and_then(|pricing| pricing.price_1m_output_tokens),
        median_output_tokens_per_second: model.median_output_tokens_per_second,
        median_time_to_first_token_seconds: model.median_time_to_first_token_seconds,
        intelligence_index: intelligence_index(model),
        coding_index: coding_index(model),
    }
}

pub(super) fn intelligence_index(model: &ArtificialAnalysisModel) -> Option<f64> {
    model
        .evaluations
        .get("artificial_analysis_intelligence_index")
        .and_then(serde_json::Value::as_f64)
}

pub(super) fn coding_index(model: &ArtificialAnalysisModel) -> Option<f64> {
    model
        .evaluations
        .get("artificial_analysis_coding_index")
        .and_then(serde_json::Value::as_f64)
}

fn sort_key_ascending(value: Option<f64>) -> i64 {
    let scaled = value.unwrap_or(f64::INFINITY) * 1_000_000.0;
    if scaled.is_infinite() {
        i64::MAX
    } else {
        scaled as i64
    }
}
