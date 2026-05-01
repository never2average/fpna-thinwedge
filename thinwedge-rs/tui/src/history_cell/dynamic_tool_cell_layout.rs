use crate::history_cell::card_inner_width;
use crate::history_cell::with_border_with_inner_width;
use ratatui::prelude::*;
use ratatui::style::Stylize;
use textwrap::wrap;

const DYNAMIC_TOOL_MAX_INNER_WIDTH: usize = 76;

#[derive(Debug)]
pub(super) struct PanelRow {
    pub(super) label: String,
    value: PanelValue,
}

#[derive(Debug)]
enum PanelValue {
    Plain(String),
    Styled(Vec<Span<'static>>),
}

pub(super) fn plain_row(label: &str, value: String) -> PanelRow {
    PanelRow {
        label: label.to_string(),
        value: PanelValue::Plain(value),
    }
}

pub(super) fn status_row(label: &str, status: &str) -> PanelRow {
    PanelRow {
        label: label.to_string(),
        value: PanelValue::Styled(status_badge(status)),
    }
}

pub(super) fn terminal_kv_panel(
    title: &str,
    subtitle: Option<&str>,
    rows: Vec<PanelRow>,
    width: u16,
) -> Vec<Line<'static>> {
    let Some(inner_width) = card_inner_width(width, DYNAMIC_TOOL_MAX_INNER_WIDTH) else {
        return Vec::new();
    };
    let label_width = rows.iter().map(|row| row.label.len()).max().unwrap_or(0);
    let value_width = inner_width.saturating_sub(label_width + 2).max(1);
    let mut lines = terminal_panel_header(title, subtitle, inner_width);
    for row in rows {
        match row.value {
            PanelValue::Plain(value) => {
                let wrapped = wrap(&value, value_width);
                for (index, segment) in wrapped.iter().enumerate() {
                    let label_text = if index == 0 {
                        format!("{label:<label_width$}", label = row.label)
                    } else {
                        " ".repeat(label_width)
                    };
                    lines.push(Line::from(vec![
                        Span::from(label_text).dim(),
                        "  ".into(),
                        segment.to_string().into(),
                    ]));
                }
            }
            PanelValue::Styled(spans) => {
                lines.push(Line::from(vec![
                    Span::from(format!("{label:<label_width$}", label = row.label)).dim(),
                    "  ".into(),
                ]));
                if let Some(line) = lines.last_mut() {
                    line.spans.extend(spans);
                }
            }
        }
    }
    with_border_with_inner_width(lines, inner_width)
}

pub(super) fn terminal_detail_panel(
    title: &str,
    subtitle: Option<&str>,
    entries: Vec<String>,
    width: u16,
) -> Vec<Line<'static>> {
    let Some(inner_width) = card_inner_width(width, DYNAMIC_TOOL_MAX_INNER_WIDTH) else {
        return Vec::new();
    };
    let mut lines = terminal_panel_header(title, subtitle, inner_width);
    for entry in entries {
        for segment in wrap(&entry, inner_width.max(1)) {
            lines.push(Line::from(segment.to_string()));
        }
    }
    with_border_with_inner_width(lines, inner_width)
}

pub(super) fn table_row(widths: &[usize], columns: &[&str]) -> String {
    widths
        .iter()
        .zip(columns.iter())
        .map(|(width, value)| format!("{:width$}", truncate_cell(value, *width), width = *width))
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn table_rule(width: usize) -> String {
    "─".repeat(width)
}

fn terminal_panel_header(
    title: &str,
    subtitle: Option<&str>,
    inner_width: usize,
) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(Span::from(title.to_string()).bold())];
    if let Some(subtitle) = subtitle
        && !subtitle.is_empty()
    {
        lines.push(Line::from(Span::from(subtitle.to_string()).dim()));
    }
    lines.push(Line::from(Span::from("─".repeat(inner_width)).dim()));
    lines
}

fn status_badge(status: &str) -> Vec<Span<'static>> {
    let upper = status.to_ascii_uppercase();
    let badge = format!("[{upper}]");
    let styled = match upper.as_str() {
        "RUNNING" | "COMPLETED" => Span::from(badge).green().bold(),
        "FAILED" | "CANCELLED" | "TERMINATED" => Span::from(badge).red().bold(),
        "QUEUED" | "STARTING" | "STOPPING" => Span::from(badge).yellow().bold(),
        _ => Span::from(badge).cyan().bold(),
    };
    vec![styled]
}

fn truncate_cell(value: &str, width: usize) -> String {
    let mut out = value.chars().take(width).collect::<String>();
    if out.chars().count() == width && value.chars().count() > width && width > 1 {
        out.pop();
        out.push('…');
    }
    out
}
