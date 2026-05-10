use anyhow::Context;
use clap::Parser;
use clap::ValueEnum;
use std::io::IsTerminal;
use std::io::Write;
use std::process::Command;
use thinwedge_config::config_toml::ConfigToml;
use thinwedge_config::types::ArdentConfigToml;
use thinwedge_config::types::AwsIdentityConfigToml;
use thinwedge_config::types::DbSandboxConfigToml;
use thinwedge_core::config::Config;
use thinwedge_core::config::edit::ConfigEdit;
use thinwedge_core::config::edit::ConfigEditsBuilder;
use thinwedge_core::config::find_thinwedge_home;
use thinwedge_utils_cli::CliConfigOverrides;
use toml_edit::Item as TomlItem;
use toml_edit::value;

const DEFAULT_PROVIDER: DbSandboxProviderArg = DbSandboxProviderArg::Neon;
const DEFAULT_SOURCE_URL_ENV: &str = "THINWEDGE_ARDENT_SOURCE_DATABASE_URL";
const DEFAULT_NEON_API_KEY_ENV: &str = "THINWEDGE_NEON_API_KEY";
const DEFAULT_NEON_PROJECT_ID_ENV: &str = "THINWEDGE_NEON_PROJECT_ID";

#[derive(Debug, Parser)]
#[command(bin_name = "thinwedge db-sandbox")]
pub struct DbSandboxCli {
    #[clap(skip)]
    pub config_overrides: CliConfigOverrides,

    #[command(subcommand)]
    command: DbSandboxSubcommand,
}

#[derive(Debug, clap::Subcommand)]
enum DbSandboxSubcommand {
    /// Show ThinWedge DB sandbox configuration and the matching probe command.
    Status(DbSandboxStatusCommand),
    /// Save non-secret DB sandbox setup.
    Configure(DbSandboxConfigureCommand),
    /// Run the bottom-up DB sandbox probe script when available.
    Preflight(DbSandboxPreflightCommand),
}

#[derive(Debug, Parser)]
struct DbSandboxStatusCommand {
    /// Show the equivalent probe command without running anything.
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, Parser, Default)]
struct DbSandboxConfigureCommand {
    /// Enable DB sandbox setup in config.toml.
    #[arg(long = "enabled", conflicts_with = "disabled")]
    enable: bool,

    /// Disable DB sandbox setup in config.toml.
    #[arg(long = "disabled", conflicts_with = "enabled")]
    disable: bool,

    /// Do not prompt for missing values.
    #[arg(long)]
    no_prompt: bool,

    /// Show the config edits that would be saved.
    #[arg(long)]
    dry_run: bool,

    /// Source DB provider. Defaults to Neon when omitted in interactive setup.
    #[arg(long, value_enum)]
    provider: Option<DbSandboxProviderArg>,

    /// Env var that contains the source Postgres URL for URL-based providers.
    #[arg(long)]
    source_url_env: Option<String>,

    /// Env var that contains a Neon API key.
    #[arg(long)]
    neon_api_key_env: Option<String>,

    /// Env var that contains the Neon project id.
    #[arg(long)]
    neon_project_id_env: Option<String>,

    /// Optional non-secret Neon project id.
    #[arg(long)]
    neon_project_id: Option<String>,

    /// Optional branch backend. Use `ardent` only after deciding to connect Ardent.
    #[arg(long, value_enum)]
    branch_backend: Option<DbSandboxBackendArg>,
}

#[derive(Debug, Parser)]
struct DbSandboxPreflightCommand {
    /// Source provider to probe. Defaults to config or Neon.
    #[arg(long, value_enum)]
    provider: Option<DbSandboxProviderArg>,

    /// Include mutation-gated branch lifecycle checks when backend supports them.
    #[arg(long)]
    include_branch_lifecycle: bool,

    /// Allow mutation-gated probe steps. Requires explicit approval.
    #[arg(long)]
    allow_mutation: bool,

    /// Print the probe command without running it.
    #[arg(long)]
    dry_run: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
enum DbSandboxProviderArg {
    Neon,
    Postgresql,
    Rds,
    Supabase,
    Planetscale,
}

impl DbSandboxProviderArg {
    fn as_config_value(self) -> &'static str {
        match self {
            Self::Neon => "neon",
            Self::Postgresql => "postgresql",
            Self::Rds => "rds",
            Self::Supabase => "supabase",
            Self::Planetscale => "planetscale",
        }
    }

    fn from_config(value: Option<thinwedge_config::types::DbSandboxProviderToml>) -> Self {
        match value {
            Some(thinwedge_config::types::DbSandboxProviderToml::Postgresql) => Self::Postgresql,
            Some(thinwedge_config::types::DbSandboxProviderToml::Rds) => Self::Rds,
            Some(thinwedge_config::types::DbSandboxProviderToml::Supabase) => Self::Supabase,
            Some(thinwedge_config::types::DbSandboxProviderToml::Planetscale) => Self::Planetscale,
            Some(thinwedge_config::types::DbSandboxProviderToml::Neon) | None => Self::Neon,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
enum DbSandboxBackendArg {
    None,
    Ardent,
}

impl DbSandboxBackendArg {
    fn as_config_value(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Ardent => "ardent",
        }
    }

    fn from_config(value: Option<thinwedge_config::types::DbSandboxBackendToml>) -> Self {
        match value {
            Some(thinwedge_config::types::DbSandboxBackendToml::Ardent) => Self::Ardent,
            Some(thinwedge_config::types::DbSandboxBackendToml::None) | None => Self::None,
        }
    }
}

#[derive(Debug)]
struct LoadedDbSandboxConfig {
    billing: Option<AwsIdentityConfigToml>,
    db_ops: Option<AwsIdentityConfigToml>,
    db_sandbox: Option<DbSandboxConfigToml>,
    ardent: Option<ArdentConfigToml>,
}

#[derive(Default)]
struct DbSandboxConfigUpdates {
    enabled: Option<bool>,
    provider: Option<String>,
    source_url_env: Option<String>,
    neon_api_key_env: Option<String>,
    neon_project_id_env: Option<String>,
    neon_project_id: Option<String>,
    branch_backend: Option<String>,
}

impl DbSandboxConfigUpdates {
    fn into_edits(self) -> Vec<ConfigEdit> {
        let mut edits = Vec::new();
        push_bool_edit(&mut edits, &["db_sandbox", "enabled"], self.enabled);
        push_string_edit(&mut edits, &["db_sandbox", "provider"], self.provider);
        push_string_edit(
            &mut edits,
            &["db_sandbox", "source_url_env"],
            self.source_url_env,
        );
        push_string_edit(
            &mut edits,
            &["db_sandbox", "neon_api_key_env"],
            self.neon_api_key_env,
        );
        push_string_edit(
            &mut edits,
            &["db_sandbox", "neon_project_id_env"],
            self.neon_project_id_env,
        );
        push_string_edit(
            &mut edits,
            &["db_sandbox", "neon_project_id"],
            self.neon_project_id,
        );
        push_string_edit(
            &mut edits,
            &["db_sandbox", "branch_backend"],
            self.branch_backend,
        );
        edits
    }
}

pub async fn run_db_sandbox_cli(cli: DbSandboxCli) -> anyhow::Result<()> {
    let cli_overrides = cli
        .config_overrides
        .parse_overrides()
        .map_err(anyhow::Error::msg)?;
    match cli.command {
        DbSandboxSubcommand::Status(cmd) => run_status(cli_overrides, cmd).await,
        DbSandboxSubcommand::Configure(cmd) => run_configure(cmd).await,
        DbSandboxSubcommand::Preflight(cmd) => run_preflight(cli_overrides, cmd).await,
    }
}

async fn run_status(
    cli_overrides: Vec<(String, toml::Value)>,
    cmd: DbSandboxStatusCommand,
) -> anyhow::Result<()> {
    let loaded = load_db_sandbox_config(cli_overrides).await?;
    let provider =
        DbSandboxProviderArg::from_config(loaded.db_sandbox.as_ref().and_then(|cfg| cfg.provider));
    let backend = DbSandboxBackendArg::from_config(
        loaded
            .db_sandbox
            .as_ref()
            .and_then(|cfg| cfg.branch_backend),
    );
    println!("ThinWedge DB sandbox config:");
    println!(
        "  db_sandbox.enabled: {}",
        loaded
            .db_sandbox
            .as_ref()
            .and_then(|cfg| cfg.enabled)
            .unwrap_or(false)
    );
    println!("  db_sandbox.provider: {}", provider.as_config_value());
    println!("  db_sandbox.branch_backend: {}", backend.as_config_value());
    print_env_ref(
        "db_sandbox.source_url_env",
        loaded
            .db_sandbox
            .as_ref()
            .and_then(|cfg| cfg.source_url_env.as_deref())
            .unwrap_or(DEFAULT_SOURCE_URL_ENV),
    );
    print_env_ref(
        "db_sandbox.neon_api_key_env",
        loaded
            .db_sandbox
            .as_ref()
            .and_then(|cfg| cfg.neon_api_key_env.as_deref())
            .unwrap_or(DEFAULT_NEON_API_KEY_ENV),
    );
    print_env_ref(
        "db_sandbox.neon_project_id_env",
        loaded
            .db_sandbox
            .as_ref()
            .and_then(|cfg| cfg.neon_project_id_env.as_deref())
            .unwrap_or(DEFAULT_NEON_PROJECT_ID_ENV),
    );
    println!(
        "  db_sandbox.neon_project_id: {}",
        loaded
            .db_sandbox
            .as_ref()
            .and_then(|cfg| cfg.neon_project_id.as_deref())
            .unwrap_or("<unset>")
    );
    print_identity_summary("billing", loaded.billing.as_ref());
    print_identity_summary("db_ops", loaded.db_ops.as_ref());
    println!(
        "  ardent.enabled: {}",
        loaded
            .ardent
            .as_ref()
            .and_then(|cfg| cfg.enabled)
            .unwrap_or(false)
    );
    println!(
        "  ardent.default_connector: {}",
        loaded
            .ardent
            .as_ref()
            .and_then(|cfg| cfg.default_connector.as_deref())
            .unwrap_or("<unset>")
    );
    let plan = preflight_plan(provider, backend, false, false);
    if cmd.dry_run {
        println!("dry-run: {}", plan.join(" "));
    } else {
        println!("preflight: {}", plan.join(" "));
    }
    Ok(())
}

async fn run_configure(cmd: DbSandboxConfigureCommand) -> anyhow::Result<()> {
    let thinwedge_home = find_thinwedge_home()?.to_path_buf();
    let mut edits = db_sandbox_edits_from_configure_args(&cmd);
    if edits.is_empty() && !cmd.no_prompt && std::io::stdin().is_terminal() {
        edits = prompt_database_sandbox_config_edits()?;
    }
    if edits.is_empty() {
        println!("No DB sandbox config changes requested.");
        return Ok(());
    }
    if cmd.dry_run {
        println!("Would save {} DB sandbox config value(s).", edits.len());
        return Ok(());
    }
    ConfigEditsBuilder::new(&thinwedge_home)
        .with_edits(edits)
        .apply()
        .await?;
    println!(
        "Saved DB sandbox config to {}",
        thinwedge_home.join("config.toml").display()
    );
    Ok(())
}

async fn run_preflight(
    cli_overrides: Vec<(String, toml::Value)>,
    cmd: DbSandboxPreflightCommand,
) -> anyhow::Result<()> {
    let loaded = load_db_sandbox_config(cli_overrides).await?;
    let provider = cmd.provider.unwrap_or_else(|| {
        DbSandboxProviderArg::from_config(loaded.db_sandbox.as_ref().and_then(|cfg| cfg.provider))
    });
    let backend = DbSandboxBackendArg::from_config(
        loaded
            .db_sandbox
            .as_ref()
            .and_then(|cfg| cfg.branch_backend),
    );
    let plan = preflight_plan(
        provider,
        backend,
        cmd.include_branch_lifecycle,
        cmd.allow_mutation,
    );
    let script = std::path::Path::new(&plan[0]);
    if cmd.dry_run {
        println!("dry-run: {}", plan.join(" "));
        return Ok(());
    }
    if !script.exists() {
        println!(
            "Probe script not found in this working tree. Run from the repository root:\n{}",
            plan.join(" ")
        );
        return Ok(());
    }
    let status = Command::new(&plan[0]).args(&plan[1..]).status()?;
    if !status.success() {
        anyhow::bail!("DB sandbox preflight failed: {}", plan.join(" "));
    }
    Ok(())
}

fn db_sandbox_edits_from_configure_args(cmd: &DbSandboxConfigureCommand) -> Vec<ConfigEdit> {
    let mut updates = DbSandboxConfigUpdates::default();
    if cmd.enable {
        updates.enabled = Some(true);
    }
    if cmd.disable {
        updates.enabled = Some(false);
    }
    updates.provider = cmd
        .provider
        .map(DbSandboxProviderArg::as_config_value)
        .map(str::to_string);
    updates.source_url_env = cmd.source_url_env.clone();
    updates.neon_api_key_env = cmd.neon_api_key_env.clone();
    updates.neon_project_id_env = cmd.neon_project_id_env.clone();
    updates.neon_project_id = cmd.neon_project_id.clone();
    updates.branch_backend = cmd
        .branch_backend
        .map(DbSandboxBackendArg::as_config_value)
        .map(str::to_string);
    updates.into_edits()
}

pub(crate) fn prompt_database_sandbox_config_edits() -> anyhow::Result<Vec<ConfigEdit>> {
    if !std::io::stdin().is_terminal() {
        return Ok(Vec::new());
    }
    let answer = prompt_optional("Configure safe database sandboxes for finance agents? [Y/n]")?;
    if matches!(answer.as_deref(), Some("n") | Some("no")) {
        return Ok(Vec::new());
    }

    let provider = prompt_optional(
        "DB sandbox provider neon/postgresql/rds/supabase/planetscale [default: neon]",
    )?
    .unwrap_or_else(|| DEFAULT_PROVIDER.as_config_value().to_string());
    let provider = match provider.as_str() {
        "neon" | "postgresql" | "rds" | "supabase" | "planetscale" => provider,
        other => anyhow::bail!("unsupported DB sandbox provider `{other}`"),
    };
    let backend = prompt_optional("Branch backend none/ardent [default: none]")?
        .unwrap_or_else(|| DbSandboxBackendArg::None.as_config_value().to_string());
    let backend = match backend.as_str() {
        "none" | "ardent" => backend,
        other => anyhow::bail!("unsupported DB sandbox branch backend `{other}`"),
    };

    let mut updates = DbSandboxConfigUpdates {
        enabled: Some(true),
        provider: Some(provider),
        branch_backend: Some(backend),
        ..Default::default()
    };
    updates.source_url_env = prompt_value(
        "Source Postgres URL env var [default: THINWEDGE_ARDENT_SOURCE_DATABASE_URL]",
    )?;
    updates.neon_api_key_env =
        prompt_value("Neon API key env var [default: THINWEDGE_NEON_API_KEY]")?;
    updates.neon_project_id_env =
        prompt_value("Neon project id env var [default: THINWEDGE_NEON_PROJECT_ID]")?;
    updates.neon_project_id = prompt_value("Neon project id [optional, non-secret]")?;

    Ok(updates.into_edits())
}

fn preflight_plan(
    provider: DbSandboxProviderArg,
    backend: DbSandboxBackendArg,
    include_branch_lifecycle: bool,
    allow_mutation: bool,
) -> Vec<String> {
    let mut plan = vec![
        "scripts/probes/check-db-sandbox-readiness.sh".to_string(),
        "--source-provider".to_string(),
        provider.as_config_value().to_string(),
    ];
    if backend == DbSandboxBackendArg::Ardent {
        plan.push("--branch-backend".to_string());
        plan.push("ardent".to_string());
    } else {
        plan.push("--branch-backend".to_string());
        plan.push("none".to_string());
    }
    if include_branch_lifecycle {
        plan.push("--include-branch-lifecycle".to_string());
    }
    if allow_mutation {
        plan.push("--allow-mutation".to_string());
    }
    plan
}

async fn load_db_sandbox_config(
    cli_overrides: Vec<(String, toml::Value)>,
) -> anyhow::Result<LoadedDbSandboxConfig> {
    let config = Config::load_with_cli_overrides(cli_overrides).await?;
    let config_toml: ConfigToml = config
        .config_layer_stack
        .effective_config()
        .clone()
        .try_into()
        .context("failed to deserialize effective ThinWedge config")?;
    Ok(LoadedDbSandboxConfig {
        billing: config_toml.billing,
        db_ops: config_toml.db_ops,
        db_sandbox: config_toml.db_sandbox,
        ardent: config_toml.ardent,
    })
}

fn print_identity_summary(name: &str, config: Option<&AwsIdentityConfigToml>) {
    match config {
        Some(config) => {
            println!(
                "  {name}.aws_profile: {}",
                config.aws_profile.as_deref().unwrap_or("<unset>")
            );
            println!(
                "  {name}.role_arn: {}",
                config.role_arn.as_deref().unwrap_or("<unset>")
            );
            println!(
                "  {name}.region: {}",
                config.region.as_deref().unwrap_or("<unset>")
            );
        }
        None => println!("  {name}: <unset>"),
    }
}

fn print_env_ref(name: &str, env_name: &str) {
    println!("  {name}: {env_name}");
}

fn prompt_optional(prompt: &str) -> anyhow::Result<Option<String>> {
    let value = prompt_value(prompt)?;
    Ok(value.map(|value| value.to_ascii_lowercase()))
}

fn prompt_value(prompt: &str) -> anyhow::Result<Option<String>> {
    eprint!("{prompt}: ");
    std::io::stderr().flush()?;
    let mut value = String::new();
    std::io::stdin().read_line(&mut value)?;
    let value = value.trim().to_string();
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

fn push_string_edit(edits: &mut Vec<ConfigEdit>, segments: &[&str], val: Option<String>) {
    if let Some(val) = val.filter(|val| !val.trim().is_empty()) {
        push_set_path_edit(edits, segments, value(val));
    }
}

fn push_bool_edit(edits: &mut Vec<ConfigEdit>, segments: &[&str], val: Option<bool>) {
    if let Some(val) = val {
        push_set_path_edit(edits, segments, value(val));
    }
}

fn push_set_path_edit(edits: &mut Vec<ConfigEdit>, segments: &[&str], value: TomlItem) {
    edits.push(ConfigEdit::SetPath {
        segments: segments
            .iter()
            .map(|segment| (*segment).to_string())
            .collect(),
        value,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configure_args_default_to_non_secret_db_sandbox_edits() {
        let cmd = DbSandboxConfigureCommand {
            enable: true,
            provider: Some(DbSandboxProviderArg::Neon),
            neon_api_key_env: Some("THINWEDGE_NEON_API_KEY".to_string()),
            neon_project_id: Some("twilight-lab-63846303".to_string()),
            branch_backend: Some(DbSandboxBackendArg::None),
            ..Default::default()
        };
        let edits = db_sandbox_edits_from_configure_args(&cmd);
        assert_eq!(edits.len(), 5);
    }

    #[test]
    fn preflight_plan_uses_neon_and_no_backend_by_default() {
        let plan = preflight_plan(
            DbSandboxProviderArg::Neon,
            DbSandboxBackendArg::None,
            false,
            false,
        );
        assert_eq!(
            plan,
            vec![
                "scripts/probes/check-db-sandbox-readiness.sh",
                "--source-provider",
                "neon",
                "--branch-backend",
                "none",
            ]
        );
    }

    #[test]
    fn preflight_plan_can_opt_into_ardent_mutations() {
        let plan = preflight_plan(
            DbSandboxProviderArg::Neon,
            DbSandboxBackendArg::Ardent,
            true,
            true,
        );
        assert!(plan.contains(&"--branch-backend".to_string()));
        assert!(plan.contains(&"ardent".to_string()));
        assert!(plan.contains(&"--include-branch-lifecycle".to_string()));
        assert!(plan.contains(&"--allow-mutation".to_string()));
    }
}
