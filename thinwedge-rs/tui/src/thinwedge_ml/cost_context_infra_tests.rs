use super::AwsLineItemSelectBy;
use super::aws_json_rpc_endpoint_url;
use super::extract_on_demand_price_dimensions;
use super::lowest_hourly_price_usd;
use super::select_price_dimension;
use pretty_assertions::assert_eq;
use serde_json::json;

#[test]
fn extracts_on_demand_dimensions_from_aws_product() {
    let product = json!({
        "terms": {
            "OnDemand": {
                "offer-1": {
                    "priceDimensions": {
                        "rate-1": {
                            "description": "Linux usage",
                            "unit": "Hrs",
                            "pricePerUnit": { "USD": "0.11" },
                            "beginRange": "0",
                            "endRange": "Inf"
                        }
                    }
                }
            }
        }
    });

    assert_eq!(
        extract_on_demand_price_dimensions(&product),
        vec![json!({
            "offerCode": "offer-1",
            "rateCode": "rate-1",
            "description": "Linux usage",
            "unit": "Hrs",
            "pricePerUnitUsd": "0.11",
            "beginRange": "0",
            "endRange": "Inf"
        })]
    );
    assert_eq!(lowest_hourly_price_usd(&product), Some(0.11));
}

#[test]
fn select_price_dimension_prefers_lowest_matching_unit() {
    let products = vec![
        json!({
            "product": { "sku": "sku-1" },
            "terms": { "OnDemand": { "offer-1": { "priceDimensions": {
                "rate-1": {
                    "description": "Hours",
                    "unit": "Hrs",
                    "pricePerUnit": { "USD": "0.50" }
                }
            }}}}
        }),
        json!({
            "product": { "sku": "sku-2" },
            "terms": { "OnDemand": { "offer-2": { "priceDimensions": {
                "rate-2": {
                    "description": "Hours cheaper",
                    "unit": "Hrs",
                    "pricePerUnit": { "USD": "0.20" }
                }
            }}}}
        }),
    ];

    let selected = select_price_dimension(&products, Some("Hrs"), AwsLineItemSelectBy::LowestPrice);

    assert_eq!(selected.sku, Some("sku-2".to_string()));
    assert_eq!(selected.unit, Some("Hrs".to_string()));
    assert_eq!(selected.price_per_unit_usd, Some(0.20));
}

#[test]
fn aws_json_rpc_endpoint_uses_endpoint_prefix_and_region() {
    assert_eq!(
        aws_json_rpc_endpoint_url("ce", "us-east-1"),
        "https://ce.us-east-1.amazonaws.com/"
    );
    assert_eq!(
        aws_json_rpc_endpoint_url("billing", "us-east-1"),
        "https://billing.us-east-1.amazonaws.com/"
    );
}
