use super::logic::aws_cost_explorer_request;
use super::logic::aws_get_products;
use super::logic::aws_pricing_request;
use super::logic::parse_aws_pricelist_entries;
use super::logic::strip_nulls;
use super::to_aws_date_interval;
use super::types::QueryAwsByAccountArgs;
use super::types::QueryAwsByServiceArgs;
use super::types::SearchAwsPriceListAttributeNamesArgs;
use super::types::SearchAwsPriceListAttributeValuesArgs;
use super::types::SearchAwsServicesArgs;
use thinwedge_app_server_protocol::DynamicToolSpec;
use serde::Serialize;
use serde_json::Value as JsonValue;
use serde_json::json;
use std::collections::BTreeSet;

pub(super) fn dynamic_tool_specs() -> Vec<DynamicToolSpec> {
    vec![
        DynamicToolSpec {
            namespace: Some("infracosts".to_string()),
            name: "searchAwsServices".to_string(),
            description: "Search AWS Price List service codes for infrastructure pricing workflows."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "search": { "type": "string" },
                    "maxResults": { "type": "integer", "minimum": 1 },
                    "apiRegion": { "type": "string" },
                    "profile": { "type": "string" }
                },
                "additionalProperties": false
            }),
            defer_loading: false,
        },
        DynamicToolSpec {
            namespace: Some("infracosts".to_string()),
            name: "searchAwsPriceListAttributeNames".to_string(),
            description:
                "Search attribute names for an AWS Price List service before building product filters."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "serviceCode": { "type": "string" },
                    "search": { "type": "string" },
                    "limit": { "type": "integer", "minimum": 1 },
                    "apiRegion": { "type": "string" },
                    "profile": { "type": "string" }
                },
                "required": ["serviceCode"],
                "additionalProperties": false
            }),
            defer_loading: false,
        },
        DynamicToolSpec {
            namespace: Some("infracosts".to_string()),
            name: "searchAwsPriceListAttributeValues".to_string(),
            description:
                "Sample AWS Price List products to discover candidate values for one attribute field."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "serviceCode": { "type": "string" },
                    "field": { "type": "string" },
                    "search": { "type": "string" },
                    "filters": { "type": "array", "items": { "$ref": "#/$defs/filter" } },
                    "samplePages": { "type": "integer", "minimum": 1, "maximum": 20 },
                    "valuesLimit": { "type": "integer", "minimum": 1 },
                    "apiRegion": { "type": "string" },
                    "profile": { "type": "string" }
                },
                "required": ["serviceCode", "field"],
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
            name: "queryAwsByService".to_string(),
            description: "Shortcut Cost Explorer query filtered to one AWS service."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "timePeriod": { "$ref": "#/$defs/dateInterval" },
                    "service": { "type": "string" },
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
                "required": ["timePeriod", "service", "granularity"],
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
            name: "queryAwsByAccount".to_string(),
            description: "Shortcut Cost Explorer query filtered to one or more linked accounts."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "timePeriod": { "$ref": "#/$defs/dateInterval" },
                    "linkedAccounts": {
                        "type": "array",
                        "items": { "type": "string" },
                        "minItems": 1
                    },
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
                "required": ["timePeriod", "linkedAccounts", "granularity"],
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
    ]
}

pub(super) async fn handle_dynamic_tool_call(
    tool: &str,
    arguments: JsonValue,
) -> Option<Result<String, String>> {
    match tool {
        "searchAwsServices" => Some(search_aws_services(arguments).await),
        "searchAwsPriceListAttributeNames" => {
            Some(search_aws_price_list_attribute_names(arguments).await)
        }
        "searchAwsPriceListAttributeValues" => {
            Some(search_aws_price_list_attribute_values(arguments).await)
        }
        "queryAwsByService" => Some(query_aws_by_service(arguments).await),
        "queryAwsByAccount" => Some(query_aws_by_account(arguments).await),
        _ => None,
    }
}

async fn search_aws_services(arguments: JsonValue) -> Result<String, String> {
    let args: SearchAwsServicesArgs = parse_arguments(arguments)?;
    let mut next_token = None;
    let query = args.search.as_deref().map(str::to_ascii_lowercase);
    let max_results = args.max_results.unwrap_or(50);
    let mut matches = Vec::new();

    while matches.len() < max_results {
        let response = aws_pricing_request(
            args.profile.as_deref(),
            args.api_region.as_deref(),
            "DescribeServices",
            strip_nulls(json!({
                "FormatVersion": "aws_v1",
                "MaxResults": 100,
                "NextToken": next_token,
            })),
        )
        .await?;
        let services = response
            .get("Services")
            .and_then(JsonValue::as_array)
            .ok_or_else(|| "AWS pricing service response did not contain `Services`".to_string())?;
        for service in services {
            let Some(service_code) = service.get("ServiceCode").and_then(JsonValue::as_str) else {
                continue;
            };
            if query
                .as_ref()
                .is_none_or(|query| service_code.to_ascii_lowercase().contains(query))
            {
                matches.push(service.clone());
                if matches.len() >= max_results {
                    break;
                }
            }
        }
        next_token = response
            .get("NextToken")
            .and_then(JsonValue::as_str)
            .map(str::to_string);
        if next_token.is_none() {
            break;
        }
    }

    pretty_json(&json!({
        "source": "AWS Price List Query API",
        "matches": matches,
        "returnedCount": matches.len(),
        "truncated": next_token.is_some() && matches.len() >= max_results,
    }))
}

async fn search_aws_price_list_attribute_names(arguments: JsonValue) -> Result<String, String> {
    let args: SearchAwsPriceListAttributeNamesArgs = parse_arguments(arguments)?;
    let response = aws_pricing_request(
        args.profile.as_deref(),
        args.api_region.as_deref(),
        "DescribeServices",
        strip_nulls(json!({
            "ServiceCode": args.service_code,
            "FormatVersion": "aws_v1",
            "MaxResults": 1,
        })),
    )
    .await?;
    let attributes = response
        .pointer("/Services/0/AttributeNames")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| "AWS pricing service response did not contain `AttributeNames`".to_string())?
        .iter()
        .filter_map(JsonValue::as_str)
        .filter(|name| {
            args.search.as_deref().is_none_or(|query| {
                name.to_ascii_lowercase()
                    .contains(&query.to_ascii_lowercase())
            })
        })
        .take(args.limit.unwrap_or(200))
        .map(str::to_string)
        .collect::<Vec<_>>();
    pretty_json(&json!({
        "source": "AWS Price List Query API",
        "serviceCode": response.pointer("/Services/0/ServiceCode").cloned().unwrap_or(JsonValue::Null),
        "attributeNames": attributes,
    }))
}

async fn search_aws_price_list_attribute_values(arguments: JsonValue) -> Result<String, String> {
    let args: SearchAwsPriceListAttributeValuesArgs = parse_arguments(arguments)?;
    let mut next_token = None;
    let sample_pages = args.sample_pages.unwrap_or(3).min(20);
    let values_limit = args.values_limit.unwrap_or(100);
    let search = args.search.as_deref().map(str::to_ascii_lowercase);
    let mut values = BTreeSet::new();
    let mut sampled_products = 0usize;

    for _ in 0..sample_pages {
        let response = aws_get_products(
            args.profile.as_deref(),
            args.api_region.as_deref(),
            &args.service_code,
            &args.filters,
            Some(100),
            next_token.as_deref(),
        )
        .await?;
        let products = parse_aws_pricelist_entries(&response)?;
        sampled_products += products.len();
        for product in products {
            let Some(attributes) = product
                .pointer("/product/attributes")
                .and_then(JsonValue::as_object)
            else {
                continue;
            };
            let Some(value) = attributes.get(&args.field).and_then(JsonValue::as_str) else {
                continue;
            };
            if search
                .as_ref()
                .is_none_or(|query| value.to_ascii_lowercase().contains(query))
            {
                values.insert(value.to_string());
                if values.len() >= values_limit {
                    break;
                }
            }
        }
        if values.len() >= values_limit {
            break;
        }
        next_token = response
            .get("NextToken")
            .and_then(JsonValue::as_str)
            .map(str::to_string);
        if next_token.is_none() {
            break;
        }
    }

    pretty_json(&json!({
        "source": "AWS Price List Query API",
        "serviceCode": args.service_code,
        "field": args.field,
        "filters": args.filters,
        "sampledProducts": sampled_products,
        "values": values.into_iter().collect::<Vec<_>>(),
        "truncated": next_token.is_some(),
    }))
}

async fn query_aws_by_service(arguments: JsonValue) -> Result<String, String> {
    let args: QueryAwsByServiceArgs = parse_arguments(arguments)?;
    let base_filter = json!({
        "Dimensions": {
            "Key": "SERVICE",
            "Values": [args.service.clone()],
        }
    });
    let response = aws_cost_explorer_request(
        args.profile.as_deref(),
        args.api_region.as_deref(),
        "GetCostAndUsage",
        strip_nulls(json!({
            "TimePeriod": to_aws_date_interval(&args.time_period),
            "Granularity": args.granularity,
            "Metrics": args.metrics,
            "Filter": combine_expression(base_filter, args.filter),
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
        "service": args.service,
        "response": response,
    }))
}

async fn query_aws_by_account(arguments: JsonValue) -> Result<String, String> {
    let args: QueryAwsByAccountArgs = parse_arguments(arguments)?;
    let base_filter = json!({
        "Dimensions": {
            "Key": "LINKED_ACCOUNT",
            "Values": args.linked_accounts.clone(),
        }
    });
    let response = aws_cost_explorer_request(
        args.profile.as_deref(),
        args.api_region.as_deref(),
        "GetCostAndUsage",
        strip_nulls(json!({
            "TimePeriod": to_aws_date_interval(&args.time_period),
            "Granularity": args.granularity,
            "Metrics": args.metrics,
            "Filter": combine_expression(base_filter, args.filter),
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
        "linkedAccounts": args.linked_accounts,
        "response": response,
    }))
}

fn combine_expression(base_filter: JsonValue, extra_filter: Option<JsonValue>) -> JsonValue {
    match extra_filter {
        Some(extra_filter) => json!({
            "And": [base_filter, extra_filter]
        }),
        None => base_filter,
    }
}

fn parse_arguments<T>(arguments: JsonValue) -> Result<T, String>
where
    T: for<'de> serde::Deserialize<'de>,
{
    serde_json::from_value(arguments)
        .map_err(|err| format!("invalid ThinWedge infrastructure query arguments: {err}"))
}

fn pretty_json<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string_pretty(value)
        .map_err(|err| format!("failed to encode ThinWedge infrastructure query response: {err}"))
}
