use super::*;
use pretty_assertions::assert_eq;
use std::collections::HashMap;
use std::path::PathBuf;
use thinwedge_config::Constrained;
use thinwedge_login::ThinWedgeAuth;
use thinwedge_plugin::AppConnectorId;
use thinwedge_plugin::PluginCapabilitySummary;
use thinwedge_protocol::models::ManagedFileSystemPermissions;
use thinwedge_protocol::models::PermissionProfile;
use thinwedge_protocol::permissions::NetworkSandboxPolicy;
use thinwedge_protocol::protocol::AskForApproval;

fn test_mcp_config(thinwedge_home: PathBuf) -> McpConfig {
    McpConfig {
        chatgpt_base_url: "https://chatgpt.com".to_string(),
        thinwedge_home,
        mcp_oauth_credentials_store_mode: OAuthCredentialsStoreMode::default(),
        mcp_oauth_callback_port: None,
        mcp_oauth_callback_url: None,
        skill_mcp_dependency_install_enabled: true,
        approval_policy: Constrained::allow_any(AskForApproval::OnFailure),
        thinwedge_linux_sandbox_exe: None,
        use_legacy_landlock: false,
        apps_enabled: false,
        configured_mcp_servers: HashMap::new(),
        plugin_capability_summaries: Vec::new(),
    }
}

#[test]
fn qualified_mcp_tool_name_prefix_sanitizes_server_names_without_lowercasing() {
    assert_eq!(
        qualified_mcp_tool_name_prefix("Some-Server"),
        "mcp__Some_Server__".to_string()
    );
}

#[test]
fn mcp_prompt_auto_approval_honors_unrestricted_managed_profiles() {
    assert!(mcp_permission_prompt_is_auto_approved(
        AskForApproval::Never,
        &PermissionProfile::Managed {
            file_system: ManagedFileSystemPermissions::Unrestricted,
            network: NetworkSandboxPolicy::Enabled,
        },
    ));
    assert!(mcp_permission_prompt_is_auto_approved(
        AskForApproval::Never,
        &PermissionProfile::Managed {
            file_system: ManagedFileSystemPermissions::Unrestricted,
            network: NetworkSandboxPolicy::Restricted,
        },
    ));
    assert!(!mcp_permission_prompt_is_auto_approved(
        AskForApproval::Never,
        &PermissionProfile::read_only(),
    ));
    assert!(!mcp_permission_prompt_is_auto_approved(
        AskForApproval::OnRequest,
        &PermissionProfile::Managed {
            file_system: ManagedFileSystemPermissions::Unrestricted,
            network: NetworkSandboxPolicy::Enabled,
        },
    ));
}

#[test]
fn tool_plugin_provenance_collects_app_and_mcp_sources() {
    let provenance = ToolPluginProvenance::from_capability_summaries(&[
        PluginCapabilitySummary {
            display_name: "alpha-plugin".to_string(),
            app_connector_ids: vec![AppConnectorId("connector_example".to_string())],
            mcp_server_names: vec!["alpha".to_string()],
            ..PluginCapabilitySummary::default()
        },
        PluginCapabilitySummary {
            display_name: "beta-plugin".to_string(),
            app_connector_ids: vec![
                AppConnectorId("connector_example".to_string()),
                AppConnectorId("connector_gmail".to_string()),
            ],
            mcp_server_names: vec!["beta".to_string()],
            ..PluginCapabilitySummary::default()
        },
    ]);

    assert_eq!(
        provenance,
        ToolPluginProvenance {
            plugin_display_names_by_connector_id: HashMap::from([
                (
                    "connector_example".to_string(),
                    vec!["alpha-plugin".to_string(), "beta-plugin".to_string()],
                ),
                (
                    "connector_gmail".to_string(),
                    vec!["beta-plugin".to_string()],
                ),
            ]),
            plugin_display_names_by_mcp_server_name: HashMap::from([
                ("alpha".to_string(), vec!["alpha-plugin".to_string()]),
                ("beta".to_string(), vec!["beta-plugin".to_string()]),
            ]),
        }
    );
}

#[test]
fn thinwedge_apps_mcp_url_for_base_url_keeps_existing_paths() {
    assert_eq!(
        thinwedge_apps_mcp_url_for_base_url("https://chatgpt.com/backend-api"),
        "https://chatgpt.com/backend-api/wham/apps"
    );
    assert_eq!(
        thinwedge_apps_mcp_url_for_base_url("https://chat.thinwedge.com"),
        "https://chat.thinwedge.com/backend-api/wham/apps"
    );
    assert_eq!(
        thinwedge_apps_mcp_url_for_base_url("http://localhost:8080/api/thinwedge"),
        "http://localhost:8080/api/thinwedge/apps"
    );
    assert_eq!(
        thinwedge_apps_mcp_url_for_base_url("http://localhost:8080"),
        "http://localhost:8080/api/thinwedge/apps"
    );
}

#[test]
fn thinwedge_apps_mcp_url_uses_legacy_thinwedge_apps_path() {
    let config = test_mcp_config(PathBuf::from("/tmp"));

    assert_eq!(
        thinwedge_apps_mcp_url(&config),
        "https://chatgpt.com/backend-api/wham/apps"
    );
}

#[test]
fn thinwedge_apps_server_config_uses_legacy_thinwedge_apps_path() {
    let mut config = test_mcp_config(PathBuf::from("/tmp"));
    let auth = ThinWedgeAuth::create_dummy_chatgpt_auth_for_testing();

    let mut servers = with_thinwedge_apps_mcp(HashMap::new(), /*auth*/ None, &config);
    assert!(!servers.contains_key(THINWEDGE_APPS_MCP_SERVER_NAME));

    config.apps_enabled = true;

    servers = with_thinwedge_apps_mcp(servers, Some(&auth), &config);
    let server = servers
        .get(THINWEDGE_APPS_MCP_SERVER_NAME)
        .expect("thinwedge apps should be present when apps is enabled");
    let url = match &server.transport {
        McpServerTransportConfig::StreamableHttp { url, .. } => url,
        _ => panic!("expected streamable http transport for thinwedge apps"),
    };

    assert_eq!(url, "https://chatgpt.com/backend-api/wham/apps");
}

#[tokio::test]
async fn effective_mcp_servers_preserve_user_servers_and_add_thinwedge_apps() {
    let thinwedge_home = tempfile::tempdir().expect("tempdir");
    let mut config = test_mcp_config(thinwedge_home.path().to_path_buf());
    config.apps_enabled = true;
    let auth = ThinWedgeAuth::create_dummy_chatgpt_auth_for_testing();

    config.configured_mcp_servers.insert(
        "sample".to_string(),
        McpServerConfig {
            transport: McpServerTransportConfig::StreamableHttp {
                url: "https://user.example/mcp".to_string(),
                bearer_token_env_var: None,
                http_headers: None,
                env_http_headers: None,
            },
            experimental_environment: None,
            enabled: true,
            required: false,
            supports_parallel_tool_calls: false,
            disabled_reason: None,
            startup_timeout_sec: None,
            tool_timeout_sec: None,
            default_tools_approval_mode: None,
            enabled_tools: None,
            disabled_tools: None,
            scopes: None,
            oauth_resource: None,
            tools: HashMap::new(),
        },
    );
    config.configured_mcp_servers.insert(
        "docs".to_string(),
        McpServerConfig {
            transport: McpServerTransportConfig::StreamableHttp {
                url: "https://docs.example/mcp".to_string(),
                bearer_token_env_var: None,
                http_headers: None,
                env_http_headers: None,
            },
            experimental_environment: None,
            enabled: true,
            required: false,
            supports_parallel_tool_calls: false,
            disabled_reason: None,
            startup_timeout_sec: None,
            tool_timeout_sec: None,
            default_tools_approval_mode: None,
            enabled_tools: None,
            disabled_tools: None,
            scopes: None,
            oauth_resource: None,
            tools: HashMap::new(),
        },
    );

    let effective = effective_mcp_servers(&config, Some(&auth));

    let sample = effective.get("sample").expect("user server should exist");
    let docs = effective
        .get("docs")
        .expect("configured server should exist");
    let thinwedge_apps = effective
        .get(THINWEDGE_APPS_MCP_SERVER_NAME)
        .expect("thinwedge apps server should exist");

    match &sample.transport {
        McpServerTransportConfig::StreamableHttp { url, .. } => {
            assert_eq!(url, "https://user.example/mcp");
        }
        other => panic!("expected streamable http transport, got {other:?}"),
    }
    match &docs.transport {
        McpServerTransportConfig::StreamableHttp { url, .. } => {
            assert_eq!(url, "https://docs.example/mcp");
        }
        other => panic!("expected streamable http transport, got {other:?}"),
    }
    match &thinwedge_apps.transport {
        McpServerTransportConfig::StreamableHttp { url, .. } => {
            assert_eq!(url, "https://chatgpt.com/backend-api/wham/apps");
        }
        other => panic!("expected streamable http transport, got {other:?}"),
    }
}
