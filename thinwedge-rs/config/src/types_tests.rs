use super::*;
use pretty_assertions::assert_eq;

#[test]
fn deserialize_skill_config_with_name_selector() {
    let cfg: SkillConfig = toml::from_str(
        r#"
            name = "github:yeet"
            enabled = false
        "#,
    )
    .expect("should deserialize skill config with name selector");

    assert_eq!(cfg.name.as_deref(), Some("github:yeet"));
    assert_eq!(cfg.path, None);
    assert!(!cfg.enabled);
}

#[test]
fn deserialize_skill_config_with_path_selector() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let skill_path = tempdir.path().join("skills").join("demo").join("SKILL.md");
    let cfg: SkillConfig = toml::from_str(&format!(
        r#"
            path = {path:?}
            enabled = false
        "#,
        path = skill_path.display().to_string(),
    ))
    .expect("should deserialize skill config with path selector");

    assert_eq!(
        cfg,
        SkillConfig {
            path: Some(
                AbsolutePathBuf::from_absolute_path(&skill_path)
                    .expect("skill path should be absolute"),
            ),
            name: None,
            enabled: false,
        }
    );
}

#[test]
fn memories_config_clamps_count_limits_to_nonzero_values() {
    let config = MemoriesConfig::from(MemoriesToml {
        max_raw_memories_for_consolidation: Some(0),
        max_rollouts_per_startup: Some(0),
        ..Default::default()
    });

    assert_eq!(
        config,
        MemoriesConfig {
            max_raw_memories_for_consolidation: 1,
            max_rollouts_per_startup: 1,
            ..MemoriesConfig::default()
        }
    );
}

#[test]
fn memories_config_clamps_rate_limit_remaining_threshold() {
    let config = MemoriesConfig::from(MemoriesToml {
        min_rate_limit_remaining_percent: Some(101),
        ..Default::default()
    });
    assert_eq!(
        config,
        MemoriesConfig {
            min_rate_limit_remaining_percent: 100,
            ..MemoriesConfig::default()
        }
    );

    let config = MemoriesConfig::from(MemoriesToml {
        min_rate_limit_remaining_percent: Some(-1),
        ..Default::default()
    });
    assert_eq!(
        config,
        MemoriesConfig {
            min_rate_limit_remaining_percent: 0,
            ..MemoriesConfig::default()
        }
    );
}

#[test]
fn deserialize_finance_sandbox_config() {
    let cfg: crate::config_toml::ConfigToml = toml::from_str(
        r#"
            [billing]
            aws_profile = "fpna-billing"
            role_arn = "arn:aws:iam::123456789012:role/fpna-billing"
            region = "us-east-1"

            [db_ops]
            aws_profile = "fpna-db-ops"
            role_arn = "arn:aws:iam::123456789012:role/fpna-db-ops"
            region = "us-west-2"

            [ardent]
            enabled = true
            cli_path = "ardent"
            default_connector = "fpna-prod"
            branch_name_prefix = "thinwedge-agent"
            branch_ttl_minutes = 60
            data_plane = "byoc"
        "#,
    )
    .expect("finance sandbox config should deserialize");

    let billing = cfg.billing.expect("billing config");
    assert_eq!(billing.aws_profile.as_deref(), Some("fpna-billing"));
    assert_eq!(
        billing.role_arn.as_deref(),
        Some("arn:aws:iam::123456789012:role/fpna-billing")
    );
    assert_eq!(billing.region.as_deref(), Some("us-east-1"));

    let db_ops = cfg.db_ops.expect("db ops config");
    assert_eq!(db_ops.aws_profile.as_deref(), Some("fpna-db-ops"));
    assert_eq!(
        db_ops.role_arn.as_deref(),
        Some("arn:aws:iam::123456789012:role/fpna-db-ops")
    );

    let ardent = cfg.ardent.expect("ardent config");
    assert_eq!(ardent.enabled, Some(true));
    assert_eq!(ardent.default_connector.as_deref(), Some("fpna-prod"));
    assert_eq!(
        ardent.branch_name_prefix.as_deref(),
        Some("thinwedge-agent")
    );
    assert_eq!(ardent.branch_ttl_minutes, Some(60));
    assert_eq!(ardent.data_plane, Some(ArdentDataPlaneToml::Byoc));
}
