use anyhow::Context;
use anyhow::bail;
use clap::Parser;
use clap::ValueEnum;
use std::env;
use std::io::IsTerminal;
use std::io::Write;
use std::process::Command;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use thinwedge_config::config_toml::ConfigToml;
use thinwedge_config::types::ArdentConfigToml;
use thinwedge_config::types::AwsIdentityConfigToml;
use thinwedge_core::config::Config;
use thinwedge_core::config::edit::ConfigEdit;
use thinwedge_core::config::edit::ConfigEditsBuilder;
use thinwedge_core::config::find_thinwedge_home;
use thinwedge_utils_cli::CliConfigOverrides;
use toml_edit::Item as TomlItem;
use toml_edit::value;

const DEFAULT_ARDENT_CLI: &str = "ardent";
const DEFAULT_SOURCE_URL_ENV: &str = "THINWEDGE_ARDENT_SOURCE_DATABASE_URL";
const DEFAULT_BRANCH_PREFIX: &str = "thinwedge-agent";

#[derive(Debug, Parser)]
#[command(bin_name = "thinwedge ardent")]
pub struct ArdentCli {
    #[clap(skip)]
    pub config_overrides: CliConfigOverrides,

    #[command(subcommand)]
    command: ArdentSubcommand,
}

#[derive(Debug, clap::Subcommand)]
enum ArdentSubcommand {
    /// Show ThinWedge and Ardent sandbox readiness.
    Status(ArdentStatusCommand),
    /// Run Ardent CLI login.
    Login(ArdentLoginCommand),
    /// Save non-secret billing, DB Ops, and Ardent config.
    Configure(ArdentConfigureCommand),
    /// Manage Ardent connectors.
    Connector(ArdentConnectorCommand),
    /// Manage isolated Ardent database branches.
    Branch(ArdentBranchCommand),
}

#[derive(Debug, Parser)]
struct ArdentStatusCommand {
    /// Print the planned checks without running external CLIs.
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, Parser)]
struct ArdentLoginCommand {
    /// Print the Ardent login command without running it.
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, Parser, Default)]
struct ArdentConfigureCommand {
    /// Enable the Ardent integration in config.toml.
    #[arg(
        long = "enabled",
        id = "ardent-enabled",
        conflicts_with = "ardent-disabled"
    )]
    enable: bool,

    /// Disable the Ardent integration in config.toml.
    #[arg(
        long = "disabled",
        id = "ardent-disabled",
        conflicts_with = "ardent-enabled"
    )]
    disable: bool,

    /// Do not prompt for missing values.
    #[arg(long)]
    no_prompt: bool,

    /// Show the config edits that would be saved.
    #[arg(long)]
    dry_run: bool,

    /// AWS profile used for Cost Explorer, CUR, Budgets, and account metadata.
    #[arg(long)]
    billing_profile: Option<String>,

    /// AWS region used with the billing identity when needed.
    #[arg(long)]
    billing_region: Option<String>,

    /// AWS profile used for RDS, Secrets Manager, SSM, and connector setup.
    #[arg(long)]
    db_ops_profile: Option<String>,

    /// Optional production DB Ops role ARN for production credential providers.
    #[arg(long)]
    db_ops_role_arn: Option<String>,

    /// AWS region used with the DB Ops identity.
    #[arg(long)]
    db_ops_region: Option<String>,

    /// Ardent CLI executable path. Defaults to `ardent` on PATH.
    #[arg(long)]
    ardent_cli: Option<String>,

    /// Default Ardent connector name or id for branch lifecycle commands.
    #[arg(long)]
    connector: Option<String>,

    /// Prefix for ephemeral database branch names.
    #[arg(long)]
    branch_prefix: Option<String>,

    /// Requested branch TTL in minutes when supported by the Ardent CLI.
    #[arg(long)]
    branch_ttl_minutes: Option<u32>,

    /// Ardent deployment mode.
    #[arg(long, value_enum)]
    data_plane: Option<ArdentDataPlaneArg>,
}

#[derive(Debug, Parser)]
struct ArdentConnectorCommand {
    #[command(subcommand)]
    subcommand: ArdentConnectorSubcommand,
}

#[derive(Debug, clap::Subcommand)]
enum ArdentConnectorSubcommand {
    /// Create an Ardent Postgres connector from a source URL env var.
    Create(ArdentConnectorCreateCommand),
}

#[derive(Debug, Parser)]
struct ArdentConnectorCreateCommand {
    /// Connector name used for display and later branch lifecycle commands.
    #[arg(long)]
    connector: Option<String>,

    /// Env var that contains the production source database URL.
    #[arg(long, default_value = DEFAULT_SOURCE_URL_ENV)]
    source_url_env: String,

    /// Required acknowledgement for connecting a production source DB to Ardent.
    #[arg(long, alias = "yes")]
    allow_mutation: bool,

    /// Print a redacted command plan without reading or sending the source URL.
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, Parser)]
struct ArdentBranchCommand {
    #[command(subcommand)]
    subcommand: ArdentBranchSubcommand,
}

#[derive(Debug, clap::Subcommand)]
enum ArdentBranchSubcommand {
    /// Create an isolated Ardent database branch.
    Create(ArdentBranchCreateCommand),
    /// Delete an isolated Ardent database branch.
    Delete(ArdentBranchDeleteCommand),
}

#[derive(Debug, Parser)]
struct ArdentBranchCreateCommand {
    /// Connector name or id. Defaults to [ardent].default_connector.
    #[arg(long)]
    connector: Option<String>,

    /// Branch name. Defaults to [ardent].branch_name_prefix plus a timestamp.
    #[arg(long)]
    name: Option<String>,

    /// Print DATABASE_URL=<branch-url> after branch creation when discoverable.
    #[arg(long)]
    print_env: bool,

    /// Print the Ardent commands without creating a branch.
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, Parser)]
struct ArdentBranchDeleteCommand {
    /// Branch name to delete.
    name: String,

    /// Connector name or id. Defaults to [ardent].default_connector.
    #[arg(long)]
    connector: Option<String>,

    /// Print the Ardent command without deleting the branch.
    #[arg(long)]
    dry_run: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ArdentDataPlaneArg {
    Managed,
    Byoc,
}

impl ArdentDataPlaneArg {
    fn as_config_value(self) -> &'static str {
        match self {
            Self::Managed => "managed",
            Self::Byoc => "byoc",
        }
    }
}

#[derive(Debug)]
struct LoadedFinanceConfig {
    billing: Option<AwsIdentityConfigToml>,
    db_ops: Option<AwsIdentityConfigToml>,
    ardent: Option<ArdentConfigToml>,
}

#[derive(Debug)]
struct ArdentSettings {
    cli_path: String,
    default_connector: Option<String>,
    branch_prefix: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ArdentCommandPlan {
    program: String,
    args: Vec<String>,
    display_args: Vec<String>,
}

impl ArdentCommandPlan {
    fn new(program: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            program: program.into(),
            display_args: args.clone(),
            args,
        }
    }

    fn with_display_args(mut self, display_args: Vec<String>) -> Self {
        self.display_args = display_args;
        self
    }

    fn display(&self) -> String {
        let mut parts = vec![self.program.clone()];
        parts.extend(self.display_args.clone());
        parts.join(" ")
    }
}

pub async fn run_ardent_cli(cli: ArdentCli) -> anyhow::Result<()> {
    let cli_overrides = cli
        .config_overrides
        .parse_overrides()
        .map_err(anyhow::Error::msg)?;
    match cli.command {
        ArdentSubcommand::Status(cmd) => run_status(cli_overrides, cmd).await,
        ArdentSubcommand::Login(cmd) => run_login(cli_overrides, cmd).await,
        ArdentSubcommand::Configure(cmd) => run_configure(cmd).await,
        ArdentSubcommand::Connector(cmd) => match cmd.subcommand {
            ArdentConnectorSubcommand::Create(create) => {
                run_connector_create(cli_overrides, create).await
            }
        },
        ArdentSubcommand::Branch(cmd) => match cmd.subcommand {
            ArdentBranchSubcommand::Create(create) => {
                run_branch_create(cli_overrides, create).await
            }
            ArdentBranchSubcommand::Delete(delete) => {
                run_branch_delete(cli_overrides, delete).await
            }
        },
    }
}

async fn run_status(
    cli_overrides: Vec<(String, toml::Value)>,
    cmd: ArdentStatusCommand,
) -> anyhow::Result<()> {
    let loaded = load_finance_config(cli_overrides).await?;
    let settings = ArdentSettings::from_config(loaded.ardent.as_ref());
    println!("ThinWedge finance sandbox config:");
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
    println!("  ardent.cli_path: {}", settings.cli_path);
    println!(
        "  ardent.default_connector: {}",
        settings.default_connector.as_deref().unwrap_or("<unset>")
    );

    let plan = ArdentCommandPlan::new(settings.cli_path, vec!["status".to_string()]);
    if cmd.dry_run {
        println!("dry-run: {}", plan.display());
        return Ok(());
    }
    run_plan_inherit(&plan).context("Ardent status check failed")
}

async fn run_login(
    cli_overrides: Vec<(String, toml::Value)>,
    cmd: ArdentLoginCommand,
) -> anyhow::Result<()> {
    let loaded = load_finance_config(cli_overrides).await?;
    let settings = ArdentSettings::from_config(loaded.ardent.as_ref());
    let plan = ArdentCommandPlan::new(settings.cli_path, vec!["login".to_string()]);
    if cmd.dry_run {
        println!("dry-run: {}", plan.display());
        return Ok(());
    }
    run_plan_inherit(&plan).context("Ardent login failed")
}

async fn run_configure(cmd: ArdentConfigureCommand) -> anyhow::Result<()> {
    let thinwedge_home = find_thinwedge_home()?.to_path_buf();
    let mut edits = config_edits_from_configure_args(&cmd);

    if edits.is_empty() && !cmd.no_prompt && std::io::stdin().is_terminal() {
        edits = prompt_database_sandbox_config_edits()?;
    }

    if edits.is_empty() {
        println!("No database sandbox config changes requested.");
        return Ok(());
    }

    if cmd.dry_run {
        println!(
            "Would save {} database sandbox config value(s).",
            edits.len()
        );
        return Ok(());
    }

    ConfigEditsBuilder::new(&thinwedge_home)
        .with_edits(edits)
        .apply()
        .await?;
    println!(
        "Saved database sandbox config to {}",
        thinwedge_home.join("config.toml").display()
    );
    Ok(())
}

async fn run_connector_create(
    cli_overrides: Vec<(String, toml::Value)>,
    cmd: ArdentConnectorCreateCommand,
) -> anyhow::Result<()> {
    let loaded = load_finance_config(cli_overrides).await?;
    let settings = ArdentSettings::from_config(loaded.ardent.as_ref());
    let connector = cmd
        .connector
        .or(settings.default_connector)
        .unwrap_or_else(|| "postgres".to_string());

    if cmd.dry_run {
        let plan = connector_create_plan(&settings.cli_path, "<redacted-source-url>");
        println!("dry-run: {}", plan.display());
        println!("connector: {connector}");
        return Ok(());
    }

    if !cmd.allow_mutation {
        bail!(
            "creating an Ardent connector attaches a production source DB; rerun with --allow-mutation after explicit approval"
        );
    }

    let source_url = env::var(&cmd.source_url_env).with_context(|| {
        format!(
            "{} must contain the source database URL",
            cmd.source_url_env
        )
    })?;
    let plan = connector_create_plan(&settings.cli_path, &source_url);
    eprintln!("Creating Ardent Postgres connector without printing source URL.");
    run_plan_inherit(&plan).context("Ardent connector creation failed")
}

async fn run_branch_create(
    cli_overrides: Vec<(String, toml::Value)>,
    cmd: ArdentBranchCreateCommand,
) -> anyhow::Result<()> {
    let loaded = load_finance_config(cli_overrides).await?;
    let settings = ArdentSettings::from_config(loaded.ardent.as_ref());
    let connector = cmd.connector.or(settings.default_connector);
    let branch = cmd
        .name
        .unwrap_or_else(|| default_branch_name(&settings.branch_prefix));
    let create_plan = branch_create_plan(&settings.cli_path, &branch, connector.as_deref());

    if cmd.dry_run {
        println!("dry-run: {}", create_plan.display());
        if cmd.print_env {
            let info_plan = branch_info_plan(&settings.cli_path, &branch, connector.as_deref());
            println!("dry-run: {}", info_plan.display());
        }
        return Ok(());
    }

    run_plan_inherit(&create_plan).context("Ardent branch creation failed")?;
    println!("Created Ardent branch: {branch}");

    if cmd.print_env {
        let info_plan = branch_info_plan(&settings.cli_path, &branch, connector.as_deref());
        let output = run_plan_capture(&info_plan).context("failed to inspect Ardent branch")?;
        match extract_database_url(&output) {
            Some(database_url) => println!("DATABASE_URL={database_url}"),
            None => bail!("Ardent branch info did not include a DATABASE_URL or Postgres URL"),
        }
    }

    Ok(())
}

async fn run_branch_delete(
    cli_overrides: Vec<(String, toml::Value)>,
    cmd: ArdentBranchDeleteCommand,
) -> anyhow::Result<()> {
    let loaded = load_finance_config(cli_overrides).await?;
    let settings = ArdentSettings::from_config(loaded.ardent.as_ref());
    let connector = cmd.connector.or(settings.default_connector);
    let plan = branch_delete_plan(&settings.cli_path, &cmd.name, connector.as_deref());

    if cmd.dry_run {
        println!("dry-run: {}", plan.display());
        return Ok(());
    }

    run_plan_inherit(&plan).context("Ardent branch deletion failed")
}

async fn load_finance_config(
    cli_overrides: Vec<(String, toml::Value)>,
) -> anyhow::Result<LoadedFinanceConfig> {
    let config = Config::load_with_cli_overrides(cli_overrides).await?;
    let config_toml: ConfigToml = config
        .config_layer_stack
        .effective_config()
        .clone()
        .try_into()
        .context("failed to deserialize effective ThinWedge config")?;
    Ok(LoadedFinanceConfig {
        billing: config_toml.billing,
        db_ops: config_toml.db_ops,
        ardent: config_toml.ardent,
    })
}

impl ArdentSettings {
    fn from_config(config: Option<&ArdentConfigToml>) -> Self {
        Self {
            cli_path: config
                .and_then(|cfg| cfg.cli_path.clone())
                .unwrap_or_else(|| DEFAULT_ARDENT_CLI.to_string()),
            default_connector: config.and_then(|cfg| cfg.default_connector.clone()),
            branch_prefix: config
                .and_then(|cfg| cfg.branch_name_prefix.clone())
                .unwrap_or_else(|| DEFAULT_BRANCH_PREFIX.to_string()),
        }
    }
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

fn config_edits_from_configure_args(cmd: &ArdentConfigureCommand) -> Vec<ConfigEdit> {
    let mut updates = DatabaseSandboxConfigUpdates::default();
    if cmd.enable {
        updates.ardent_enabled = Some(true);
    }
    if cmd.disable {
        updates.ardent_enabled = Some(false);
    }
    updates.billing_profile = cmd.billing_profile.clone();
    updates.billing_region = cmd.billing_region.clone();
    updates.db_ops_profile = cmd.db_ops_profile.clone();
    updates.db_ops_role_arn = cmd.db_ops_role_arn.clone();
    updates.db_ops_region = cmd.db_ops_region.clone();
    updates.ardent_cli = cmd.ardent_cli.clone();
    updates.connector = cmd.connector.clone();
    updates.branch_prefix = cmd.branch_prefix.clone();
    updates.branch_ttl_minutes = cmd.branch_ttl_minutes;
    updates.data_plane = cmd
        .data_plane
        .map(ArdentDataPlaneArg::as_config_value)
        .map(str::to_string);
    updates.into_edits()
}

#[derive(Default)]
struct DatabaseSandboxConfigUpdates {
    ardent_enabled: Option<bool>,
    billing_profile: Option<String>,
    billing_region: Option<String>,
    db_ops_profile: Option<String>,
    db_ops_role_arn: Option<String>,
    db_ops_region: Option<String>,
    ardent_cli: Option<String>,
    connector: Option<String>,
    branch_prefix: Option<String>,
    branch_ttl_minutes: Option<u32>,
    data_plane: Option<String>,
}

impl DatabaseSandboxConfigUpdates {
    fn into_edits(self) -> Vec<ConfigEdit> {
        let mut edits = Vec::new();
        push_bool_edit(&mut edits, &["ardent", "enabled"], self.ardent_enabled);
        push_string_edit(
            &mut edits,
            &["billing", "aws_profile"],
            self.billing_profile,
        );
        push_string_edit(&mut edits, &["billing", "region"], self.billing_region);
        push_string_edit(&mut edits, &["db_ops", "aws_profile"], self.db_ops_profile);
        push_string_edit(&mut edits, &["db_ops", "role_arn"], self.db_ops_role_arn);
        push_string_edit(&mut edits, &["db_ops", "region"], self.db_ops_region);
        push_string_edit(&mut edits, &["ardent", "cli_path"], self.ardent_cli);
        push_string_edit(&mut edits, &["ardent", "default_connector"], self.connector);
        push_string_edit(
            &mut edits,
            &["ardent", "branch_name_prefix"],
            self.branch_prefix,
        );
        push_u32_edit(
            &mut edits,
            &["ardent", "branch_ttl_minutes"],
            self.branch_ttl_minutes,
        );
        push_string_edit(&mut edits, &["ardent", "data_plane"], self.data_plane);
        edits
    }
}

pub(crate) fn prompt_database_sandbox_config_edits() -> anyhow::Result<Vec<ConfigEdit>> {
    if !std::io::stdin().is_terminal() {
        return Ok(Vec::new());
    }

    let answer = prompt_optional("Configure safe database sandboxes for finance agents? [Y/n]")?;
    if matches!(answer.as_deref(), Some("n") | Some("no")) {
        return Ok(Vec::new());
    }

    let mut updates = DatabaseSandboxConfigUpdates {
        ardent_enabled: Some(true),
        ..Default::default()
    };
    updates.billing_profile = prompt_value("Billing AWS profile [optional]")?;
    updates.billing_region = prompt_value("Billing AWS region [optional]")?;
    updates.db_ops_profile = prompt_value("DB Ops AWS profile [optional]")?;
    updates.db_ops_role_arn = prompt_value("DB Ops role ARN for production [optional]")?;
    updates.db_ops_region = prompt_value("DB Ops AWS region [optional]")?;
    updates.ardent_cli = prompt_value("Ardent CLI path [default: ardent]")?;
    updates.connector = prompt_value("Default Ardent connector name/id [optional]")?;
    updates.branch_prefix = prompt_value("Ardent branch name prefix [default: thinwedge-agent]")?;
    updates.branch_ttl_minutes = prompt_value("Ardent branch TTL minutes [optional]")?
        .map(|raw| {
            raw.parse::<u32>()
                .context("branch TTL must be a positive integer")
        })
        .transpose()?;
    updates.data_plane = prompt_data_plane()?;

    Ok(updates.into_edits())
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

fn prompt_data_plane() -> anyhow::Result<Option<String>> {
    match prompt_optional("Ardent data plane managed/byoc [optional]")?.as_deref() {
        Some("managed") => Ok(Some("managed".to_string())),
        Some("byoc") => Ok(Some("byoc".to_string())),
        Some(other) => bail!("unsupported Ardent data plane `{other}`; expected managed or byoc"),
        None => Ok(None),
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

fn push_u32_edit(edits: &mut Vec<ConfigEdit>, segments: &[&str], val: Option<u32>) {
    if let Some(val) = val {
        push_set_path_edit(edits, segments, value(i64::from(val)));
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

fn connector_create_plan(cli_path: &str, source_url: &str) -> ArdentCommandPlan {
    ArdentCommandPlan::new(
        cli_path,
        vec![
            "connector".to_string(),
            "create".to_string(),
            "postgresql".to_string(),
            source_url.to_string(),
        ],
    )
    .with_display_args(vec![
        "connector".to_string(),
        "create".to_string(),
        "postgresql".to_string(),
        "<redacted-source-url>".to_string(),
    ])
}

fn branch_create_plan(cli_path: &str, branch: &str, connector: Option<&str>) -> ArdentCommandPlan {
    let mut args = vec![
        "branch".to_string(),
        "create".to_string(),
        branch.to_string(),
    ];
    push_connector_args(&mut args, connector);
    ArdentCommandPlan::new(cli_path, args)
}

fn branch_info_plan(cli_path: &str, branch: &str, connector: Option<&str>) -> ArdentCommandPlan {
    let mut args = vec!["branch".to_string(), "info".to_string(), branch.to_string()];
    push_connector_args(&mut args, connector);
    ArdentCommandPlan::new(cli_path, args)
}

fn branch_delete_plan(cli_path: &str, branch: &str, connector: Option<&str>) -> ArdentCommandPlan {
    let mut args = vec![
        "branch".to_string(),
        "delete".to_string(),
        branch.to_string(),
    ];
    push_connector_args(&mut args, connector);
    ArdentCommandPlan::new(cli_path, args)
}

fn push_connector_args(args: &mut Vec<String>, connector: Option<&str>) {
    if let Some(connector) = connector.filter(|connector| !connector.is_empty()) {
        args.push("--connector".to_string());
        args.push(connector.to_string());
    }
}

fn default_branch_name(prefix: &str) -> String {
    let unix_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{prefix}-{unix_seconds}")
}

fn run_plan_inherit(plan: &ArdentCommandPlan) -> anyhow::Result<()> {
    let status = Command::new(&plan.program).args(&plan.args).status()?;
    if !status.success() {
        bail!("command failed: {}", plan.display());
    }
    Ok(())
}

fn run_plan_capture(plan: &ArdentCommandPlan) -> anyhow::Result<String> {
    let output = Command::new(&plan.program).args(&plan.args).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("command failed: {}\n{}", plan.display(), stderr.trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn extract_database_url(output: &str) -> Option<String> {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
        if let Some(url) = extract_database_url_from_json(&json) {
            return Some(url);
        }
    }

    for line in output.lines() {
        if let Some(value) = line.trim().strip_prefix("DATABASE_URL=") {
            return Some(trim_url_token(value));
        }
    }

    output
        .split_whitespace()
        .find(|token| token.starts_with("postgres://") || token.starts_with("postgresql://"))
        .map(trim_url_token)
}

fn extract_database_url_from_json(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(value) => {
            if value.starts_with("postgres://") || value.starts_with("postgresql://") {
                Some(value.clone())
            } else {
                None
            }
        }
        serde_json::Value::Array(values) => values.iter().find_map(extract_database_url_from_json),
        serde_json::Value::Object(map) => {
            for key in ["DATABASE_URL", "database_url", "connection_string", "url"] {
                if let Some(url) = map.get(key).and_then(extract_database_url_from_json) {
                    return Some(url);
                }
            }
            map.values().find_map(extract_database_url_from_json)
        }
        _ => None,
    }
}

fn trim_url_token(value: &str) -> String {
    value
        .trim()
        .trim_matches(|ch| matches!(ch, '"' | '\'' | ',' | ';'))
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connector_create_plan_redacts_source_url_in_display() {
        let source_url = "postgres://source-user:secret@example.com/prod";
        let plan = connector_create_plan("ardent", source_url);
        assert!(plan.args.contains(&source_url.to_string()));
        assert!(!plan.display().contains(source_url));
        assert!(plan.display().contains("<redacted-source-url>"));
    }

    #[test]
    fn branch_create_plan_includes_connector_when_set() {
        let plan = branch_create_plan("ardent", "thinwedge-agent-1", Some("fpna-prod"));
        assert_eq!(
            plan.args,
            vec![
                "branch",
                "create",
                "thinwedge-agent-1",
                "--connector",
                "fpna-prod"
            ]
        );
    }

    #[test]
    fn extract_database_url_reads_shell_export_output() {
        let output = "branch ready\nDATABASE_URL=postgresql://agent:token@example.com/branch\n";
        assert_eq!(
            extract_database_url(output).as_deref(),
            Some("postgresql://agent:token@example.com/branch")
        );
    }

    #[test]
    fn extract_database_url_reads_json_output() {
        let output = r#"{"branch":{"database_url":"postgres://agent:token@example.com/branch"}}"#;
        assert_eq!(
            extract_database_url(output).as_deref(),
            Some("postgres://agent:token@example.com/branch")
        );
    }

    #[test]
    fn configure_args_map_to_non_secret_config_edits() {
        let cmd = ArdentConfigureCommand {
            enable: true,
            billing_profile: Some("fpna-billing".to_string()),
            db_ops_profile: Some("fpna-db-ops".to_string()),
            connector: Some("fpna-prod".to_string()),
            data_plane: Some(ArdentDataPlaneArg::Byoc),
            ..Default::default()
        };
        let edits = config_edits_from_configure_args(&cmd);
        assert_eq!(edits.len(), 5);
    }
}
