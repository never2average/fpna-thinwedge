use super::HistoryCell;
use super::new_dynamic_tool_call;
use crate::render::line_utils::line_to_static;
use serde_json::json;
use thinwedge_app_server_protocol::DynamicToolCallOutputContentItem;

fn render(cell: impl HistoryCell) -> String {
    cell.display_lines(80)
        .into_iter()
        .map(|line| {
            let line = line_to_static(&line);
            line.spans
                .iter()
                .map(|span| span.content.to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn llm_model_snapshot() {
    let payload = json!({
        "source": "Artificial Analysis",
        "model": {
            "id": "model-1",
            "name": "ThinWedge LLM",
            "slug": "thinwedge-llm",
            "model_creator": { "id": "creator-1", "name": "OpenRouter" }
        },
        "costContext": {
            "blendedPricePer1MTokensUsd": 2.75,
            "medianOutputTokensPerSecond": 148.2,
            "medianTimeToFirstTokenSeconds": 0.41,
            "intelligenceIndex": 61.2,
            "codingIndex": 58.4
        }
    });
    let cell = new_dynamic_tool_call(
        Some("llmcosts".to_string()),
        "getModel".to_string(),
        Some(vec![DynamicToolCallOutputContentItem::InputText {
            text: serde_json::to_string_pretty(&payload).expect("payload"),
        }]),
        Some(true),
        Some(182),
    );
    insta::assert_snapshot!(render(cell));
}

#[test]
fn aws_boq_snapshot() {
    let payload = json!({
        "source": "AWS Price List Query API",
        "lines": [
            {
                "label": "App EC2",
                "quantity": 730.0,
                "selectedUnit": "Hrs",
                "selectedPricePerUnitUsd": 0.192,
                "estimatedCostUsd": 140.16
            },
            {
                "label": "gp3 storage",
                "quantity": 500.0,
                "selectedUnit": "GB-Mo",
                "selectedPricePerUnitUsd": 0.08,
                "estimatedCostUsd": 40.0
            }
        ],
        "totalEstimatedCostUsd": 180.16
    });
    let cell = new_dynamic_tool_call(
        Some("infracosts".to_string()),
        "estimateAwsBoq".to_string(),
        Some(vec![DynamicToolCallOutputContentItem::InputText {
            text: serde_json::to_string_pretty(&payload).expect("payload"),
        }]),
        Some(true),
        Some(244),
    );
    insta::assert_snapshot!(render(cell));
}

#[test]
fn aws_cost_query_snapshot() {
    let payload = json!({
        "source": "AWS Cost Explorer",
        "service": "AmazonEC2",
        "response": {
            "ResultsByTime": [
                {
                    "TimePeriod": { "Start": "2026-04-01", "End": "2026-04-02" },
                    "Total": {
                        "UnblendedCost": { "Amount": "12.34", "Unit": "USD" }
                    },
                    "Groups": [
                        {
                            "Keys": ["Amazon Elastic Compute Cloud - Compute"],
                            "Metrics": {
                                "UnblendedCost": { "Amount": "10.20", "Unit": "USD" }
                            }
                        }
                    ]
                },
                {
                    "TimePeriod": { "Start": "2026-04-02", "End": "2026-04-03" },
                    "Total": {
                        "UnblendedCost": { "Amount": "14.01", "Unit": "USD" }
                    },
                    "Groups": [
                        {
                            "Keys": ["EC2-Instances"],
                            "Metrics": {
                                "UnblendedCost": { "Amount": "11.22", "Unit": "USD" }
                            }
                        }
                    ]
                }
            ]
        }
    });
    let cell = new_dynamic_tool_call(
        Some("infracosts".to_string()),
        "queryAwsByService".to_string(),
        Some(vec![DynamicToolCallOutputContentItem::InputText {
            text: serde_json::to_string_pretty(&payload).expect("payload"),
        }]),
        Some(true),
        Some(311),
    );
    insta::assert_snapshot!(render(cell));
}

#[test]
fn environment_action_snapshot() {
    let payload = json!({
        "environment": {
            "id": "env-pricing",
            "status": "running"
        },
        "execution": {
            "summaryJson": {
                "status": "running",
                "workspacePath": "/workspace/thinwedge/env-pricing",
                "httpEndpoint": "https://pod-123-8000.proxy.runpod.net"
            }
        }
    });
    let cell = new_dynamic_tool_call(
        Some("trainingenvironments".to_string()),
        "launch".to_string(),
        Some(vec![DynamicToolCallOutputContentItem::InputText {
            text: serde_json::to_string_pretty(&payload).expect("payload"),
        }]),
        Some(true),
        Some(509),
    );
    insta::assert_snapshot!(render(cell));
}

#[test]
fn model_job_snapshot() {
    let payload = json!({
        "job": {
            "id": "job-1",
            "modelId": "pricing-model",
            "type": "training",
            "status": "completed",
            "environmentId": "env-pricing"
        },
        "execution": {
            "exitCode": 0,
            "summaryJson": {
                "artifactManifestPath": "/workspace/manifests/train.json",
                "generatedFiles": [{ "path": "generated/model.py" }]
            }
        }
    });
    let cell = new_dynamic_tool_call(
        Some("statisticalmodels".to_string()),
        "submitJob".to_string(),
        Some(vec![DynamicToolCallOutputContentItem::InputText {
            text: serde_json::to_string_pretty(&payload).expect("payload"),
        }]),
        Some(true),
        Some(1431),
    );
    insta::assert_snapshot!(render(cell));
}
