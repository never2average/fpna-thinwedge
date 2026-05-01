use anyhow::Result;
use app_test_support::McpProcess;
use app_test_support::to_response;
use thinwedge_app_server_protocol::Account;
use thinwedge_app_server_protocol::AuthMode;
use thinwedge_app_server_protocol::GetAccountParams;
use thinwedge_app_server_protocol::GetAccountResponse;
use thinwedge_app_server_protocol::JSONRPCResponse;
use thinwedge_app_server_protocol::LoginAccountResponse;
use thinwedge_app_server_protocol::LogoutAccountResponse;
use thinwedge_app_server_protocol::RequestId;
use thinwedge_app_server_protocol::ServerNotification;
use thinwedge_config::types::AuthCredentialsStoreMode;
use thinwedge_login::login_with_api_key;
use pretty_assertions::assert_eq;
use std::path::Path;
use tempfile::TempDir;
use tokio::time::timeout;

const DEFAULT_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

#[derive(Default)]
struct CreateConfigTomlParams {
    forced_method: Option<String>,
    requires_openai_auth: Option<bool>,
    base_url: Option<String>,
    model_provider_id: Option<String>,
    extra_provider_config: Option<String>,
}

fn create_config_toml(thinwedge_home: &Path, params: CreateConfigTomlParams) -> std::io::Result<()> {
    let config_toml = thinwedge_home.join("config.toml");
    let base_url = params
        .base_url
        .unwrap_or_else(|| "http://127.0.0.1:0/v1".to_string());
    let forced_line = if let Some(method) = params.forced_method {
        format!("forced_login_method = \"{method}\"\n")
    } else {
        String::new()
    };
    let requires_line = match params.requires_openai_auth {
        Some(true) => "requires_openai_auth = true\n".to_string(),
        Some(false) => String::new(),
        None => String::new(),
    };
    let model_provider_id = params
        .model_provider_id
        .unwrap_or_else(|| "mock_provider".to_string());
    let provider_section = if model_provider_id == "mock_provider" {
        format!(
            r#"[model_providers.mock_provider]
name = "Mock provider for test"
base_url = "{base_url}"
wire_api = "responses"
request_max_retries = 0
stream_max_retries = 0
{requires_line}
"#
        )
    } else {
        params.extra_provider_config.unwrap_or_default()
    };
    let contents = format!(
        r#"
model = "mock-model"
approval_policy = "never"
sandbox_mode = "danger-full-access"
{forced_line}

model_provider = "{model_provider_id}"

[features]
shell_snapshot = false

{provider_section}
"#
    );
    std::fs::write(config_toml, contents)
}

#[tokio::test]
async fn logout_account_removes_auth_and_notifies() -> Result<()> {
    let thinwedge_home = TempDir::new()?;
    create_config_toml(thinwedge_home.path(), CreateConfigTomlParams::default())?;

    login_with_api_key(
        thinwedge_home.path(),
        "sk-test-key",
        AuthCredentialsStoreMode::File,
    )?;
    assert!(thinwedge_home.path().join("auth.json").exists());

    let mut mcp = McpProcess::new_with_env(thinwedge_home.path(), &[("OPENAI_API_KEY", None)]).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let id = mcp.send_logout_account_request().await?;
    let resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(id)),
    )
    .await??;
    let _ok: LogoutAccountResponse = to_response(resp)?;

    let note = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("account/updated"),
    )
    .await??;
    let parsed: ServerNotification = note.try_into()?;
    let ServerNotification::AccountUpdated(payload) = parsed else {
        panic!("unexpected notification: {parsed:?}");
    };
    assert_eq!(payload.auth_mode, None);
    assert_eq!(payload.plan_type, None);
    assert!(!thinwedge_home.path().join("auth.json").exists());

    let get_id = mcp
        .send_get_account_request(GetAccountParams {
            refresh_token: false,
        })
        .await?;
    let get_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(get_id)),
    )
    .await??;
    let account: GetAccountResponse = to_response(get_resp)?;
    assert_eq!(account.account, None);
    Ok(())
}

#[tokio::test]
async fn login_account_api_key_succeeds_and_notifies() -> Result<()> {
    let thinwedge_home = TempDir::new()?;
    create_config_toml(thinwedge_home.path(), CreateConfigTomlParams::default())?;

    let mut mcp = McpProcess::new(thinwedge_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_login_account_api_key_request("sk-test-key")
        .await?;
    let resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let login: LoginAccountResponse = to_response(resp)?;
    assert_eq!(login, LoginAccountResponse::ApiKey {});

    let note = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("account/login/completed"),
    )
    .await??;
    let parsed: ServerNotification = note.try_into()?;
    let ServerNotification::AccountLoginCompleted(payload) = parsed else {
        panic!("unexpected notification: {parsed:?}");
    };
    assert_eq!(payload.login_id, None);
    assert_eq!(payload.success, true);
    assert_eq!(payload.error, None);

    let note = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("account/updated"),
    )
    .await??;
    let parsed: ServerNotification = note.try_into()?;
    let ServerNotification::AccountUpdated(payload) = parsed else {
        panic!("unexpected notification: {parsed:?}");
    };
    assert_eq!(payload.auth_mode, Some(AuthMode::ApiKey));
    assert_eq!(payload.plan_type, None);
    assert!(thinwedge_home.path().join("auth.json").exists());
    Ok(())
}

#[tokio::test]
async fn login_account_api_key_ignores_forced_chatgpt_config() -> Result<()> {
    let thinwedge_home = TempDir::new()?;
    create_config_toml(
        thinwedge_home.path(),
        CreateConfigTomlParams {
            forced_method: Some("chatgpt".to_string()),
            ..Default::default()
        },
    )?;

    let mut mcp = McpProcess::new(thinwedge_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_login_account_api_key_request("sk-test-key")
        .await?;
    let resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let login: LoginAccountResponse = to_response(resp)?;
    assert_eq!(login, LoginAccountResponse::ApiKey {});
    Ok(())
}

#[tokio::test]
async fn get_account_no_auth() -> Result<()> {
    let thinwedge_home = TempDir::new()?;
    create_config_toml(
        thinwedge_home.path(),
        CreateConfigTomlParams {
            requires_openai_auth: Some(true),
            ..Default::default()
        },
    )?;

    let mut mcp = McpProcess::new_with_env(thinwedge_home.path(), &[("OPENAI_API_KEY", None)]).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_get_account_request(GetAccountParams {
            refresh_token: false,
        })
        .await?;
    let resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let account: GetAccountResponse = to_response(resp)?;
    assert_eq!(
        account,
        GetAccountResponse {
            account: None,
            requires_openai_auth: true,
        }
    );
    Ok(())
}

#[tokio::test]
async fn get_account_with_api_key() -> Result<()> {
    let thinwedge_home = TempDir::new()?;
    create_config_toml(
        thinwedge_home.path(),
        CreateConfigTomlParams {
            requires_openai_auth: Some(true),
            ..Default::default()
        },
    )?;

    let mut mcp = McpProcess::new(thinwedge_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let login_request_id = mcp
        .send_login_account_api_key_request("sk-test-key")
        .await?;
    let login_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(login_request_id)),
    )
    .await??;
    let _login: LoginAccountResponse = to_response(login_resp)?;

    let request_id = mcp
        .send_get_account_request(GetAccountParams {
            refresh_token: false,
        })
        .await?;
    let resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let received: GetAccountResponse = to_response(resp)?;
    assert_eq!(
        received,
        GetAccountResponse {
            account: Some(Account::ApiKey {}),
            requires_openai_auth: true,
        }
    );
    Ok(())
}

#[tokio::test]
async fn get_account_when_auth_not_required() -> Result<()> {
    let thinwedge_home = TempDir::new()?;
    create_config_toml(
        thinwedge_home.path(),
        CreateConfigTomlParams {
            requires_openai_auth: Some(false),
            ..Default::default()
        },
    )?;

    let mut mcp = McpProcess::new(thinwedge_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_get_account_request(GetAccountParams {
            refresh_token: false,
        })
        .await?;
    let resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let received: GetAccountResponse = to_response(resp)?;
    assert_eq!(
        received,
        GetAccountResponse {
            account: None,
            requires_openai_auth: false,
        }
    );
    Ok(())
}

#[tokio::test]
async fn get_account_with_aws_provider() -> Result<()> {
    let thinwedge_home = TempDir::new()?;
    create_config_toml(
        thinwedge_home.path(),
        CreateConfigTomlParams {
            model_provider_id: Some("amazon-bedrock".to_string()),
            extra_provider_config: Some(
                r#"[model_providers.amazon-bedrock.aws]
profile = "thinwedge-bedrock"
region = "us-west-2"
"#
                .to_string(),
            ),
            ..Default::default()
        },
    )?;

    let mut mcp = McpProcess::new(thinwedge_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_get_account_request(GetAccountParams {
            refresh_token: false,
        })
        .await?;
    let resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let received: GetAccountResponse = to_response(resp)?;
    assert_eq!(
        received,
        GetAccountResponse {
            account: Some(Account::AmazonBedrock {}),
            requires_openai_auth: false,
        }
    );
    Ok(())
}
