use codex_app_server_protocol::DynamicToolSpec;
use serde::Serialize;
use serde_json::Value as JsonValue;
use serde_json::json;

#[path = "cost_context_infra_logic.rs"]
mod logic;
#[path = "cost_context_infra_query.rs"]
mod query;
#[path = "cost_context_infra_types.rs"]
mod types;

use logic::aws_billing_request;
use logic::aws_cost_explorer_request;
use logic::aws_get_products;
use logic::aws_pricing_request;
use logic::extract_on_demand_price_dimensions;
use logic::lowest_hourly_price_usd;
use logic::parse_aws_pricelist_entries;
use logic::select_price_dimension;
use logic::strip_nulls;
use types::AwsBoqEstimateLine;
use types::AwsDateInterval;
use types::AwsLineItemSelectBy;
use types::AwsPricingFilter;
use types::AwsTotalImpactFilter;
use types::DescribeAwsServicesArgs;
use types::EstimateAwsBoqArgs;
use types::GetAwsAnomaliesArgs;
use types::GetAwsCostAndUsageArgs;
use types::GetAwsCostForecastArgs;
use types::GetAwsDimensionValuesArgs;
use types::GetAwsProductsArgs;
use types::GetAwsVmPriceArgs;
use types::ListBillingViewsArgs;
use types::default_aws_match_type;

pub(super) fn dynamic_tool_specs() -> Vec<DynamicToolSpec> {
    let mut specs = vec![
        DynamicToolSpec {
            namespace: Some("infracosts".to_string()),
            name: "describeAwsServices".to_string(),
            description: "Query AWS Price List service metadata for infrastructure cost analysis."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "serviceCode": { "type": "string" },
                    "maxResults": { "type": "integer", "minimum": 1, "maximum": 100 },
                    "nextToken": { "type": "string" },
                    "apiRegion": { "type": "string" },
                    "profile": { "type": "string" }
                },
                "additionalProperties": false
            }),
            defer_loading: false,
        },
        DynamicToolSpec {
            namespace: Some("infracosts".to_string()),
            name: "getAwsCostAndUsage".to_string(),
            description: "Query AWS Cost Explorer actual cost and usage data.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "timePeriod": { "$ref": "#/$defs/dateInterval" },
                    "granularity": {
                        "type": "string",
                        "enum": ["DAILY", "MONTHLY", "HOURLY"]
                    },
                    "metrics": {
                        "type": "array",
                        "items": { "type": "string" },
                        "minItems": 1
                    },
                    "filter": { "type": "object" },
                    "groupBy": {
                        "type": "array",
                        "items": { "$ref": "#/$defs/groupBy" },
                        "maxItems": 2
                    },
                    "billingViewArn": { "type": "string" },
                    "nextPageToken": { "type": "string" },
                    "apiRegion": { "type": "string" },
                    "profile": { "type": "string" }
                },
                "required": ["timePeriod", "granularity", "metrics"],
                "$defs": {
                    "dateInterval": {
                        "type": "object",
                        "properties": {
                            "start": { "type": "string" },
                            "end": { "type": "string" }
                        },
                        "required": ["start", "end"],
                        "additionalProperties": false
                    },
                    "groupBy": {
                        "type": "object",
                        "properties": {
                            "type": { "type": "string" },
                            "key": { "type": "string" }
                        },
                        "required": ["type", "key"],
                        "additionalProperties": false
                    }
                },
                "additionalProperties": false
            }),
            defer_loading: false,
        },
        DynamicToolSpec {
            namespace: Some("infracosts".to_string()),
            name: "getAwsDimensionValues".to_string(),
            description: "Query AWS Cost Explorer dimension values for billing filters and groupings."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "timePeriod": { "$ref": "#/$defs/dateInterval" },
                    "dimension": { "type": "string" },
                    "context": {
                        "type": "string",
                        "enum": ["COST_AND_USAGE", "RESERVATIONS", "SAVINGS_PLANS"]
                    },
                    "searchString": { "type": "string" },
                    "filter": { "type": "object" },
                    "maxResults": { "type": "integer", "minimum": 1 },
                    "nextPageToken": { "type": "string" },
                    "billingViewArn": { "type": "string" },
                    "apiRegion": { "type": "string" },
                    "profile": { "type": "string" }
                },
                "required": ["timePeriod", "dimension"],
                "$defs": {
                    "dateInterval": {
                        "type": "object",
                        "properties": {
                            "start": { "type": "string" },
                            "end": { "type": "string" }
                        },
                        "required": ["start", "end"],
                        "additionalProperties": false
                    }
                },
                "additionalProperties": false
            }),
            defer_loading: false,
        },
        DynamicToolSpec {
            namespace: Some("infracosts".to_string()),
            name: "getAwsCostForecast".to_string(),
            description: "Query AWS Cost Explorer cost forecasts from actual billing history."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "timePeriod": { "$ref": "#/$defs/dateInterval" },
                    "metric": { "type": "string" },
                    "granularity": {
                        "type": "string",
                        "enum": ["DAILY", "MONTHLY"]
                    },
                    "filter": { "type": "object" },
                    "billingViewArn": { "type": "string" },
                    "predictionIntervalLevel": { "type": "integer", "minimum": 51, "maximum": 99 },
                    "apiRegion": { "type": "string" },
                    "profile": { "type": "string" }
                },
                "required": ["timePeriod", "metric", "granularity"],
                "$defs": {
                    "dateInterval": {
                        "type": "object",
                        "properties": {
                            "start": { "type": "string" },
                            "end": { "type": "string" }
                        },
                        "required": ["start", "end"],
                        "additionalProperties": false
                    }
                },
                "additionalProperties": false
            }),
            defer_loading: false,
        },
        DynamicToolSpec {
            namespace: Some("infracosts".to_string()),
            name: "getAwsAnomalies".to_string(),
            description: "Query AWS Cost Explorer anomaly detection results.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "timePeriod": { "$ref": "#/$defs/dateInterval" },
                    "monitorArn": { "type": "string" },
                    "feedback": {
                        "type": "string",
                        "enum": ["YES", "NO", "PLANNED_ACTIVITY"]
                    },
                    "totalImpact": {
                        "type": "object",
                        "properties": {
                            "numericOperator": { "type": "string" },
                            "startValue": { "type": "number" },
                            "endValue": { "type": "number" }
                        },
                        "required": ["numericOperator", "startValue"],
                        "additionalProperties": false
                    },
                    "maxResults": { "type": "integer", "minimum": 1 },
                    "nextPageToken": { "type": "string" },
                    "apiRegion": { "type": "string" },
                    "profile": { "type": "string" }
                },
                "required": ["timePeriod"],
                "$defs": {
                    "dateInterval": {
                        "type": "object",
                        "properties": {
                            "start": { "type": "string" },
                            "end": { "type": "string" }
                        },
                        "required": ["start", "end"],
                        "additionalProperties": false
                    }
                },
                "additionalProperties": false
            }),
            defer_loading: false,
        },
        DynamicToolSpec {
            namespace: Some("infracosts".to_string()),
            name: "listBillingViews".to_string(),
            description: "List AWS billing views for scoped FP&A and showback access."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "activeAfterInclusive": { "type": "integer" },
                    "activeBeforeInclusive": { "type": "integer" },
                    "arns": { "type": "array", "items": { "type": "string" } },
                    "billingViewTypes": {
                        "type": "array",
                        "items": {
                            "type": "string",
                            "enum": ["PRIMARY", "BILLING_GROUP", "CUSTOM"]
                        }
                    },
                    "ownerAccountId": { "type": "string" },
                    "maxResults": { "type": "integer", "minimum": 1, "maximum": 100 },
                    "nextToken": { "type": "string" },
                    "apiRegion": { "type": "string" },
                    "profile": { "type": "string" }
                },
                "additionalProperties": false
            }),
            defer_loading: false,
        },
        DynamicToolSpec {
            namespace: Some("infracosts".to_string()),
            name: "getAwsProducts".to_string(),
            description: "Query AWS Price List products and rates for infrastructure resources."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "serviceCode": { "type": "string" },
                    "filters": { "type": "array", "items": { "$ref": "#/$defs/filter" } },
                    "maxResults": { "type": "integer", "minimum": 1, "maximum": 100 },
                    "nextToken": { "type": "string" },
                    "apiRegion": { "type": "string" },
                    "profile": { "type": "string" }
                },
                "required": ["serviceCode"],
                "$defs": {
                    "filter": {
                        "type": "object",
                        "properties": {
                            "field": { "type": "string" },
                            "value": { "type": "string" },
                            "matchType": {
                                "type": "string",
                                "enum": ["TERM_MATCH", "EQUALS", "CONTAINS", "ANY_OF", "NONE_OF"]
                            }
                        },
                        "required": ["field", "value"],
                        "additionalProperties": false
                    }
                },
                "additionalProperties": false
            }),
            defer_loading: false,
        },
        DynamicToolSpec {
            namespace: Some("infracosts".to_string()),
            name: "getAwsVmPrice".to_string(),
            description: "Get AWS EC2 VM price context from the AWS Price List Query API."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "instanceType": { "type": "string" },
                    "regionCode": { "type": "string" },
                    "location": { "type": "string" },
                    "operatingSystem": { "type": "string" },
                    "tenancy": { "type": "string" },
                    "preInstalledSw": { "type": "string" },
                    "capacityStatus": { "type": "string" },
                    "licenseModel": { "type": "string" },
                    "maxResults": { "type": "integer", "minimum": 1, "maximum": 100 },
                    "apiRegion": { "type": "string" },
                    "profile": { "type": "string" }
                },
                "required": ["instanceType"],
                "additionalProperties": false
            }),
            defer_loading: false,
        },
        DynamicToolSpec {
            namespace: Some("infracosts".to_string()),
            name: "estimateAwsBoq".to_string(),
            description:
                "Estimate a multi-line AWS BOQ from Price List data using explicit service filters and quantities."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "lineItems": {
                        "type": "array",
                        "items": { "$ref": "#/$defs/lineItem" },
                        "minItems": 1
                    },
                    "apiRegion": { "type": "string" },
                    "profile": { "type": "string" }
                },
                "required": ["lineItems"],
                "$defs": {
                    "filter": {
                        "type": "object",
                        "properties": {
                            "field": { "type": "string" },
                            "value": { "type": "string" },
                            "matchType": {
                                "type": "string",
                                "enum": ["TERM_MATCH", "EQUALS", "CONTAINS", "ANY_OF", "NONE_OF"]
                            }
                        },
                        "required": ["field", "value"],
                        "additionalProperties": false
                    },
                    "lineItem": {
                        "type": "object",
                        "properties": {
                            "label": { "type": "string" },
                            "serviceCode": { "type": "string" },
                            "filters": {
                                "type": "array",
                                "items": { "$ref": "#/$defs/filter" }
                            },
                            "quantity": { "type": "number" },
                            "expectedUnit": { "type": "string" },
                            "selectBy": {
                                "type": "string",
                                "enum": ["lowestPrice", "first"]
                            }
                        },
                        "required": ["serviceCode", "quantity"],
                        "additionalProperties": false
                    }
                },
                "additionalProperties": false
            }),
            defer_loading: false,
        },
    ];
    specs.extend(query::dynamic_tool_specs());
    specs
}

pub(super) async fn handle_dynamic_tool_call(
    tool: &str,
    arguments: JsonValue,
) -> Result<String, String> {
    match tool {
        "describeAwsServices" => describe_aws_services(arguments).await,
        "getAwsCostAndUsage" => get_aws_cost_and_usage(arguments).await,
        "getAwsDimensionValues" => get_aws_dimension_values(arguments).await,
        "getAwsCostForecast" => get_aws_cost_forecast(arguments).await,
        "getAwsAnomalies" => get_aws_anomalies(arguments).await,
        "listBillingViews" => list_billing_views(arguments).await,
        "getAwsProducts" => get_aws_products(arguments).await,
        "getAwsVmPrice" => get_aws_vm_price(arguments).await,
        "estimateAwsBoq" => estimate_aws_boq(arguments).await,
        _ => query::handle_dynamic_tool_call(tool, arguments)
            .await
            .unwrap_or_else(|| {
                Err(format!(
                    "Unsupported ThinWedge cost tool `infracosts.{tool}`"
                ))
            }),
    }
}

async fn describe_aws_services(arguments: JsonValue) -> Result<String, String> {
    let args: DescribeAwsServicesArgs = parse_arguments(arguments)?;
    let response = aws_pricing_request(
        args.profile.as_deref(),
        args.api_region.as_deref(),
        "DescribeServices",
        strip_nulls(json!({
            "ServiceCode": args.service_code,
            "FormatVersion": "aws_v1",
            "MaxResults": args.max_results,
            "NextToken": args.next_token,
        })),
    )
    .await?;
    pretty_json(&json!({ "source": "AWS Price List Query API", "response": response }))
}

async fn get_aws_cost_and_usage(arguments: JsonValue) -> Result<String, String> {
    let args: GetAwsCostAndUsageArgs = parse_arguments(arguments)?;
    let response = aws_cost_explorer_request(
        args.profile.as_deref(),
        args.api_region.as_deref(),
        "GetCostAndUsage",
        strip_nulls(json!({
            "TimePeriod": to_aws_date_interval(&args.time_period),
            "Granularity": args.granularity,
            "Metrics": args.metrics,
            "Filter": args.filter,
            "GroupBy": (!args.group_by.is_empty()).then(|| {
                args.group_by.iter().map(|group| {
                    json!({
                        "Type": group.r#type,
                        "Key": group.key,
                    })
                }).collect::<Vec<_>>()
            }),
            "BillingViewArn": args.billing_view_arn,
            "NextPageToken": args.next_page_token,
        })),
    )
    .await?;
    pretty_json(&json!({
        "source": "AWS Cost Explorer",
        "response": response,
    }))
}

async fn get_aws_dimension_values(arguments: JsonValue) -> Result<String, String> {
    let args: GetAwsDimensionValuesArgs = parse_arguments(arguments)?;
    let response = aws_cost_explorer_request(
        args.profile.as_deref(),
        args.api_region.as_deref(),
        "GetDimensionValues",
        strip_nulls(json!({
            "TimePeriod": to_aws_date_interval(&args.time_period),
            "Dimension": args.dimension,
            "Context": args.context,
            "SearchString": args.search_string,
            "Filter": args.filter,
            "MaxResults": args.max_results,
            "NextPageToken": args.next_page_token,
            "BillingViewArn": args.billing_view_arn,
        })),
    )
    .await?;
    pretty_json(&json!({
        "source": "AWS Cost Explorer",
        "response": response,
    }))
}

async fn get_aws_cost_forecast(arguments: JsonValue) -> Result<String, String> {
    let args: GetAwsCostForecastArgs = parse_arguments(arguments)?;
    let response = aws_cost_explorer_request(
        args.profile.as_deref(),
        args.api_region.as_deref(),
        "GetCostForecast",
        strip_nulls(json!({
            "TimePeriod": to_aws_date_interval(&args.time_period),
            "Metric": args.metric,
            "Granularity": args.granularity,
            "Filter": args.filter,
            "BillingViewArn": args.billing_view_arn,
            "PredictionIntervalLevel": args.prediction_interval_level,
        })),
    )
    .await?;
    pretty_json(&json!({
        "source": "AWS Cost Explorer",
        "response": response,
    }))
}

async fn get_aws_anomalies(arguments: JsonValue) -> Result<String, String> {
    let args: GetAwsAnomaliesArgs = parse_arguments(arguments)?;
    let response = aws_cost_explorer_request(
        args.profile.as_deref(),
        args.api_region.as_deref(),
        "GetAnomalies",
        strip_nulls(json!({
            "DateInterval": to_aws_anomaly_date_interval(&args.time_period),
            "MonitorArn": args.monitor_arn,
            "Feedback": args.feedback,
            "TotalImpact": args.total_impact.as_ref().map(to_aws_total_impact_filter),
            "MaxResults": args.max_results,
            "NextPageToken": args.next_page_token,
        })),
    )
    .await?;
    pretty_json(&json!({
        "source": "AWS Cost Explorer",
        "response": response,
    }))
}

async fn list_billing_views(arguments: JsonValue) -> Result<String, String> {
    let args: ListBillingViewsArgs = parse_arguments(arguments)?;
    let active_time_range = match (args.active_after_inclusive, args.active_before_inclusive) {
        (Some(start), Some(end)) => Some(json!({
            "activeTimeRange": {
                "activeAfterInclusive": start,
                "activeBeforeInclusive": end,
            }
        })),
        _ => None,
    };
    let response = aws_billing_request(
        args.profile.as_deref(),
        args.api_region.as_deref(),
        "ListBillingViews",
        strip_nulls(json!({
            "activeTimeRange": active_time_range.and_then(|value| value.get("activeTimeRange").cloned()),
            "arns": (!args.arns.is_empty()).then_some(args.arns),
            "billingViewTypes": (!args.billing_view_types.is_empty()).then_some(args.billing_view_types),
            "ownerAccountId": args.owner_account_id,
            "maxResults": args.max_results,
            "nextToken": args.next_token,
        })),
    )
    .await?;
    pretty_json(&json!({
        "source": "AWS Billing",
        "response": response,
    }))
}

async fn get_aws_products(arguments: JsonValue) -> Result<String, String> {
    let args: GetAwsProductsArgs = parse_arguments(arguments)?;
    let response = aws_get_products(
        args.profile.as_deref(),
        args.api_region.as_deref(),
        &args.service_code,
        &args.filters,
        args.max_results,
        args.next_token.as_deref(),
    )
    .await?;
    pretty_json(&json!({
        "source": "AWS Price List Query API",
        "serviceCode": args.service_code,
        "filters": args.filters,
        "response": response,
        "products": parse_aws_pricelist_entries(&response)?,
    }))
}

async fn get_aws_vm_price(arguments: JsonValue) -> Result<String, String> {
    let args: GetAwsVmPriceArgs = parse_arguments(arguments)?;
    let mut filters = vec![AwsPricingFilter {
        field: "instanceType".to_string(),
        value: args.instance_type.clone(),
        match_type: default_aws_match_type(),
    }];
    if let Some(region_code) = args.region_code.as_ref() {
        filters.push(AwsPricingFilter {
            field: "regionCode".to_string(),
            value: region_code.clone(),
            match_type: default_aws_match_type(),
        });
    }
    if let Some(location) = args.location.as_ref() {
        filters.push(AwsPricingFilter {
            field: "location".to_string(),
            value: location.clone(),
            match_type: default_aws_match_type(),
        });
    }
    filters.extend([
        AwsPricingFilter {
            field: "operatingSystem".to_string(),
            value: args.operating_system.unwrap_or_else(|| "Linux".to_string()),
            match_type: default_aws_match_type(),
        },
        AwsPricingFilter {
            field: "tenancy".to_string(),
            value: args.tenancy.unwrap_or_else(|| "Shared".to_string()),
            match_type: default_aws_match_type(),
        },
        AwsPricingFilter {
            field: "preInstalledSw".to_string(),
            value: args.pre_installed_sw.unwrap_or_else(|| "NA".to_string()),
            match_type: default_aws_match_type(),
        },
        AwsPricingFilter {
            field: "capacitystatus".to_string(),
            value: args.capacity_status.unwrap_or_else(|| "Used".to_string()),
            match_type: default_aws_match_type(),
        },
        AwsPricingFilter {
            field: "licenseModel".to_string(),
            value: args
                .license_model
                .unwrap_or_else(|| "No License required".to_string()),
            match_type: default_aws_match_type(),
        },
    ]);

    let response = aws_get_products(
        args.profile.as_deref(),
        args.api_region.as_deref(),
        "AmazonEC2",
        &filters,
        args.max_results,
        None,
    )
    .await?;
    let products = parse_aws_pricelist_entries(&response)?;
    let matches = products
        .iter()
        .map(|product| {
            json!({
                "sku": product.pointer("/product/sku"),
                "productFamily": product.pointer("/product/productFamily"),
                "attributes": product.pointer("/product/attributes").cloned().unwrap_or(JsonValue::Null),
                "onDemandPriceDimensions": extract_on_demand_price_dimensions(product),
                "lowestHourlyUsd": lowest_hourly_price_usd(product),
            })
        })
        .collect::<Vec<_>>();

    pretty_json(&json!({
        "source": "AWS Price List Query API",
        "serviceCode": "AmazonEC2",
        "filters": filters,
        "matches": matches,
        "nextToken": response.get("NextToken").cloned().unwrap_or(JsonValue::Null),
    }))
}

async fn estimate_aws_boq(arguments: JsonValue) -> Result<String, String> {
    let args: EstimateAwsBoqArgs = parse_arguments(arguments)?;
    let mut lines = Vec::new();
    let mut total_estimated_cost_usd = 0.0;
    for line_item in args.line_items {
        let response = aws_get_products(
            args.profile.as_deref(),
            args.api_region.as_deref(),
            &line_item.service_code,
            &line_item.filters,
            Some(100),
            None,
        )
        .await?;
        let products = parse_aws_pricelist_entries(&response)?;
        let selected = select_price_dimension(
            &products,
            line_item.expected_unit.as_deref(),
            line_item
                .select_by
                .unwrap_or(AwsLineItemSelectBy::LowestPrice),
        );
        let estimated_cost_usd = selected
            .price_per_unit_usd
            .map(|price| price * line_item.quantity);
        if let Some(cost) = estimated_cost_usd {
            total_estimated_cost_usd += cost;
        }
        lines.push(AwsBoqEstimateLine {
            label: line_item
                .label
                .unwrap_or_else(|| line_item.service_code.clone()),
            service_code: line_item.service_code,
            quantity: line_item.quantity,
            selected_unit: selected.unit,
            selected_price_per_unit_usd: selected.price_per_unit_usd,
            estimated_cost_usd,
            matched_sku: selected.sku,
            matched_description: selected.description,
        });
    }
    pretty_json(&json!({
        "source": "AWS Price List Query API",
        "lines": lines,
        "totalEstimatedCostUsd": total_estimated_cost_usd,
    }))
}

fn parse_arguments<T>(arguments: JsonValue) -> Result<T, String>
where
    T: for<'de> serde::Deserialize<'de>,
{
    serde_json::from_value(arguments)
        .map_err(|err| format!("invalid ThinWedge infrastructure cost tool arguments: {err}"))
}

fn pretty_json<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string_pretty(value)
        .map_err(|err| format!("failed to encode ThinWedge infrastructure cost response: {err}"))
}

fn to_aws_date_interval(time_period: &AwsDateInterval) -> JsonValue {
    json!({
        "Start": time_period.start,
        "End": time_period.end,
    })
}

fn to_aws_anomaly_date_interval(time_period: &AwsDateInterval) -> JsonValue {
    json!({
        "StartDate": time_period.start,
        "EndDate": time_period.end,
    })
}

fn to_aws_total_impact_filter(filter: &AwsTotalImpactFilter) -> JsonValue {
    json!({
        "NumericOperator": filter.numeric_operator,
        "StartValue": filter.start_value,
        "EndValue": filter.end_value,
    })
}

#[cfg(test)]
#[path = "cost_context_infra_tests.rs"]
mod tests;
