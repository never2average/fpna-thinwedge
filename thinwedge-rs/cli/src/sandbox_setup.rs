use std::path::PathBuf;

use clap::ArgAction;
use clap::ArgGroup;
use clap::Parser;
use thinwedge_core::config::edit::ConfigEditsBuilder;
use thinwedge_core::config::find_thinwedge_home;

#[derive(Debug, Parser)]
#[command(group(
    ArgGroup::new("sandbox_user")
        .required(true)
        .args(["user", "current_user"])
))]
pub(crate) struct SandboxSetupCommand {
    /// Set up the elevated Windows sandbox.
    #[arg(long = "elevated", action = ArgAction::SetTrue)]
    elevated_sandbox_level: bool,

    /// Windows user that will run ThinWedge after managed deployment.
    #[arg(
        long = "user",
        value_name = "USER",
        conflicts_with = "current_user",
        requires = "thinwedge_home"
    )]
    user: Option<String>,

    /// Use the current Windows user as the ThinWedge user.
    #[arg(
        long = "current-user",
        default_value_t = false,
        conflicts_with = "user"
    )]
    current_user: bool,

    /// THINWEDGE_HOME for the ThinWedge user. Required with --user.
    #[arg(long = "thinwedge-home", value_name = "DIR")]
    thinwedge_home: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SandboxSetupLevel {
    Elevated,
}

impl SandboxSetupCommand {
    fn setup_level(&self) -> anyhow::Result<SandboxSetupLevel> {
        if self.elevated_sandbox_level {
            Ok(SandboxSetupLevel::Elevated)
        } else {
            anyhow::bail!("`thinwedge sandbox setup` currently requires --elevated");
        }
    }
}

pub(crate) async fn run(cmd: SandboxSetupCommand) -> anyhow::Result<()> {
    match cmd.setup_level()? {
        SandboxSetupLevel::Elevated => run_elevated(cmd).await,
    }
}

pub(crate) fn parse_setup_command(
    sandbox_command: &[String],
) -> anyhow::Result<Option<SandboxSetupCommand>> {
    if sandbox_command
        .first()
        .is_none_or(|command| command != "setup")
    {
        return Ok(None);
    }

    SandboxSetupCommand::try_parse_from(sandbox_command.iter().map(String::as_str))
        .map(Some)
        .map_err(anyhow::Error::from)
}

async fn run_elevated(cmd: SandboxSetupCommand) -> anyhow::Result<()> {
    let identity = resolve_sandbox_setup_identity(&cmd)?;

    thinwedge_core::windows_sandbox::run_elevated_provisioning_setup(
        identity.thinwedge_home.as_path(),
        identity.real_user.as_str(),
    )?;
    ConfigEditsBuilder::new(identity.thinwedge_home.as_path())
        .set_windows_sandbox_mode("elevated")
        .apply()
        .await
        .map_err(|err| {
            anyhow::anyhow!(
                "sandbox provisioning succeeded, but failed to persist elevated sandbox config: {err}"
            )
        })?;

    println!(
        "Windows elevated sandbox setup completed for {} at {}.",
        identity.real_user,
        identity.thinwedge_home.display()
    );
    Ok(())
}

struct SandboxSetupIdentity {
    real_user: String,
    thinwedge_home: PathBuf,
}

fn resolve_sandbox_setup_identity(
    cmd: &SandboxSetupCommand,
) -> anyhow::Result<SandboxSetupIdentity> {
    if cmd.current_user {
        let real_user = std::env::var("USERNAME")
            .or_else(|_| std::env::var("USER"))
            .map_err(|err| {
                anyhow::anyhow!("failed to determine current user from environment: {err}")
            })?;
        let thinwedge_home = match cmd.thinwedge_home.clone() {
            Some(thinwedge_home) => thinwedge_home,
            None => find_thinwedge_home()?.to_path_buf(),
        };
        return Ok(SandboxSetupIdentity {
            real_user,
            thinwedge_home,
        });
    }

    let real_user = cmd
        .user
        .clone()
        .ok_or_else(|| anyhow::anyhow!("--user or --current-user is required"))?;
    let thinwedge_home = cmd
        .thinwedge_home
        .clone()
        .ok_or_else(|| anyhow::anyhow!("--thinwedge-home is required with --user"))?;
    Ok(SandboxSetupIdentity {
        real_user,
        thinwedge_home,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_managed_user_identity() {
        let command = SandboxSetupCommand::try_parse_from([
            "setup",
            "--elevated",
            "--user",
            "DOMAIN\\alice",
            "--thinwedge-home",
            r"C:\Users\alice\.thinwedge",
        ])
        .expect("parse");

        assert!(command.elevated_sandbox_level);
        assert_eq!(command.user.as_deref(), Some(r"DOMAIN\alice"));
        assert!(!command.current_user);
        assert_eq!(
            command.thinwedge_home.as_deref(),
            Some(std::path::Path::new(r"C:\Users\alice\.thinwedge"))
        );
    }

    #[test]
    fn requires_explicit_user_identity() {
        let err = SandboxSetupCommand::try_parse_from(["setup", "--elevated"])
            .expect_err("parse should fail");

        assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn requires_thinwedge_home_for_managed_user() {
        let err =
            SandboxSetupCommand::try_parse_from(["setup", "--elevated", "--user", "DOMAIN\\alice"])
                .expect_err("parse should fail");

        assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn parses_setup_from_sandbox_command_args() {
        let command = parse_setup_command(&[
            "setup".to_string(),
            "--elevated".to_string(),
            "--user".to_string(),
            r"DOMAIN\alice".to_string(),
            "--thinwedge-home".to_string(),
            r"C:\Users\alice\.thinwedge".to_string(),
        ])
        .expect("parse")
        .expect("setup command");

        assert_eq!(command.user.as_deref(), Some(r"DOMAIN\alice"));
    }

    #[test]
    fn ignores_non_setup_sandbox_command_args() {
        let command =
            parse_setup_command(&["echo".to_string(), "hello".to_string()]).expect("parse");

        assert!(command.is_none());
    }
}
