use std::sync::Arc;

use pretty_assertions::assert_eq;
use thinwedge_config::McpServerTransportConfig;
use thinwedge_core::McpManager;
use thinwedge_core::config::Config;
use thinwedge_core::config::ConfigBuilder;
use thinwedge_core_plugins::PluginsManager;
use thinwedge_extension_api::ExtensionRegistryBuilder;
use thinwedge_extension_api::McpServerContribution;
use thinwedge_extension_api::McpServerContributionContext;
use thinwedge_extension_api::McpServerContributor;
use thinwedge_login::ThinWedgeAuth;
use thinwedge_mcp::THINWEDGE_APPS_MCP_SERVER_NAME;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn contributes_hosted_plugin_runtime_without_an_executor() -> TestResult {
    let thinwedge_home = tempfile::tempdir()?;
    let config = ConfigBuilder::default()
        .thinwedge_home(thinwedge_home.path().to_path_buf())
        .fallback_cwd(Some(thinwedge_home.path().to_path_buf()))
        .cli_overrides(vec![
            ("features.apps".to_string(), true.into()),
            ("chatgpt_base_url".to_string(), "https://chatgpt.com".into()),
        ])
        .build()
        .await?;
    let auth = ThinWedgeAuth::create_dummy_chatgpt_auth_for_testing();
    let manager = installed_manager(&config);

    let servers = manager.effective_servers(&config, Some(&auth)).await;
    let server = servers
        .get(THINWEDGE_APPS_MCP_SERVER_NAME)
        .and_then(|server| server.configured_config())
        .ok_or("hosted plugin runtime should be contributed as a configured server")?;
    let McpServerTransportConfig::StreamableHttp { url, .. } = &server.transport else {
        panic!("hosted plugin runtime should use streamable HTTP");
    };
    assert_eq!(url, "https://chatgpt.com/backend-api/ps/mcp");

    Ok(())
}

#[tokio::test]
async fn runtime_overlay_preserves_disabled_server() -> TestResult {
    let thinwedge_home = tempfile::tempdir()?;
    let config = ConfigBuilder::default()
        .thinwedge_home(thinwedge_home.path().to_path_buf())
        .fallback_cwd(Some(thinwedge_home.path().to_path_buf()))
        .cli_overrides(vec![
            ("features.apps".to_string(), true.into()),
            (
                "mcp_servers.thinwedge_apps.url".to_string(),
                "https://example.com/mcp".into(),
            ),
            (
                "mcp_servers.thinwedge_apps.enabled".to_string(),
                false.into(),
            ),
        ])
        .build()
        .await?;
    let auth = ThinWedgeAuth::create_dummy_chatgpt_auth_for_testing();
    let manager = installed_manager(&config);

    let servers = manager.effective_servers(&config, Some(&auth)).await;
    let server = servers
        .get(THINWEDGE_APPS_MCP_SERVER_NAME)
        .ok_or("hosted plugin runtime should remain configured")?;

    assert!(!server.enabled());
    Ok(())
}

#[tokio::test]
async fn legacy_fallback_overwrites_reserved_config_without_an_extension() -> TestResult {
    let thinwedge_home = tempfile::tempdir()?;
    let config = ConfigBuilder::default()
        .thinwedge_home(thinwedge_home.path().to_path_buf())
        .fallback_cwd(Some(thinwedge_home.path().to_path_buf()))
        .cli_overrides(vec![
            ("features.apps".to_string(), true.into()),
            (
                "mcp_servers.thinwedge_apps.url".to_string(),
                "https://example.com/mcp".into(),
            ),
        ])
        .build()
        .await?;
    let auth = ThinWedgeAuth::create_dummy_chatgpt_auth_for_testing();
    let manager = McpManager::new(Arc::new(PluginsManager::new(
        config.thinwedge_home.to_path_buf(),
    )));

    let servers = manager.effective_servers(&config, Some(&auth)).await;
    let server = servers
        .get(THINWEDGE_APPS_MCP_SERVER_NAME)
        .and_then(|server| server.configured_config())
        .ok_or("legacy Apps MCP should be present")?;
    let McpServerTransportConfig::StreamableHttp { url, .. } = &server.transport else {
        panic!("legacy Apps MCP should use streamable HTTP");
    };
    assert_eq!(url, "https://chatgpt.com/backend-api/wham/apps");

    Ok(())
}

#[tokio::test]
async fn later_extension_can_remove_same_name_registration() -> TestResult {
    let thinwedge_home = tempfile::tempdir()?;
    let config = ConfigBuilder::default()
        .thinwedge_home(thinwedge_home.path().to_path_buf())
        .fallback_cwd(Some(thinwedge_home.path().to_path_buf()))
        .cli_overrides(vec![("features.apps".to_string(), true.into())])
        .build()
        .await?;
    let auth = ThinWedgeAuth::create_dummy_chatgpt_auth_for_testing();
    let mut builder = ExtensionRegistryBuilder::new();
    thinwedge_mcp_extension::install(&mut builder);
    builder.mcp_server_contributor(Arc::new(RemoveThinWedgeApps));
    let manager = McpManager::new_with_extensions(
        Arc::new(PluginsManager::new(config.thinwedge_home.to_path_buf())),
        Arc::new(builder.build()),
    );

    let servers = manager.effective_servers(&config, Some(&auth)).await;

    assert!(!servers.contains_key(THINWEDGE_APPS_MCP_SERVER_NAME));
    Ok(())
}

#[tokio::test]
async fn hosted_apps_mcp_requires_chatgpt_auth() -> TestResult {
    let thinwedge_home = tempfile::tempdir()?;
    let config = ConfigBuilder::default()
        .thinwedge_home(thinwedge_home.path().to_path_buf())
        .fallback_cwd(Some(thinwedge_home.path().to_path_buf()))
        .cli_overrides(vec![("features.apps".to_string(), true.into())])
        .build()
        .await?;
    let auth = ThinWedgeAuth::from_api_key("test");
    let manager = installed_manager(&config);

    let servers = manager.effective_servers(&config, Some(&auth)).await;
    assert!(!servers.contains_key(THINWEDGE_APPS_MCP_SERVER_NAME));

    Ok(())
}

#[tokio::test]
async fn disabled_apps_remove_reserved_server_config_for_all_hosts() -> TestResult {
    let thinwedge_home = tempfile::tempdir()?;
    let config = ConfigBuilder::default()
        .thinwedge_home(thinwedge_home.path().to_path_buf())
        .fallback_cwd(Some(thinwedge_home.path().to_path_buf()))
        .cli_overrides(vec![
            ("features.apps".to_string(), false.into()),
            (
                "mcp_servers.thinwedge_apps.url".to_string(),
                "https://example.com/mcp".into(),
            ),
        ])
        .build()
        .await?;
    let managers = [
        installed_manager(&config),
        McpManager::new(Arc::new(PluginsManager::new(
            config.thinwedge_home.to_path_buf(),
        ))),
    ];
    for manager in managers {
        let servers = manager.runtime_servers(&config).await;
        assert!(!servers.contains_key(THINWEDGE_APPS_MCP_SERVER_NAME));
    }
    Ok(())
}

fn installed_manager(config: &Config) -> McpManager {
    let mut builder = ExtensionRegistryBuilder::new();
    thinwedge_mcp_extension::install(&mut builder);
    McpManager::new_with_extensions(
        Arc::new(PluginsManager::new(config.thinwedge_home.to_path_buf())),
        Arc::new(builder.build()),
    )
}

struct RemoveThinWedgeApps;

impl McpServerContributor<Config> for RemoveThinWedgeApps {
    fn id(&self) -> &'static str {
        "remove_thinwedge_apps"
    }

    fn contribute<'a>(
        &'a self,
        _context: McpServerContributionContext<'a, Config>,
    ) -> thinwedge_extension_api::ExtensionFuture<'a, Vec<McpServerContribution>> {
        Box::pin(async move {
            vec![McpServerContribution::Remove {
                name: THINWEDGE_APPS_MCP_SERVER_NAME.to_string(),
            }]
        })
    }
}
