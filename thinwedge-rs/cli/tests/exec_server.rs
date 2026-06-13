use std::path::Path;

use anyhow::Result;
use predicates::str::contains;
use tempfile::TempDir;

fn thinwedge_command(thinwedge_home: &Path) -> Result<assert_cmd::Command> {
    let mut cmd = assert_cmd::Command::new(thinwedge_utils_cargo_bin::cargo_bin("thinwedge")?);
    cmd.env("THINWEDGE_HOME", thinwedge_home);
    Ok(cmd)
}

#[test]
fn strict_config_rejects_unknown_config_fields_for_exec_server() -> Result<()> {
    let thinwedge_home = TempDir::new()?;
    std::fs::write(
        thinwedge_home.path().join("config.toml"),
        r#"
foo = "bar"
"#,
    )?;

    let mut cmd = thinwedge_command(thinwedge_home.path())?;
    cmd.args([
        "exec-server",
        "--strict-config",
        "--listen",
        "http://127.0.0.1:0",
    ])
    .assert()
    .failure()
    .stderr(contains("unknown configuration field"));

    Ok(())
}
