use std::path::Path;

use anyhow::Result;
use predicates::str::contains;
use pretty_assertions::assert_eq;
use serde_json::Value;
use tempfile::TempDir;

fn thinwedge_command(thinwedge_home: &Path) -> Result<assert_cmd::Command> {
    let mut cmd = assert_cmd::Command::new(thinwedge_utils_cargo_bin::cargo_bin("thinwedge")?);
    cmd.env("THINWEDGE_HOME", thinwedge_home);
    Ok(cmd)
}

fn write_file_auth_config(thinwedge_home: &Path) -> Result<()> {
    std::fs::write(
        thinwedge_home.join("config.toml"),
        "cli_auth_credentials_store = \"file\"\n",
    )?;
    Ok(())
}

fn read_auth_json(thinwedge_home: &Path) -> Result<Value> {
    let auth_json = std::fs::read_to_string(thinwedge_home.join("auth.json"))?;
    Ok(serde_json::from_str(&auth_json)?)
}

#[test]
fn login_with_api_key_reads_stdin_and_writes_auth_json() -> Result<()> {
    let thinwedge_home = TempDir::new()?;
    write_file_auth_config(thinwedge_home.path())?;

    let mut cmd = thinwedge_command(thinwedge_home.path())?;
    cmd.args([
        "-c",
        "forced_login_method=\"api\"",
        "login",
        "--with-api-key",
    ])
    .write_stdin("sk-test\n")
    .assert()
    .success()
    .stderr(contains("Successfully logged in"));

    let auth = read_auth_json(thinwedge_home.path())?;
    assert_eq!(auth["THINWEDGE_API_KEY"], "sk-test");
    assert!(auth.get("tokens").is_none());
    assert!(auth.get("agent_identity").is_none());

    Ok(())
}

#[test]
fn login_defaults_to_openrouter_env_key() -> Result<()> {
    let thinwedge_home = TempDir::new()?;
    write_file_auth_config(thinwedge_home.path())?;

    let mut cmd = thinwedge_command(thinwedge_home.path())?;
    cmd.env("OPENROUTER_API_KEY", "sk-openrouter")
        .args(["login"])
        .assert()
        .success()
        .stderr(contains("Reading API key from OPENROUTER_API_KEY"))
        .stderr(contains("Successfully logged in"));

    let auth = read_auth_json(thinwedge_home.path())?;
    assert_eq!(auth["THINWEDGE_API_KEY"], "sk-openrouter");
    assert!(auth.get("tokens").is_none());
    assert!(auth.get("agent_identity").is_none());

    Ok(())
}

#[test]
fn login_without_env_key_prints_api_token_guidance() -> Result<()> {
    let thinwedge_home = TempDir::new()?;
    write_file_auth_config(thinwedge_home.path())?;

    let mut cmd = thinwedge_command(thinwedge_home.path())?;
    cmd.env_remove("OPENROUTER_API_KEY")
        .env_remove("THINWEDGE_API_KEY")
        .args(["login"])
        .assert()
        .failure()
        .stderr(contains(
            "ThinWedge uses OpenRouter-compatible API-token authentication",
        ))
        .stderr(contains("thinwedge login --with-api-key"));

    Ok(())
}

#[test]
fn device_auth_is_disabled_for_thinwedge_login() -> Result<()> {
    let thinwedge_home = TempDir::new()?;
    write_file_auth_config(thinwedge_home.path())?;

    let mut cmd = thinwedge_command(thinwedge_home.path())?;
    cmd.args(["login", "--device-auth"])
        .assert()
        .failure()
        .stderr(contains(
            "Managed browser login is not supported in ThinWedge",
        ));

    Ok(())
}

#[test]
fn login_with_agent_identity_rejects_invalid_jwt() -> Result<()> {
    let thinwedge_home = TempDir::new()?;
    write_file_auth_config(thinwedge_home.path())?;

    let mut cmd = thinwedge_command(thinwedge_home.path())?;
    cmd.args(["login", "--with-agent-identity"])
        .write_stdin("not-a-jwt\n")
        .assert()
        .failure()
        .stderr(contains("Error logging in with Agent Identity"));

    Ok(())
}
