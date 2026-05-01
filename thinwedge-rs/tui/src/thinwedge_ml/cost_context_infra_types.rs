use serde::Deserialize;
use serde::Serialize;
use serde_json::Value as JsonValue;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(super) struct AwsPricingFilter {
    pub(super) field: String,
    pub(super) value: String,
    #[serde(default = "default_aws_match_type")]
    pub(super) match_type: String,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct DescribeAwsServicesArgs {
    pub(super) service_code: Option<String>,
    pub(super) max_results: Option<u32>,
    pub(super) next_token: Option<String>,
    pub(super) api_region: Option<String>,
    pub(super) profile: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SearchAwsServicesArgs {
    pub(super) search: Option<String>,
    pub(super) max_results: Option<usize>,
    pub(super) api_region: Option<String>,
    pub(super) profile: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SearchAwsPriceListAttributeNamesArgs {
    pub(super) service_code: String,
    pub(super) search: Option<String>,
    pub(super) limit: Option<usize>,
    pub(super) api_region: Option<String>,
    pub(super) profile: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SearchAwsPriceListAttributeValuesArgs {
    pub(super) service_code: String,
    pub(super) field: String,
    pub(super) search: Option<String>,
    #[serde(default)]
    pub(super) filters: Vec<AwsPricingFilter>,
    pub(super) sample_pages: Option<u32>,
    pub(super) values_limit: Option<usize>,
    pub(super) api_region: Option<String>,
    pub(super) profile: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct AwsDateInterval {
    pub(super) start: String,
    pub(super) end: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct AwsGroupBy {
    pub(super) r#type: String,
    pub(super) key: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct GetAwsCostAndUsageArgs {
    pub(super) time_period: AwsDateInterval,
    pub(super) granularity: String,
    pub(super) metrics: Vec<String>,
    pub(super) filter: Option<JsonValue>,
    #[serde(default)]
    pub(super) group_by: Vec<AwsGroupBy>,
    pub(super) billing_view_arn: Option<String>,
    pub(super) next_page_token: Option<String>,
    pub(super) api_region: Option<String>,
    pub(super) profile: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct QueryAwsByServiceArgs {
    pub(super) time_period: AwsDateInterval,
    pub(super) service: String,
    pub(super) granularity: String,
    #[serde(default = "default_cost_explorer_metrics")]
    pub(super) metrics: Vec<String>,
    pub(super) filter: Option<JsonValue>,
    #[serde(default)]
    pub(super) group_by: Vec<AwsGroupBy>,
    pub(super) billing_view_arn: Option<String>,
    pub(super) next_page_token: Option<String>,
    pub(super) api_region: Option<String>,
    pub(super) profile: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct QueryAwsByAccountArgs {
    pub(super) time_period: AwsDateInterval,
    pub(super) linked_accounts: Vec<String>,
    pub(super) granularity: String,
    #[serde(default = "default_cost_explorer_metrics")]
    pub(super) metrics: Vec<String>,
    pub(super) filter: Option<JsonValue>,
    #[serde(default)]
    pub(super) group_by: Vec<AwsGroupBy>,
    pub(super) billing_view_arn: Option<String>,
    pub(super) next_page_token: Option<String>,
    pub(super) api_region: Option<String>,
    pub(super) profile: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct GetAwsDimensionValuesArgs {
    pub(super) time_period: AwsDateInterval,
    pub(super) dimension: String,
    pub(super) context: Option<String>,
    pub(super) search_string: Option<String>,
    pub(super) filter: Option<JsonValue>,
    pub(super) max_results: Option<u32>,
    pub(super) next_page_token: Option<String>,
    pub(super) billing_view_arn: Option<String>,
    pub(super) api_region: Option<String>,
    pub(super) profile: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct GetAwsCostForecastArgs {
    pub(super) time_period: AwsDateInterval,
    pub(super) metric: String,
    pub(super) granularity: String,
    pub(super) filter: Option<JsonValue>,
    pub(super) billing_view_arn: Option<String>,
    pub(super) prediction_interval_level: Option<u32>,
    pub(super) api_region: Option<String>,
    pub(super) profile: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ListBillingViewsArgs {
    pub(super) active_after_inclusive: Option<i64>,
    pub(super) active_before_inclusive: Option<i64>,
    #[serde(default)]
    pub(super) arns: Vec<String>,
    #[serde(default)]
    pub(super) billing_view_types: Vec<String>,
    pub(super) owner_account_id: Option<String>,
    pub(super) max_results: Option<u32>,
    pub(super) next_token: Option<String>,
    pub(super) api_region: Option<String>,
    pub(super) profile: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct AwsTotalImpactFilter {
    pub(super) numeric_operator: String,
    pub(super) start_value: f64,
    pub(super) end_value: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct GetAwsAnomaliesArgs {
    pub(super) time_period: AwsDateInterval,
    pub(super) monitor_arn: Option<String>,
    pub(super) feedback: Option<String>,
    pub(super) total_impact: Option<AwsTotalImpactFilter>,
    pub(super) max_results: Option<u32>,
    pub(super) next_page_token: Option<String>,
    pub(super) api_region: Option<String>,
    pub(super) profile: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct GetAwsProductsArgs {
    pub(super) service_code: String,
    #[serde(default)]
    pub(super) filters: Vec<AwsPricingFilter>,
    pub(super) max_results: Option<u32>,
    pub(super) next_token: Option<String>,
    pub(super) api_region: Option<String>,
    pub(super) profile: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct GetAwsVmPriceArgs {
    pub(super) instance_type: String,
    pub(super) region_code: Option<String>,
    pub(super) location: Option<String>,
    pub(super) operating_system: Option<String>,
    pub(super) tenancy: Option<String>,
    pub(super) pre_installed_sw: Option<String>,
    pub(super) capacity_status: Option<String>,
    pub(super) license_model: Option<String>,
    pub(super) max_results: Option<u32>,
    pub(super) api_region: Option<String>,
    pub(super) profile: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct EstimateAwsBoqArgs {
    pub(super) line_items: Vec<AwsBoqLineItem>,
    pub(super) api_region: Option<String>,
    pub(super) profile: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct AwsBoqLineItem {
    pub(super) label: Option<String>,
    pub(super) service_code: String,
    #[serde(default)]
    pub(super) filters: Vec<AwsPricingFilter>,
    pub(super) quantity: f64,
    pub(super) expected_unit: Option<String>,
    pub(super) select_by: Option<AwsLineItemSelectBy>,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub(super) enum AwsLineItemSelectBy {
    LowestPrice,
    First,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(super) struct AwsBoqEstimateLine {
    pub(super) label: String,
    pub(super) service_code: String,
    pub(super) quantity: f64,
    pub(super) selected_unit: Option<String>,
    pub(super) selected_price_per_unit_usd: Option<f64>,
    pub(super) estimated_cost_usd: Option<f64>,
    pub(super) matched_sku: Option<String>,
    pub(super) matched_description: Option<String>,
}

#[derive(Default)]
pub(super) struct SelectedPriceDimension {
    pub(super) sku: Option<String>,
    pub(super) unit: Option<String>,
    pub(super) description: Option<String>,
    pub(super) price_per_unit_usd: Option<f64>,
}

pub(super) fn default_aws_match_type() -> String {
    "TERM_MATCH".to_string()
}

fn default_cost_explorer_metrics() -> Vec<String> {
    vec!["UnblendedCost".to_string()]
}
