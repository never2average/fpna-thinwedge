use super::HistoryCell;
use super::format_and_truncate_tool_result;
use codex_app_server_protocol::DynamicToolCallOutputContentItem;
#[path = "dynamic_tool_cell_layout.rs"]
mod layout;
use layout::plain_row;
use layout::status_row;
use layout::table_row;
use layout::table_rule;
use layout::terminal_detail_panel;
use layout::terminal_kv_panel;
use ratatui::prelude::*;
use ratatui::style::Stylize;
use serde_json::Value as JsonValue;

#[derive(Debug)]
pub(crate) struct DynamicToolCallCell {
    namespace: Option<String>,
    tool: String,
    raw_text: String,
    result_json: Option<JsonValue>,
    success: Option<bool>,
    duration_ms: Option<i64>,
}

pub(crate) fn new_dynamic_tool_call(
    namespace: Option<String>,
    tool: String,
    content_items: Option<Vec<DynamicToolCallOutputContentItem>>,
    success: Option<bool>,
    duration_ms: Option<i64>,
) -> DynamicToolCallCell {
    let raw_text = flatten_content_items(content_items.as_deref());
    let result_json = serde_json::from_str::<JsonValue>(&raw_text).ok();
    DynamicToolCallCell {
        namespace,
        tool,
        raw_text,
        result_json,
        success,
        duration_ms,
    }
}

impl HistoryCell for DynamicToolCallCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let mut lines = vec![tool_header(
            self.namespace.as_deref(),
            &self.tool,
            self.success,
            self.duration_ms,
        )];

        let body = self
            .result_json
            .as_ref()
            .and_then(|value| {
                render_dynamic_tool_body(self.namespace.as_deref(), &self.tool, value, width)
            })
            .unwrap_or_else(|| render_raw_fallback(&self.raw_text, width));

        if !body.is_empty() {
            lines.push(Line::from(""));
            lines.extend(body);
        }

        lines
    }
}

fn flatten_content_items(items: Option<&[DynamicToolCallOutputContentItem]>) -> String {
    let mut parts = Vec::new();
    if let Some(items) = items {
        for item in items {
            match item {
                DynamicToolCallOutputContentItem::InputText { text } => parts.push(text.clone()),
                DynamicToolCallOutputContentItem::InputImage { image_url } => {
                    parts.push(format!("<image output: {image_url}>"));
                }
            }
        }
    }
    parts.join("\n\n")
}

fn tool_header(
    namespace: Option<&str>,
    tool: &str,
    success: Option<bool>,
    duration_ms: Option<i64>,
) -> Line<'static> {
    let bullet = match success {
        Some(true) => "•".green().bold(),
        Some(false) => "•".red().bold(),
        None => "•".yellow().bold(),
    };
    let mut spans = vec![
        bullet,
        " ".into(),
        "ThinWedge".bold(),
        " ".into(),
        Span::from(format!("{}.{tool}", namespace.unwrap_or("tool"))).cyan(),
    ];
    if let Some(duration_ms) = duration_ms {
        spans.push(" ".into());
        spans.push(Span::from(format!("({duration_ms} ms)")).dim());
    }
    Line::from(spans)
}

fn render_dynamic_tool_body(
    namespace: Option<&str>,
    tool: &str,
    value: &JsonValue,
    width: u16,
) -> Option<Vec<Line<'static>>> {
    match (namespace, tool) {
        (Some("llmcosts"), "getModel") => render_llm_model(value, width),
        (Some("llmcosts"), "compareModels") => render_llm_comparison(value, width),
        (Some("infracosts"), "getAwsCostAndUsage")
        | (Some("infracosts"), "queryAwsByService")
        | (Some("infracosts"), "queryAwsByAccount") => render_aws_cost_query(value, width),
        (Some("infracosts"), "getAwsVmPrice") => render_aws_vm_price(value, width),
        (Some("infracosts"), "estimateAwsBoq") => render_aws_boq(value, width),
        (Some("trainingenvironments"), "get") => render_environment_detail(value, width),
        (Some("trainingenvironments"), "launch")
        | (Some("trainingenvironments"), "attach")
        | (Some("trainingenvironments"), "stop") => render_environment_action(value, width),
        (Some("statisticalmodels"), "get") => render_model_detail(value, width),
        (Some("statisticalmodels"), "submitJob") | (Some("statisticalmodels"), "getJob") => {
            render_job_detail(value, width)
        }
        _ => None,
    }
}

fn render_llm_model(value: &JsonValue, width: u16) -> Option<Vec<Line<'static>>> {
    let model = value.get("model")?;
    let cost = value.get("costContext");
    let title = format!(
        "LLM MARKET / {}",
        string_at(model, &["name"])
            .unwrap_or("MODEL")
            .to_ascii_uppercase()
    );
    let rows = vec![
        plain_row(
            "MODEL",
            string_at(model, &["name"])
                .unwrap_or("unknown model")
                .to_string(),
        ),
        plain_row(
            "CREATOR",
            string_at(model, &["model_creator", "name"])
                .unwrap_or("unknown creator")
                .to_string(),
        ),
        plain_row(
            "SLUG",
            string_at(model, &["slug"]).unwrap_or("n/a").to_string(),
        ),
        plain_row(
            "BLENDED",
            optional_number(cost, "blendedPricePer1MTokensUsd")
                .map(|value| format!("{}/1M", format_usd(value)))
                .unwrap_or_else(|| "n/a".to_string()),
        ),
        plain_row(
            "SPEED",
            optional_number(cost, "medianOutputTokensPerSecond")
                .map(|value| format!("{value:.1} tok/s"))
                .unwrap_or_else(|| "n/a".to_string()),
        ),
        plain_row(
            "TTFT",
            optional_number(cost, "medianTimeToFirstTokenSeconds")
                .map(|value| format!("{value:.2}s"))
                .unwrap_or_else(|| "n/a".to_string()),
        ),
        plain_row(
            "INTEL",
            optional_number(cost, "intelligenceIndex")
                .map(|value| format!("{value:.1}"))
                .unwrap_or_else(|| "n/a".to_string()),
        ),
        plain_row(
            "CODING",
            optional_number(cost, "codingIndex")
                .map(|value| format!("{value:.1}"))
                .unwrap_or_else(|| "n/a".to_string()),
        ),
    ];
    Some(terminal_kv_panel(
        &title,
        Some("live AA snapshot"),
        rows,
        width,
    ))
}

fn render_llm_comparison(value: &JsonValue, width: u16) -> Option<Vec<Line<'static>>> {
    let comparisons = value.get("comparisons")?.as_array()?;
    let mut rows = vec![table_row(
        &[16, 11, 8, 8, 8],
        &["MODEL", "BLENDED", "TOK/S", "TTFT", "CODING"],
    )];
    let mut best_coding: Option<(&str, f64)> = None;
    let mut best_latency: Option<(&str, f64)> = None;
    let mut cheapest: Option<(&str, f64)> = None;

    for entry in comparisons {
        let model = entry.get("model").unwrap_or(&JsonValue::Null);
        let cost = entry.get("costContext").unwrap_or(&JsonValue::Null);
        let name = string_at(model, &["name"]).unwrap_or("unknown model");
        let blended = optional_number(Some(cost), "blendedPricePer1MTokensUsd");
        let speed = optional_number(Some(cost), "medianOutputTokensPerSecond");
        let ttft = optional_number(Some(cost), "medianTimeToFirstTokenSeconds");
        let coding = optional_number(Some(cost), "codingIndex");

        if let Some(value) = coding
            && best_coding.is_none_or(|(_, best)| value > best)
        {
            best_coding = Some((name, value));
        }
        if let Some(value) = ttft
            && best_latency.is_none_or(|(_, best)| value < best)
        {
            best_latency = Some((name, value));
        }
        if let Some(value) = blended
            && cheapest.is_none_or(|(_, best)| value < best)
        {
            cheapest = Some((name, value));
        }

        rows.push(table_row(
            &[16, 11, 8, 8, 8],
            &[
                name,
                &blended
                    .map(format_usd_compact)
                    .unwrap_or_else(|| "n/a".to_string()),
                &speed
                    .map(|value| format!("{value:.0}"))
                    .unwrap_or_else(|| "n/a".to_string()),
                &ttft
                    .map(|value| format!("{value:.2}"))
                    .unwrap_or_else(|| "n/a".to_string()),
                &coding
                    .map(|value| format!("{value:.1}"))
                    .unwrap_or_else(|| "n/a".to_string()),
            ],
        ));
    }

    rows.push(table_rule(59));
    rows.push(format!(
        "CHEAPEST {}   BEST CODING {}   LOWEST TTFT {}",
        cheapest.map(|(name, _)| name).unwrap_or("n/a"),
        best_coding.map(|(name, _)| name).unwrap_or("n/a"),
        best_latency.map(|(name, _)| name).unwrap_or("n/a")
    ));
    Some(terminal_detail_panel(
        "LLM MARKET / COMPARE",
        Some("live AA snapshot"),
        rows,
        width,
    ))
}

fn render_aws_vm_price(value: &JsonValue, width: u16) -> Option<Vec<Line<'static>>> {
    let first_match = value.get("matches")?.as_array()?.first()?;
    let attributes = first_match.get("attributes").unwrap_or(&JsonValue::Null);
    let title = format!(
        "AWS VM / {}",
        string_at(attributes, &["instanceType"])
            .unwrap_or("INSTANCE")
            .to_ascii_uppercase()
    );
    let rows = vec![
        plain_row(
            "INSTANCE",
            string_at(attributes, &["instanceType"])
                .unwrap_or("unknown")
                .to_string(),
        ),
        plain_row(
            "LOCATION",
            string_at(attributes, &["location"])
                .unwrap_or("n/a")
                .to_string(),
        ),
        plain_row(
            "MEMORY",
            string_at(attributes, &["memory"])
                .unwrap_or("n/a")
                .to_string(),
        ),
        plain_row(
            "VCPU",
            string_at(attributes, &["vcpu"])
                .unwrap_or("n/a")
                .to_string(),
        ),
        plain_row(
            "LOWEST",
            first_match
                .get("lowestHourlyUsd")
                .and_then(JsonValue::as_f64)
                .map(|value| format!("{}/hr", format_usd(value)))
                .unwrap_or_else(|| "n/a".to_string()),
        ),
        plain_row(
            "MATCHES",
            value
                .get("matches")
                .and_then(JsonValue::as_array)
                .map(|matches| matches.len().to_string())
                .unwrap_or_else(|| "0".to_string()),
        ),
    ];
    Some(terminal_kv_panel(&title, Some("price list"), rows, width))
}

fn render_aws_cost_query(value: &JsonValue, width: u16) -> Option<Vec<Line<'static>>> {
    let response = value.get("response")?;
    let periods = response.get("ResultsByTime")?.as_array()?;
    let mut lines = vec![table_row(
        &[11, 11, 12, 20],
        &["START", "END", "TOTAL", "TOP GROUP"],
    )];

    for period in periods.iter().take(6) {
        let start = string_at(period, &["TimePeriod", "Start"]).unwrap_or("n/a");
        let end = string_at(period, &["TimePeriod", "End"]).unwrap_or("n/a");
        let total = period
            .pointer("/Total/UnblendedCost/Amount")
            .or_else(|| period.pointer("/Total/BlendedCost/Amount"))
            .and_then(JsonValue::as_str)
            .map(|value| format!("${value}"))
            .unwrap_or_else(|| "n/a".to_string());
        let top_group = period
            .get("Groups")
            .and_then(JsonValue::as_array)
            .and_then(|groups| groups.first())
            .map(render_cost_group)
            .unwrap_or_else(|| "n/a".to_string());
        lines.push(table_row(
            &[11, 11, 12, 20],
            &[start, end, &total, &top_group],
        ));
    }

    lines.push(table_rule(58));
    if let Some(service) = string_at(value, &["service"]) {
        lines.push(format!("SERVICE {service}"));
    }
    if let Some(accounts) = value.get("linkedAccounts").and_then(JsonValue::as_array)
        && !accounts.is_empty()
    {
        let joined = accounts
            .iter()
            .filter_map(JsonValue::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("ACCOUNTS {joined}"));
    }
    Some(terminal_detail_panel(
        "AWS COST / QUERY",
        Some("cost explorer"),
        lines,
        width,
    ))
}

fn render_aws_boq(value: &JsonValue, width: u16) -> Option<Vec<Line<'static>>> {
    let mut lines = vec![table_row(
        &[18, 8, 8, 10, 10],
        &["LINE ITEM", "QTY", "UNIT", "UNIT $", "EXT $"],
    )];
    let item_lines = value
        .get("lines")?
        .as_array()?
        .iter()
        .map(|line| {
            let label = string_at(line, &["label"]).unwrap_or("line item");
            let quantity = line
                .get("quantity")
                .and_then(JsonValue::as_f64)
                .unwrap_or_default();
            let unit = string_at(line, &["selectedUnit"]).unwrap_or("unit");
            let unit_price = line
                .get("selectedPricePerUnitUsd")
                .and_then(JsonValue::as_f64)
                .map(format_usd)
                .unwrap_or_else(|| "n/a".to_string());
            let total = line
                .get("estimatedCostUsd")
                .and_then(JsonValue::as_f64)
                .map(format_usd)
                .unwrap_or_else(|| "n/a".to_string());
            table_row(
                &[18, 8, 8, 10, 10],
                &[label, &format!("{quantity:.0}"), unit, &unit_price, &total],
            )
        })
        .collect::<Vec<_>>();
    lines.extend(item_lines);
    let total = value
        .get("totalEstimatedCostUsd")
        .and_then(JsonValue::as_f64)
        .map(format_usd)
        .unwrap_or_else(|| "n/a".to_string());
    lines.push(table_rule(58));
    lines.push(format!("SUBTOTAL {}", pad_left(&total, 46)));
    Some(terminal_detail_panel(
        "AWS BOQ / ESTIMATE",
        Some(&format!("total {total}")),
        lines,
        width,
    ))
}

fn render_environment_detail(value: &JsonValue, width: u16) -> Option<Vec<Line<'static>>> {
    let environment = value.get("environment")?;
    let mut rows = vec![
        plain_row(
            "ENV",
            string_at(environment, &["id"])
                .unwrap_or("unknown")
                .to_string(),
        ),
        status_row(
            "STATUS",
            string_at(environment, &["status"]).unwrap_or("unknown"),
        ),
        plain_row(
            "PROVIDER",
            string_at(environment, &["metadata", "provider"])
                .unwrap_or("local")
                .to_string(),
        ),
        plain_row(
            "WORKSPACE",
            string_at(environment, &["metadata", "runpod", "workspacePath"])
                .or_else(|| string_at(environment, &["workingDirectory"]))
                .unwrap_or("n/a")
                .to_string(),
        ),
    ];
    rows.push(plain_row(
        "ACTIONS",
        string_array(value.get("availableActions")).join(", "),
    ));
    let commands = command_summaries(value.get("toolCommands"));
    if !commands.is_empty() {
        rows.push(plain_row("COMMANDS", commands.join(" | ")));
    }
    Some(terminal_kv_panel(
        "ENVIRONMENT / DETAIL",
        Some("registry"),
        rows,
        width,
    ))
}

fn render_environment_action(value: &JsonValue, width: u16) -> Option<Vec<Line<'static>>> {
    let environment = value.get("environment")?;
    let execution = value.get("execution");
    let summary = execution.and_then(|execution| execution.get("summaryJson"));
    let rows = vec![
        plain_row(
            "ENV",
            string_at(environment, &["id"])
                .unwrap_or("unknown")
                .to_string(),
        ),
        status_row(
            "STATUS",
            summary
                .and_then(|summary| string_at(summary, &["status"]))
                .or_else(|| string_at(environment, &["status"]))
                .unwrap_or("unknown"),
        ),
        plain_row(
            "WORKSPACE",
            summary
                .and_then(|summary| string_at(summary, &["workspacePath"]))
                .unwrap_or("n/a")
                .to_string(),
        ),
        plain_row(
            "HTTP",
            summary
                .and_then(|summary| string_at(summary, &["httpEndpoint"]))
                .unwrap_or("n/a")
                .to_string(),
        ),
    ];
    Some(terminal_kv_panel(
        "ENVIRONMENT / ACTION",
        Some("attach ready"),
        rows,
        width,
    ))
}

fn render_model_detail(value: &JsonValue, width: u16) -> Option<Vec<Line<'static>>> {
    let model = value.get("model")?;
    let mut rows = vec![
        plain_row(
            "MODEL",
            string_at(model, &["name"])
                .or_else(|| string_at(model, &["id"]))
                .unwrap_or("unknown")
                .to_string(),
        ),
        plain_row(
            "ID",
            string_at(model, &["id"]).unwrap_or("unknown").to_string(),
        ),
        plain_row(
            "INFERENCE",
            string_at(model, &["inference", "providerId"])
                .unwrap_or("n/a")
                .to_string(),
        ),
        plain_row(
            "DEFAULT ENV",
            string_at(model, &["defaultEnvironmentId"])
                .unwrap_or("n/a")
                .to_string(),
        ),
        plain_row(
            "ACTIONS",
            string_array(value.get("availableActions")).join(", "),
        ),
    ];
    let commands = command_summaries(value.get("toolCommands"));
    if !commands.is_empty() {
        rows.push(plain_row("COMMANDS", commands.join(" | ")));
    }
    Some(terminal_kv_panel(
        "MODEL REGISTRY / DETAIL",
        Some("filesystem registry"),
        rows,
        width,
    ))
}

fn render_job_detail(value: &JsonValue, width: u16) -> Option<Vec<Line<'static>>> {
    let job = value.get("job").unwrap_or(value);
    let execution = value.get("execution").or_else(|| job.get("execution"));
    let summary = execution.and_then(|execution| execution.get("summaryJson"));
    let rows = vec![
        plain_row(
            "JOB",
            string_at(job, &["id"]).unwrap_or("unknown").to_string(),
        ),
        plain_row(
            "MODEL",
            string_at(job, &["modelId"])
                .unwrap_or("unknown")
                .to_string(),
        ),
        plain_row(
            "TYPE",
            string_at(job, &["type"]).unwrap_or("unknown").to_string(),
        ),
        status_row("STATUS", string_at(job, &["status"]).unwrap_or("unknown")),
        plain_row(
            "ENV",
            string_at(job, &["environmentId"])
                .unwrap_or("n/a")
                .to_string(),
        ),
        plain_row(
            "EXIT",
            execution
                .and_then(|execution| execution.get("exitCode"))
                .and_then(JsonValue::as_i64)
                .map(|value| value.to_string())
                .unwrap_or_else(|| "n/a".to_string()),
        ),
        plain_row(
            "ARTIFACTS",
            summary
                .and_then(|summary| string_at(summary, &["artifactManifestPath"]))
                .unwrap_or("n/a")
                .to_string(),
        ),
        plain_row(
            "GENERATED",
            summary
                .and_then(|summary| summary.get("generatedFiles"))
                .and_then(JsonValue::as_array)
                .map(|files| files.len().to_string())
                .unwrap_or_else(|| "0".to_string()),
        ),
    ];
    Some(terminal_kv_panel(
        "MODEL JOB / EXECUTION",
        Some("recorded summary"),
        rows,
        width,
    ))
}

fn render_raw_fallback(raw_text: &str, width: u16) -> Vec<Line<'static>> {
    if raw_text.trim().is_empty() {
        return Vec::new();
    }
    let text = format_and_truncate_tool_result(raw_text, 20, width as usize);
    text.split('\n')
        .map(|line| Line::from(Span::from(line.to_string()).dim()))
        .collect()
}

fn render_cost_group(group: &JsonValue) -> String {
    let key = group
        .get("Keys")
        .and_then(JsonValue::as_array)
        .and_then(|keys| keys.first())
        .and_then(JsonValue::as_str)
        .unwrap_or("group");
    let amount = group
        .pointer("/Metrics/UnblendedCost/Amount")
        .or_else(|| group.pointer("/Metrics/BlendedCost/Amount"))
        .and_then(JsonValue::as_str)
        .unwrap_or("n/a");
    format!("{key} ${amount}")
}

fn truncate_cell(value: &str, width: usize) -> String {
    let mut out = value.chars().take(width).collect::<String>();
    if out.chars().count() == width && value.chars().count() > width && width > 1 {
        out.pop();
        out.push('…');
    }
    out
}

fn pad_left(value: &str, width: usize) -> String {
    if value.len() >= width {
        value.to_string()
    } else {
        format!("{}{}", " ".repeat(width - value.len()), value)
    }
}

fn command_summaries(value: Option<&JsonValue>) -> Vec<String> {
    value
        .and_then(JsonValue::as_array)
        .into_iter()
        .flatten()
        .filter_map(|command| {
            let name = string_at(command, &["name"])?;
            let shell_command = string_at(command, &["command"]).unwrap_or("n/a");
            Some(format!("{name}={shell_command}"))
        })
        .collect()
}

fn string_array(value: Option<&JsonValue>) -> Vec<String> {
    value
        .and_then(JsonValue::as_array)
        .into_iter()
        .flatten()
        .filter_map(JsonValue::as_str)
        .map(str::to_string)
        .collect()
}

fn string_at<'a>(value: &'a JsonValue, path: &[&str]) -> Option<&'a str> {
    let mut cursor = value;
    for key in path {
        cursor = cursor.get(*key)?;
    }
    cursor.as_str()
}

fn optional_number(value: Option<&JsonValue>, key: &str) -> Option<f64> {
    value?.get(key)?.as_f64()
}

fn format_usd(value: f64) -> String {
    format!("${value:.4}")
}

fn format_usd_compact(value: f64) -> String {
    format!("${value:.2}")
}

#[cfg(test)]
#[path = "dynamic_tool_cell_tests.rs"]
mod tests;
