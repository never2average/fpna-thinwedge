//! CLI login commands and their direct-user observability surfaces.
//!
//! The TUI path already installs a broader tracing stack with feedback, OpenTelemetry, and other
//! interactive-session layers. Direct `thinwedge login` intentionally does less: it preserves
//! straightforward stderr UX and adds only a small file-backed tracing layer for login-specific
//! targets. Keeping that setup local avoids pulling the TUI's session-oriented logging machinery
//! into a one-shot CLI command while still producing a durable `thinwedge-login.log` artifact that
//! support can request from users.

use std::fs::OpenOptions;
use std::io::IsTerminal;
use std::io::Read;
use thinwedge_app_server_protocol::AuthMode;
use thinwedge_core::config::Config;
use thinwedge_login::OPENROUTER_API_KEY_ENV_VAR;
use thinwedge_login::ThinWedgeAuth;
use thinwedge_login::login_with_agent_identity;
use thinwedge_login::login_with_api_key;
use thinwedge_login::logout_with_revoke;
use thinwedge_login::read_preferred_api_key_env_var_name;
use thinwedge_login::read_preferred_api_key_from_env;
use thinwedge_protocol::config_types::ForcedLoginMethod;
use thinwedge_utils_cli::CliConfigOverrides;
use tracing_appender::non_blocking;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

const MANAGED_LOGIN_DISABLED_MESSAGE: &str = "Managed browser login is not supported in ThinWedge. Set OPENROUTER_API_KEY and run `thinwedge login`, or pipe a token with `printenv OPENROUTER_API_KEY | thinwedge login --with-api-key`.";
const API_KEY_LOGIN_DISABLED_MESSAGE: &str =
    "API key login is disabled by configuration for this workspace.";
const AGENT_IDENTITY_LOGIN_DISABLED_MESSAGE: &str =
    "Agent Identity login is disabled. Use API key login instead.";
const LOGIN_SUCCESS_MESSAGE: &str = "Successfully logged in";

/// Installs a small file-backed tracing layer for direct `thinwedge login` flows.
///
/// This deliberately duplicates a narrow slice of the TUI logging setup instead of reusing it
/// wholesale. The TUI stack includes session-oriented layers that are valuable for interactive
/// runs but unnecessary for a one-shot login command. Keeping the direct CLI path local lets this
/// command produce a durable `thinwedge-login.log` artifact without coupling it to the TUI's broader
/// telemetry and feedback initialization.
fn init_login_file_logging(config: &Config) -> Option<WorkerGuard> {
    let log_dir = match thinwedge_core::config::log_dir(config) {
        Ok(log_dir) => log_dir,
        Err(err) => {
            eprintln!("Warning: failed to resolve login log directory: {err}");
            return None;
        }
    };

    if let Err(err) = std::fs::create_dir_all(&log_dir) {
        eprintln!(
            "Warning: failed to create login log directory {}: {err}",
            log_dir.display()
        );
        return None;
    }

    let mut log_file_opts = OpenOptions::new();
    log_file_opts.create(true).append(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        log_file_opts.mode(0o600);
    }

    let log_path = log_dir.join("thinwedge-login.log");
    let log_file = match log_file_opts.open(&log_path) {
        Ok(log_file) => log_file,
        Err(err) => {
            eprintln!(
                "Warning: failed to open login log file {}: {err}",
                log_path.display()
            );
            return None;
        }
    };

    let (non_blocking, guard) = non_blocking(log_file);
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("thinwedge_cli=info,thinwedge_core=info,thinwedge_login=info")
    });
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_target(true)
        .with_ansi(false)
        .with_filter(env_filter);

    // Direct `thinwedge login` otherwise relies on ephemeral stderr output.
    // Persist the same login targets to a file so support can inspect auth failures
    // without reproducing them through TUI or app-server.
    if let Err(err) = tracing_subscriber::registry().with(file_layer).try_init() {
        eprintln!(
            "Warning: failed to initialize login log file {}: {err}",
            log_path.display()
        );
        return None;
    }

    Some(guard)
}

fn print_api_token_login_help() {
    eprintln!(
        "ThinWedge uses OpenRouter-compatible API-token authentication.\n\nSet OPENROUTER_API_KEY and run:\n\n    thinwedge login\n\nOr pipe the token directly:\n\n    printenv OPENROUTER_API_KEY | thinwedge login --with-api-key\n\nThe token is stored locally in ThinWedge auth storage."
    );
}

pub async fn run_login_with_api_key(
    cli_config_overrides: CliConfigOverrides,
    api_key: String,
) -> ! {
    let config = load_config_or_exit(cli_config_overrides).await;
    let _login_log_guard = init_login_file_logging(&config);
    tracing::info!("starting api key login flow");

    if matches!(config.forced_login_method, Some(ForcedLoginMethod::Chatgpt)) {
        eprintln!("{API_KEY_LOGIN_DISABLED_MESSAGE}");
        std::process::exit(1);
    }

    match login_with_api_key(
        &config.thinwedge_home,
        &api_key,
        config.cli_auth_credentials_store_mode,
    ) {
        Ok(_) => {
            eprintln!("{LOGIN_SUCCESS_MESSAGE}");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("Error logging in: {e}");
            std::process::exit(1);
        }
    }
}

pub async fn run_login_with_preferred_api_key(cli_config_overrides: CliConfigOverrides) -> ! {
    match read_preferred_api_key_from_env() {
        Some(api_key) => {
            let env_var_name =
                read_preferred_api_key_env_var_name().unwrap_or(OPENROUTER_API_KEY_ENV_VAR);
            eprintln!("Reading API key from {env_var_name}...");
            run_login_with_api_key(cli_config_overrides, api_key).await;
        }
        None => {
            let config = load_config_or_exit(cli_config_overrides).await;
            let _login_log_guard = init_login_file_logging(&config);
            tracing::info!("api token login requested without an API key env var");
            print_api_token_login_help();
            std::process::exit(1);
        }
    }
}

pub async fn run_login_with_agent_identity(
    cli_config_overrides: CliConfigOverrides,
    agent_identity: String,
) -> ! {
    let config = load_config_or_exit(cli_config_overrides).await;
    let _login_log_guard = init_login_file_logging(&config);
    tracing::info!("starting agent identity login flow");

    if matches!(config.forced_login_method, Some(ForcedLoginMethod::Api)) {
        eprintln!("{AGENT_IDENTITY_LOGIN_DISABLED_MESSAGE}");
        std::process::exit(1);
    }

    match login_with_agent_identity(
        &config.thinwedge_home,
        &agent_identity,
        config.cli_auth_credentials_store_mode,
        Some(&config.chatgpt_base_url),
    )
    .await
    {
        Ok(_) => {
            eprintln!("{LOGIN_SUCCESS_MESSAGE}");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("Error logging in with Agent Identity: {e}");
            std::process::exit(1);
        }
    }
}

pub fn read_api_key_from_stdin() -> String {
    read_stdin_secret(
        "--with-api-key expects the API key on stdin. Try piping it, e.g. `printenv OPENROUTER_API_KEY | thinwedge login --with-api-key`.",
        "Reading API key from stdin...",
        "No API key provided via stdin.",
    )
}

pub fn read_agent_identity_from_stdin() -> String {
    read_stdin_secret(
        "--with-agent-identity expects the Agent Identity token on stdin. Try piping it, e.g. `printenv THINWEDGE_AGENT_IDENTITY | thinwedge login --with-agent-identity`.",
        "Reading Agent Identity token from stdin...",
        "No Agent Identity token provided via stdin.",
    )
}

fn read_stdin_secret(terminal_message: &str, reading_message: &str, empty_message: &str) -> String {
    let mut stdin = std::io::stdin();

    if stdin.is_terminal() {
        eprintln!("{terminal_message}");
        std::process::exit(1);
    }

    eprintln!("{reading_message}");

    let mut buffer = String::new();
    if let Err(err) = stdin.read_to_string(&mut buffer) {
        eprintln!("Failed to read stdin: {err}");
        std::process::exit(1);
    }

    let secret = buffer.trim().to_string();
    if secret.is_empty() {
        eprintln!("{empty_message}");
        std::process::exit(1);
    }

    secret
}

/// Legacy managed-auth device code flow.
pub async fn run_login_with_device_code(
    cli_config_overrides: CliConfigOverrides,
    _issuer_base_url: Option<String>,
    _client_id: Option<String>,
) -> ! {
    let config = load_config_or_exit(cli_config_overrides).await;
    let _login_log_guard = init_login_file_logging(&config);
    tracing::info!("starting device code login flow");
    eprintln!("{MANAGED_LOGIN_DISABLED_MESSAGE}");
    std::process::exit(1);
}

/// Legacy managed-auth fallback flow.
pub async fn run_login_with_device_code_fallback_to_browser(
    cli_config_overrides: CliConfigOverrides,
    _issuer_base_url: Option<String>,
    _client_id: Option<String>,
) -> ! {
    let config = load_config_or_exit(cli_config_overrides).await;
    let _login_log_guard = init_login_file_logging(&config);
    tracing::info!("starting login flow with device code fallback");
    eprintln!("{MANAGED_LOGIN_DISABLED_MESSAGE}");
    std::process::exit(1);
}

pub async fn run_login_status(cli_config_overrides: CliConfigOverrides) -> ! {
    let config = load_config_or_exit(cli_config_overrides).await;

    match ThinWedgeAuth::from_auth_storage(
        &config.thinwedge_home,
        config.cli_auth_credentials_store_mode,
        Some(&config.chatgpt_base_url),
    )
    .await
    {
        Ok(Some(auth)) => match auth.auth_mode() {
            AuthMode::ApiKey => match auth.get_token() {
                Ok(api_key) => {
                    eprintln!("Logged in using an API key - {}", safe_format_key(&api_key));
                    std::process::exit(0);
                }
                Err(e) => {
                    eprintln!("Unexpected error retrieving API key: {e}");
                    std::process::exit(1);
                }
            },
            AuthMode::Chatgpt | AuthMode::ChatgptAuthTokens => {
                eprintln!(
                    "Legacy managed login credentials are present, but ThinWedge now uses OpenRouter-compatible API-token authentication. Run `thinwedge logout`, then set OPENROUTER_API_KEY and run `thinwedge login`."
                );
                std::process::exit(1);
            }
            AuthMode::AgentIdentity => {
                eprintln!("Logged in using Agent Identity");
                std::process::exit(0);
            }
        },
        Ok(None) => {
            eprintln!("Not logged in");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Error checking login status: {e}");
            std::process::exit(1);
        }
    }
}

pub async fn run_logout(cli_config_overrides: CliConfigOverrides) -> ! {
    let config = load_config_or_exit(cli_config_overrides).await;

    match logout_with_revoke(
        &config.thinwedge_home,
        config.cli_auth_credentials_store_mode,
    )
    .await
    {
        Ok(true) => {
            eprintln!("Successfully logged out");
            std::process::exit(0);
        }
        Ok(false) => {
            eprintln!("Not logged in");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("Error logging out: {e}");
            std::process::exit(1);
        }
    }
}

async fn load_config_or_exit(cli_config_overrides: CliConfigOverrides) -> Config {
    let cli_overrides = match cli_config_overrides.parse_overrides() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error parsing -c overrides: {e}");
            std::process::exit(1);
        }
    };

    match Config::load_with_cli_overrides(cli_overrides).await {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Error loading configuration: {e}");
            std::process::exit(1);
        }
    }
}

fn safe_format_key(key: &str) -> String {
    if key.len() <= 13 {
        return "***".to_string();
    }
    let prefix = &key[..8];
    let suffix = &key[key.len() - 5..];
    format!("{prefix}***{suffix}")
}

#[cfg(test)]
mod tests {
    use super::safe_format_key;

    #[test]
    fn formats_long_key() {
        let key = "REDACTED_OPENAI_KEY";
        assert_eq!(safe_format_key(key), "sk-proj-***ABCDE");
    }

    #[test]
    fn short_key_returns_stars() {
        let key = "sk-proj-12345";
        assert_eq!(safe_format_key(key), "***");
    }
}
