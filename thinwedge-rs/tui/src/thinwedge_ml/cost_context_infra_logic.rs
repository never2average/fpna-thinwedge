use super::AwsLineItemSelectBy;
use super::AwsPricingFilter;
use super::types::SelectedPriceDimension;
use bytes::Bytes;
use thinwedge_aws_auth::AwsAuthConfig;
use thinwedge_aws_auth::AwsAuthContext;
use thinwedge_aws_auth::AwsRequestToSign;
use http::HeaderMap;
use http::HeaderValue;
use http::Method;
use serde_json::Value as JsonValue;
use serde_json::json;

const AWS_DEFAULT_REGION: &str = "us-east-1";
const AWS_PRICING_ENDPOINT_PREFIX: &str = "api.pricing";
const AWS_PRICING_TARGET_PREFIX: &str = "AWSPriceListService";
const AWS_CE_ENDPOINT_PREFIX: &str = "ce";
const AWS_CE_TARGET_PREFIX: &str = "AWSInsightsIndexService";
const AWS_BILLING_ENDPOINT_PREFIX: &str = "billing";
const AWS_BILLING_TARGET_PREFIX: &str = "AWSBilling";

struct AwsJsonRpcService<'a> {
    service: &'a str,
    endpoint_prefix: &'a str,
    target_prefix: &'a str,
    json_version: &'a str,
}

pub(super) async fn aws_get_products(
    profile: Option<&str>,
    api_region: Option<&str>,
    service_code: &str,
    filters: &[AwsPricingFilter],
    max_results: Option<u32>,
    next_token: Option<&str>,
) -> Result<JsonValue, String> {
    aws_pricing_request(
        profile,
        api_region,
        "GetProducts",
        strip_nulls(json!({
            "ServiceCode": service_code,
            "FormatVersion": "aws_v1",
            "Filters": filters.iter().map(|filter| {
                json!({
                    "Field": filter.field,
                    "Value": filter.value,
                    "Type": filter.match_type,
                })
            }).collect::<Vec<_>>(),
            "MaxResults": max_results,
            "NextToken": next_token,
        })),
    )
    .await
}

pub(super) async fn aws_pricing_request(
    profile: Option<&str>,
    api_region: Option<&str>,
    operation: &str,
    payload: JsonValue,
) -> Result<JsonValue, String> {
    aws_json_rpc_request(
        profile,
        api_region,
        AwsJsonRpcService {
            service: "pricing",
            endpoint_prefix: AWS_PRICING_ENDPOINT_PREFIX,
            target_prefix: AWS_PRICING_TARGET_PREFIX,
            json_version: "1.1",
        },
        operation,
        payload,
    )
    .await
}

pub(super) async fn aws_cost_explorer_request(
    profile: Option<&str>,
    api_region: Option<&str>,
    operation: &str,
    payload: JsonValue,
) -> Result<JsonValue, String> {
    aws_json_rpc_request(
        profile,
        api_region,
        AwsJsonRpcService {
            service: "ce",
            endpoint_prefix: AWS_CE_ENDPOINT_PREFIX,
            target_prefix: AWS_CE_TARGET_PREFIX,
            json_version: "1.1",
        },
        operation,
        payload,
    )
    .await
}

pub(super) async fn aws_billing_request(
    profile: Option<&str>,
    api_region: Option<&str>,
    operation: &str,
    payload: JsonValue,
) -> Result<JsonValue, String> {
    aws_json_rpc_request(
        profile,
        api_region,
        AwsJsonRpcService {
            service: "billing",
            endpoint_prefix: AWS_BILLING_ENDPOINT_PREFIX,
            target_prefix: AWS_BILLING_TARGET_PREFIX,
            json_version: "1.0",
        },
        operation,
        payload,
    )
    .await
}

async fn aws_json_rpc_request(
    profile: Option<&str>,
    api_region: Option<&str>,
    service: AwsJsonRpcService<'_>,
    operation: &str,
    payload: JsonValue,
) -> Result<JsonValue, String> {
    let region = api_region.unwrap_or(AWS_DEFAULT_REGION);
    let url = aws_json_rpc_endpoint_url(service.endpoint_prefix, region);
    let body = serde_json::to_vec(&payload)
        .map_err(|err| format!("failed to encode AWS {} request: {err}", service.service))?;
    let mut headers = HeaderMap::new();
    headers.insert(
        http::header::CONTENT_TYPE,
        HeaderValue::from_str(&format!("application/x-amz-json-{}", service.json_version))
            .map_err(|err| format!("invalid AWS content type header: {err}"))?,
    );
    let x_amz_target = format!("{}.{}", service.target_prefix, operation);
    headers.insert(
        "x-amz-target",
        HeaderValue::from_str(&x_amz_target)
            .map_err(|err| format!("invalid AWS target header `{x_amz_target}`: {err}"))?,
    );
    let signed = AwsAuthContext::load(AwsAuthConfig {
        profile: profile.map(str::to_string),
        region: Some(region.to_string()),
        service: service.service.to_string(),
    })
    .await
    .map_err(|err| format!("failed to load AWS auth context: {err}"))?
    .sign(AwsRequestToSign {
        method: Method::POST,
        url,
        headers,
        body: Bytes::from(body.clone()),
    })
    .await
    .map_err(|err| format!("failed to sign AWS {} request: {err}", service.service))?;

    let mut request = reqwest::Client::new().post(signed.url);
    for (name, value) in &signed.headers {
        let value = value
            .to_str()
            .map_err(|err| format!("AWS signed header is not valid UTF-8: {err}"))?;
        request = request.header(name.as_str(), value);
    }
    let response = request
        .body(body)
        .send()
        .await
        .map_err(|err| format!("failed to call AWS {} API: {err}", service.service))?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<body unavailable>".to_string());
        return Err(format!(
            "AWS {} API request failed with status {status}: {body}",
            service.service
        ));
    }
    response.json::<JsonValue>().await.map_err(|err| {
        format!(
            "failed to decode AWS {} API response: {err}",
            service.service
        )
    })
}

pub(super) fn aws_json_rpc_endpoint_url(endpoint_prefix: &str, region: &str) -> String {
    format!("https://{endpoint_prefix}.{region}.amazonaws.com/")
}

pub(super) fn parse_aws_pricelist_entries(response: &JsonValue) -> Result<Vec<JsonValue>, String> {
    response
        .get("PriceList")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| "AWS pricing API response did not contain a `PriceList` array".to_string())?
        .iter()
        .map(|entry| {
            let entry = entry
                .as_str()
                .ok_or_else(|| "AWS pricing `PriceList` entry was not a JSON string".to_string())?;
            serde_json::from_str::<JsonValue>(entry)
                .map_err(|err| format!("failed to decode AWS pricing product entry: {err}"))
        })
        .collect()
}

pub(super) fn extract_on_demand_price_dimensions(product: &JsonValue) -> Vec<JsonValue> {
    let Some(offers) = product
        .pointer("/terms/OnDemand")
        .and_then(JsonValue::as_object)
    else {
        return Vec::new();
    };
    let mut dimensions = Vec::new();
    for (offer_code, offer) in offers {
        let Some(price_dimensions) = offer.get("priceDimensions").and_then(JsonValue::as_object)
        else {
            continue;
        };
        for (rate_code, dimension) in price_dimensions {
            dimensions.push(json!({
                "offerCode": offer_code,
                "rateCode": rate_code,
                "description": dimension.get("description").cloned().unwrap_or(JsonValue::Null),
                "unit": dimension.get("unit").cloned().unwrap_or(JsonValue::Null),
                "pricePerUnitUsd": dimension.pointer("/pricePerUnit/USD").cloned().unwrap_or(JsonValue::Null),
                "beginRange": dimension.get("beginRange").cloned().unwrap_or(JsonValue::Null),
                "endRange": dimension.get("endRange").cloned().unwrap_or(JsonValue::Null),
            }));
        }
    }
    dimensions
}

pub(super) fn lowest_hourly_price_usd(product: &JsonValue) -> Option<f64> {
    extract_on_demand_price_dimensions(product)
        .into_iter()
        .filter(|dimension| dimension.get("unit").and_then(JsonValue::as_str) == Some("Hrs"))
        .filter_map(|dimension| {
            dimension
                .get("pricePerUnitUsd")
                .and_then(JsonValue::as_str)
                .and_then(|price| price.parse::<f64>().ok())
        })
        .reduce(f64::min)
}

pub(super) fn select_price_dimension(
    products: &[JsonValue],
    expected_unit: Option<&str>,
    select_by: AwsLineItemSelectBy,
) -> SelectedPriceDimension {
    let mut dimensions = products
        .iter()
        .flat_map(|product| {
            let sku = product
                .pointer("/product/sku")
                .and_then(JsonValue::as_str)
                .map(str::to_string);
            extract_on_demand_price_dimensions(product)
                .into_iter()
                .filter_map(move |dimension| {
                    let unit = dimension.get("unit").and_then(JsonValue::as_str)?;
                    if expected_unit.is_some_and(|expected| expected != unit) {
                        return None;
                    }
                    let price = dimension
                        .get("pricePerUnitUsd")
                        .and_then(JsonValue::as_str)
                        .and_then(|price| price.parse::<f64>().ok())?;
                    Some(SelectedPriceDimension {
                        sku: sku.clone(),
                        unit: Some(unit.to_string()),
                        description: dimension
                            .get("description")
                            .and_then(JsonValue::as_str)
                            .map(str::to_string),
                        price_per_unit_usd: Some(price),
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    match select_by {
        AwsLineItemSelectBy::First => dimensions.into_iter().next().unwrap_or_default(),
        AwsLineItemSelectBy::LowestPrice => {
            dimensions.sort_by(|left, right| {
                left.price_per_unit_usd
                    .partial_cmp(&right.price_per_unit_usd)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            dimensions.into_iter().next().unwrap_or_default()
        }
    }
}

pub(super) fn strip_nulls(value: JsonValue) -> JsonValue {
    match value {
        JsonValue::Object(map) => JsonValue::Object(
            map.into_iter()
                .filter_map(|(key, value)| {
                    let value = strip_nulls(value);
                    (!value.is_null()).then_some((key, value))
                })
                .collect(),
        ),
        JsonValue::Array(values) => JsonValue::Array(values.into_iter().map(strip_nulls).collect()),
        other => other,
    }
}
