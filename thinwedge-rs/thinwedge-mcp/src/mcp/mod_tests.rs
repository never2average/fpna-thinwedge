use super::*;
use crate::McpServerRegistration;
use pretty_assertions::assert_eq;
use std::collections::HashMap;
use std::path::PathBuf;
use thinwedge_config::Constrained;
use thinwedge_config::types::AppToolApproval;
use thinwedge_config::types::AuthKeyringBackendKind;
use thinwedge_login::ThinWedgeAuth;
use thinwedge_plugin::AppConnectorId;
use thinwedge_plugin::PluginCapabilitySummary;
use thinwedge_protocol::models::ManagedFileSystemPermissions;
use thinwedge_protocol::models::PermissionProfile;
use thinwedge_protocol::permissions::NetworkSandboxPolicy;
use thinwedge_protocol::protocol::AskForApproval;
use thinwedge_protocol::protocol::GranularApprovalConfig;

fn test_mcp_config(thinwedge_home: PathBuf) -> McpConfig {
    McpConfig {
        chatgpt_base_url: "https://chatgpt.com".to_string(),
        apps_mcp_product_sku: None,
        thinwedge_home,
        mcp_oauth_credentials_store_mode: OAuthCredentialsStoreMode::default(),
        auth_keyring_backend_kind: AuthKeyringBackendKind::default(),
        mcp_oauth_callback_port: None,
        mcp_oauth_callback_url: None,
        skill_mcp_dependency_install_enabled: true,
        approval_policy: Constrained::allow_any(AskForApproval::OnFailure),
        thinwedge_linux_sandbox_exe: None,
        use_legacy_landlock: false,
        apps_enabled: false,
        prefix_mcp_tool_names: true,
        client_elicitation_capability: ElicitationCapability::default(),
        mcp_server_catalog: ResolvedMcpCatalog::default(),
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
        McpPermissionPromptAutoApproveContext::default(),
    ));
    assert!(mcp_permission_prompt_is_auto_approved(
        AskForApproval::Never,
        &PermissionProfile::Managed {
            file_system: ManagedFileSystemPermissions::Unrestricted,
            network: NetworkSandboxPolicy::Restricted,
        },
        McpPermissionPromptAutoApproveContext::default(),
    ));
    assert!(!mcp_permission_prompt_is_auto_approved(
        AskForApproval::Never,
        &PermissionProfile::read_only(),
        McpPermissionPromptAutoApproveContext::default(),
    ));
    assert!(!mcp_permission_prompt_is_auto_approved(
        AskForApproval::OnRequest,
        &PermissionProfile::Managed {
            file_system: ManagedFileSystemPermissions::Unrestricted,
            network: NetworkSandboxPolicy::Enabled,
        },
        McpPermissionPromptAutoApproveContext::default(),
    ));
}

#[test]
fn mcp_prompt_auto_approval_honors_approved_tools_in_all_permission_modes() {
    for approval_policy in [
        AskForApproval::UnlessTrusted,
        AskForApproval::OnFailure,
        AskForApproval::OnRequest,
        AskForApproval::Granular(GranularApprovalConfig {
            sandbox_approval: true,
            rules: true,
            skill_approval: true,
            request_permissions: true,
            mcp_elicitations: true,
        }),
        AskForApproval::Never,
    ] {
        assert!(mcp_permission_prompt_is_auto_approved(
            approval_policy,
            &PermissionProfile::read_only(),
            McpPermissionPromptAutoApproveContext {
                tool_approval_mode: Some(AppToolApproval::Approve),
            },
        ));
    }

    assert!(!mcp_permission_prompt_is_auto_approved(
        AskForApproval::OnRequest,
        &PermissionProfile::read_only(),
        McpPermissionPromptAutoApproveContext {
            tool_approval_mode: Some(AppToolApproval::Auto),
        },
    ));
}

#[test]
fn mcp_prompt_auto_approval_rejects_auto_mode_in_default_permission_mode() {
    assert!(!mcp_permission_prompt_is_auto_approved(
        AskForApproval::OnRequest,
        &PermissionProfile::read_only(),
        McpPermissionPromptAutoApproveContext {
            tool_approval_mode: Some(AppToolApproval::Auto),
        },
    ));
}

#[test]
fn tool_plugin_provenance_collects_app_and_mcp_sources() {
    let mut config = test_mcp_config(PathBuf::new());
    let mut catalog = ResolvedMcpCatalog::builder();
    catalog.register(McpServerRegistration::from_plugin(
        "alpha".to_string(),
        "alpha@test".to_string(),
        /*plugin_order*/ 0,
        thinwedge_apps_mcp_server_config(
            "https://alpha.example",
            /*apps_mcp_product_sku*/ None,
        ),
    ));
    config.mcp_server_catalog = catalog.build();
    config.plugin_capability_summaries = vec![
        PluginCapabilitySummary {
            config_name: "alpha@test".to_string(),
            display_name: "alpha-plugin".to_string(),
            app_connector_ids: vec![AppConnectorId("connector_example".to_string())],
            mcp_server_names: vec!["alpha".to_string()],
            ..PluginCapabilitySummary::default()
        },
        PluginCapabilitySummary {
            config_name: "beta@test".to_string(),
            display_name: "beta-plugin".to_string(),
            app_connector_ids: vec![
                AppConnectorId("connector_example".to_string()),
                AppConnectorId("connector_gmail".to_string()),
            ],
            mcp_server_names: vec!["beta".to_string()],
            ..PluginCapabilitySummary::default()
        },
    ];
    let provenance = tool_plugin_provenance(&config);

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
            plugin_display_names_by_mcp_server_name: HashMap::from([(
                "alpha".to_string(),
                vec!["alpha-plugin".to_string()],
            )]),
            plugin_ids_by_mcp_server_name: HashMap::from([(
                "alpha".to_string(),
                "alpha@test".to_string(),
            )]),
        }
    );
    assert_eq!(
        provenance.plugin_id_for_mcp_server_name("alpha"),
        Some("alpha@test")
    );
    assert_eq!(provenance.plugin_id_for_mcp_server_name("beta"), None);
}

#[test]
fn thinwedge_apps_mcp_url_for_base_url_keeps_existing_paths() {
    assert_eq!(
        thinwedge_apps_mcp_url_for_base_url("https://chatgpt.com/backend-api"),
        "https://chatgpt.com/backend-api/wham/apps"
    );
    assert_eq!(
        thinwedge_apps_mcp_url_for_base_url("https://chat.openai.com"),
        "https://chat.openai.com/backend-api/wham/apps"
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
fn thinwedge_apps_server_config_uses_legacy_thinwedge_apps_path() {
    let config =
        thinwedge_apps_mcp_server_config("https://chatgpt.com", /*apps_mcp_product_sku*/ None);
    let url = match &config.transport {
        McpServerTransportConfig::StreamableHttp { url, .. } => url,
        _ => panic!("expected streamable http transport for thinwedge apps"),
    };

    assert_eq!(url, "https://chatgpt.com/backend-api/wham/apps");
}

#[test]
fn thinwedge_apps_server_config_forwards_configured_product_sku_header() {
    let config = thinwedge_apps_mcp_server_config("https://chatgpt.com", Some("tpp"));

    match &config.transport {
        McpServerTransportConfig::StreamableHttp {
            http_headers,
            env_http_headers,
            ..
        } => {
            assert_eq!(
                http_headers,
                &Some(HashMap::from([(
                    "X-OpenAI-Product-Sku".to_string(),
                    "tpp".to_string(),
                )]))
            );
            assert!(env_http_headers.is_none());
        }
        other => panic!("expected streamable http transport, got {other:?}"),
    }
}

#[tokio::test]
async fn effective_mcp_servers_preserve_runtime_servers() {
    let thinwedge_home = tempfile::tempdir().expect("tempdir");
    let mut config = test_mcp_config(thinwedge_home.path().to_path_buf());
    config.apps_enabled = true;
    let auth = ThinWedgeAuth::create_dummy_chatgpt_auth_for_testing();

    let mut catalog = ResolvedMcpCatalog::builder();
    catalog.register(McpServerRegistration::from_config(
        "sample".to_string(),
        McpServerConfig {
            transport: McpServerTransportConfig::StreamableHttp {
                url: "https://user.example/mcp".to_string(),
                bearer_token_env_var: None,
                http_headers: None,
                env_http_headers: None,
            },
            environment_id: thinwedge_config::DEFAULT_MCP_SERVER_ENVIRONMENT_ID.to_string(),
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
            oauth: None,
            oauth_resource: None,
            tools: HashMap::new(),
        },
    ));
    catalog.register(McpServerRegistration::from_config(
        "docs".to_string(),
        McpServerConfig {
            transport: McpServerTransportConfig::StreamableHttp {
                url: "https://docs.example/mcp".to_string(),
                bearer_token_env_var: None,
                http_headers: None,
                env_http_headers: None,
            },
            environment_id: thinwedge_config::DEFAULT_MCP_SERVER_ENVIRONMENT_ID.to_string(),
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
            oauth: None,
            oauth_resource: None,
            tools: HashMap::new(),
        },
    ));
    catalog.register(McpServerRegistration::from_config(
        THINWEDGE_APPS_MCP_SERVER_NAME.to_string(),
        thinwedge_apps_mcp_server_config(
            &config.chatgpt_base_url,
            config.apps_mcp_product_sku.as_deref(),
        ),
    ));
    config.mcp_server_catalog = catalog.build();

    let effective = effective_mcp_servers(&config, Some(&auth));

    let sample = effective.get("sample").expect("user server should exist");
    let docs = effective
        .get("docs")
        .expect("configured server should exist");
    let thinwedge_apps = effective
        .get(THINWEDGE_APPS_MCP_SERVER_NAME)
        .expect("thinwedge apps server should exist");

    let sample = sample
        .configured_config()
        .expect("configured server should retain transport");
    let docs = docs
        .configured_config()
        .expect("configured server should retain transport");
    let thinwedge_apps = thinwedge_apps
        .configured_config()
        .expect("thinwedge apps should use configured transport");

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
