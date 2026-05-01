use super::*;
use crate::plugins::PluginInstallRequest;
use crate::plugins::PluginsManager;
use crate::plugins::test_support::load_plugins_config;
use crate::plugins::test_support::write_curated_plugin_sha;
use crate::plugins::test_support::write_plugins_feature_config;
use crate::plugins::test_support::write_thinwedge_curated_marketplace;
use core_test_support::PathExt;
use pretty_assertions::assert_eq;
use rmcp::model::ElicitationAction;
use serde_json::json;
use tempfile::tempdir;
use thinwedge_config::CONFIG_TOML_FILE;
use thinwedge_config::config_toml::ConfigToml;
use thinwedge_config::types::ToolSuggestConfig;
use thinwedge_config::types::ToolSuggestDisabledTool;
use thinwedge_config::types::ToolSuggestDiscoverable;
use thinwedge_config::types::ToolSuggestDiscoverableType;
use thinwedge_core_plugins::startup_sync::curated_plugins_repo_path;
use thinwedge_rmcp_client::ElicitationResponse;
use thinwedge_tools::DiscoverablePluginInfo;
use thinwedge_utils_absolute_path::AbsolutePathBuf;

#[tokio::test]
async fn verified_plugin_suggestion_completed_requires_installed_plugin() {
    let thinwedge_home = tempdir().expect("tempdir should succeed");
    let curated_root = curated_plugins_repo_path(thinwedge_home.path());
    write_thinwedge_curated_marketplace(&curated_root, &["sample"]);
    write_curated_plugin_sha(thinwedge_home.path());
    write_plugins_feature_config(thinwedge_home.path());

    let config = load_plugins_config(thinwedge_home.path()).await;
    let plugins_manager = PluginsManager::new(thinwedge_home.path().to_path_buf());

    assert!(!verified_plugin_suggestion_completed(
        "sample@thinwedge-curated",
        &config,
        &plugins_manager,
    ));

    plugins_manager
        .install_plugin(PluginInstallRequest {
            plugin_name: "sample".to_string(),
            marketplace_path: AbsolutePathBuf::try_from(
                curated_root.join(".agents/plugins/marketplace.json"),
            )
            .expect("marketplace path"),
        })
        .await
        .expect("plugin should install");

    let refreshed_config = load_plugins_config(thinwedge_home.path()).await;
    assert!(verified_plugin_suggestion_completed(
        "sample@thinwedge-curated",
        &refreshed_config,
        &plugins_manager,
    ));
}

#[test]
fn tool_suggest_response_persists_only_decline_always_mode() {
    assert!(tool_suggest_response_requests_persistent_disable(
        &ElicitationResponse {
            action: ElicitationAction::Decline,
            content: None,
            meta: Some(json!({ TOOL_SUGGEST_PERSIST_KEY: TOOL_SUGGEST_PERSIST_ALWAYS_VALUE })),
        }
    ));
    assert!(!tool_suggest_response_requests_persistent_disable(
        &ElicitationResponse {
            action: ElicitationAction::Accept,
            content: None,
            meta: Some(json!({ TOOL_SUGGEST_PERSIST_KEY: TOOL_SUGGEST_PERSIST_ALWAYS_VALUE })),
        }
    ));
    assert!(!tool_suggest_response_requests_persistent_disable(
        &ElicitationResponse {
            action: ElicitationAction::Decline,
            content: None,
            meta: Some(json!({ TOOL_SUGGEST_PERSIST_KEY: "session" })),
        }
    ));
    assert!(!tool_suggest_response_requests_persistent_disable(
        &ElicitationResponse {
            action: ElicitationAction::Decline,
            content: None,
            meta: None,
        }
    ));
}

#[tokio::test]
async fn persist_tool_suggest_disable_writes_connector_config() {
    let thinwedge_home = tempdir().expect("tempdir should succeed");
    let tool = connector_tool("connector_calendar", "Google Calendar");

    persist_tool_suggest_disable(&thinwedge_home.path().abs(), &tool)
        .await
        .expect("persist connector disable");

    let contents =
        std::fs::read_to_string(thinwedge_home.path().join(CONFIG_TOML_FILE)).expect("read config");
    let parsed: ConfigToml = toml::from_str(&contents).expect("parse config");
    assert_eq!(
        parsed.tool_suggest,
        Some(ToolSuggestConfig {
            discoverables: Vec::new(),
            disabled_tools: vec![ToolSuggestDisabledTool::connector("connector_calendar")],
        })
    );
}

#[tokio::test]
async fn persist_tool_suggest_disable_writes_plugin_config() {
    let thinwedge_home = tempdir().expect("tempdir should succeed");
    let tool = DiscoverableTool::Plugin(Box::new(DiscoverablePluginInfo {
        id: "slack@thinwedge-curated".to_string(),
        name: "Slack".to_string(),
        description: None,
        has_skills: true,
        mcp_server_names: Vec::new(),
        app_connector_ids: Vec::new(),
    }));

    persist_tool_suggest_disable(&thinwedge_home.path().abs(), &tool)
        .await
        .expect("persist plugin disable");

    let contents =
        std::fs::read_to_string(thinwedge_home.path().join(CONFIG_TOML_FILE)).expect("read config");
    let parsed: ConfigToml = toml::from_str(&contents).expect("parse config");
    assert_eq!(
        parsed.tool_suggest,
        Some(ToolSuggestConfig {
            discoverables: Vec::new(),
            disabled_tools: vec![ToolSuggestDisabledTool::plugin("slack@thinwedge-curated")],
        })
    );
}

#[tokio::test]
async fn persist_tool_suggest_disable_dedupes_existing_disabled_tools() {
    let thinwedge_home = tempdir().expect("tempdir should succeed");
    let tool = connector_tool("connector_calendar", "Google Calendar");
    std::fs::write(
        thinwedge_home.path().join(CONFIG_TOML_FILE),
        r#"
[tool_suggest]
discoverables = [
  { type = "plugin", id = "sample@thinwedge-curated" }
]

[[tool_suggest.disabled_tools]]
type = "connector"
id = " connector_calendar "

[[tool_suggest.disabled_tools]]
type = "connector"
id = "connector_calendar"

[[tool_suggest.disabled_tools]]
type = "connector"
id = "   "

[[tool_suggest.disabled_tools]]
type = "plugin"
id = "slack@thinwedge-curated"
"#,
    )
    .expect("write config");

    persist_tool_suggest_disable(&thinwedge_home.path().abs(), &tool)
        .await
        .expect("persist connector disable");

    let contents =
        std::fs::read_to_string(thinwedge_home.path().join(CONFIG_TOML_FILE)).expect("read config");
    let parsed: ConfigToml = toml::from_str(&contents).expect("parse config");
    assert_eq!(
        parsed.tool_suggest,
        Some(ToolSuggestConfig {
            discoverables: vec![ToolSuggestDiscoverable {
                kind: ToolSuggestDiscoverableType::Plugin,
                id: "sample@thinwedge-curated".to_string(),
            }],
            disabled_tools: vec![
                ToolSuggestDisabledTool::connector("connector_calendar"),
                ToolSuggestDisabledTool::plugin("slack@thinwedge-curated"),
            ],
        })
    );
}

fn connector_tool(id: &str, name: &str) -> DiscoverableTool {
    DiscoverableTool::Connector(Box::new(AppInfo {
        id: id.to_string(),
        name: name.to_string(),
        description: None,
        logo_url: None,
        logo_url_dark: None,
        distribution_channel: None,
        branding: None,
        app_metadata: None,
        labels: None,
        install_url: None,
        is_accessible: false,
        is_enabled: true,
        plugin_display_names: Vec::new(),
    }))
}
