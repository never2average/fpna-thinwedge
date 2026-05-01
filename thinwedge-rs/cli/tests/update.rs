use anyhow::Result;
use predicates::str::contains;
use std::path::Path;
use tempfile::TempDir;

fn thinwedge_command(thinwedge_home: &Path) -> Result<assert_cmd::Command> {
    let mut cmd = assert_cmd::Command::new(thinwedge_utils_cargo_bin::cargo_bin("thinwedge")?);
    cmd.env("THINWEDGE_HOME", thinwedge_home);
    Ok(cmd)
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn update_does_not_start_interactive_prompt() -> Result<()> {
    let thinwedge_home = TempDir::new()?;

    thinwedge_command(thinwedge_home.path())?
        .arg("update")
        .assert()
        .failure()
        .stderr(contains("`thinwedge update` is not available in debug builds"));

    Ok(())
}
