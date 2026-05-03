//! Applies agent-role configuration layers on top of an existing session config.
//!
//! Roles are selected at spawn time and are loaded with the same config machinery as
//! `config.toml`. This module resolves built-in and user-defined role files, inserts the role as a
//! high-precedence layer, and preserves the caller's current profile/provider unless the role
//! explicitly takes ownership of model selection. It does not decide when to spawn a sub-agent or
//! which role to use; the multi-agent tool handler owns that orchestration.

use crate::config::AgentRoleConfig;
use crate::config::Config;
use crate::config::ConfigOverrides;
use crate::config::agent_roles::parse_agent_role_file_contents;
use crate::config::deserialize_config_toml_with_base;
use anyhow::anyhow;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;
use std::sync::LazyLock;
use thinwedge_app_server_protocol::ConfigLayerSource;
use thinwedge_config::ConfigLayerEntry;
use thinwedge_config::ConfigLayerStack;
use thinwedge_config::ConfigLayerStackOrdering;
use thinwedge_config::config_toml::ConfigToml;
use thinwedge_config::loader::resolve_relative_paths_in_config_toml;
use thinwedge_exec_server::LOCAL_FS;
use toml::Value as TomlValue;

/// The role name used when a caller omits `agent_type`.
pub const DEFAULT_ROLE_NAME: &str = "CFO";
const AGENT_TYPE_UNAVAILABLE_ERROR: &str = "agent type is currently not available";

/// Applies a named role layer to `config` while preserving caller-owned model selection.
///
/// The role layer is inserted at session-flag precedence so it can override persisted config, but
/// the caller's current `profile` and `model_provider` remain sticky runtime choices unless the
/// role explicitly sets `profile`, explicitly sets `model_provider`, or rewrites the active
/// profile's `model_provider` in place. Rebuilding the config without those overrides would make a
/// spawned agent silently fall back to the default provider, which is the bug this preservation
/// logic avoids.
pub(crate) async fn apply_role_to_config(
    config: &mut Config,
    role_name: Option<&str>,
) -> Result<(), String> {
    let role_name = role_name.unwrap_or(DEFAULT_ROLE_NAME);

    let role = resolve_role_config(config, role_name)
        .cloned()
        .ok_or_else(|| format!("unknown agent_type '{role_name}'"))?;

    apply_role_to_config_inner(config, role_name, &role)
        .await
        .map_err(|err| {
            tracing::warn!("failed to apply role to config: {err}");
            AGENT_TYPE_UNAVAILABLE_ERROR.to_string()
        })?;
    config.role_visible_skills = role.visible_skills.clone().unwrap_or_default();
    Ok(())
}

async fn apply_role_to_config_inner(
    config: &mut Config,
    role_name: &str,
    role: &AgentRoleConfig,
) -> anyhow::Result<()> {
    let is_built_in = !config.agent_roles.contains_key(role_name);
    let Some(config_file) = role.config_file.as_ref() else {
        return Ok(());
    };
    let role_layer_toml = load_role_layer_toml(config, config_file, is_built_in, role_name).await?;
    if role_layer_toml
        .as_table()
        .is_some_and(toml::map::Map::is_empty)
    {
        return Ok(());
    }
    let (preserve_current_profile, preserve_current_provider) =
        preservation_policy(config, &role_layer_toml);

    *config = reload::build_next_config(
        config,
        role_layer_toml,
        preserve_current_profile,
        preserve_current_provider,
    )
    .await?;
    Ok(())
}

async fn load_role_layer_toml(
    config: &Config,
    config_file: &Path,
    is_built_in: bool,
    role_name: &str,
) -> anyhow::Result<TomlValue> {
    let (role_config_toml, role_config_base) = if is_built_in {
        let role_config_contents = built_in::config_file_contents(config_file)
            .map(str::to_owned)
            .ok_or(anyhow!("No corresponding config content"))?;
        let role_config_toml: TomlValue = toml::from_str(&role_config_contents)?;
        (role_config_toml, config.thinwedge_home.as_path())
    } else {
        let role_config_contents = tokio::fs::read_to_string(config_file).await?;
        let role_config_base = config_file
            .parent()
            .ok_or(anyhow!("No corresponding config content"))?;
        let role_config_toml = parse_agent_role_file_contents(
            &role_config_contents,
            config_file,
            role_config_base,
            Some(role_name),
        )?
        .config;
        (role_config_toml, role_config_base)
    };

    deserialize_config_toml_with_base(role_config_toml.clone(), role_config_base)?;
    Ok(resolve_relative_paths_in_config_toml(
        role_config_toml,
        role_config_base,
    )?)
}

pub(crate) fn resolve_role_config<'a>(
    config: &'a Config,
    role_name: &str,
) -> Option<&'a AgentRoleConfig> {
    config
        .agent_roles
        .get(role_name)
        .or_else(|| built_in::configs().get(role_name))
}

fn preservation_policy(config: &Config, role_layer_toml: &TomlValue) -> (bool, bool) {
    let role_selects_provider = role_layer_toml.get("model_provider").is_some();
    let role_selects_profile = role_layer_toml.get("profile").is_some();
    let role_updates_active_profile_provider = config
        .active_profile
        .as_ref()
        .and_then(|active_profile| {
            role_layer_toml
                .get("profiles")
                .and_then(TomlValue::as_table)
                .and_then(|profiles| profiles.get(active_profile))
                .and_then(TomlValue::as_table)
                .map(|profile| profile.contains_key("model_provider"))
        })
        .unwrap_or(false);
    let preserve_current_profile = !role_selects_provider && !role_selects_profile;
    let preserve_current_provider =
        preserve_current_profile && !role_updates_active_profile_provider;
    (preserve_current_profile, preserve_current_provider)
}

mod reload {
    use super::*;

    pub(super) async fn build_next_config(
        config: &Config,
        role_layer_toml: TomlValue,
        preserve_current_profile: bool,
        preserve_current_provider: bool,
    ) -> anyhow::Result<Config> {
        let active_profile_name = preserve_current_profile
            .then_some(config.active_profile.as_deref())
            .flatten();
        let config_layer_stack =
            build_config_layer_stack(config, &role_layer_toml, active_profile_name)?;
        let mut merged_config = deserialize_effective_config(config, &config_layer_stack)?;
        if preserve_current_profile {
            merged_config.profile = None;
        }

        let mut next_config = Config::load_config_with_layer_stack(
            LOCAL_FS.as_ref(),
            merged_config,
            reload_overrides(config, preserve_current_provider),
            config.thinwedge_home.clone(),
            config_layer_stack,
        )
        .await?;
        if preserve_current_profile {
            next_config.active_profile = config.active_profile.clone();
        }
        Ok(next_config)
    }

    fn build_config_layer_stack(
        config: &Config,
        role_layer_toml: &TomlValue,
        active_profile_name: Option<&str>,
    ) -> anyhow::Result<ConfigLayerStack> {
        let mut layers = existing_layers(config);
        if let Some(resolved_profile_layer) =
            resolved_profile_layer(config, &layers, role_layer_toml, active_profile_name)?
        {
            insert_layer(&mut layers, resolved_profile_layer);
        }
        insert_layer(&mut layers, role_layer(role_layer_toml.clone()));
        Ok(ConfigLayerStack::new(
            layers,
            config.config_layer_stack.requirements().clone(),
            config.config_layer_stack.requirements_toml().clone(),
        )?)
    }

    fn resolved_profile_layer(
        config: &Config,
        existing_layers: &[ConfigLayerEntry],
        role_layer_toml: &TomlValue,
        active_profile_name: Option<&str>,
    ) -> anyhow::Result<Option<ConfigLayerEntry>> {
        let Some(active_profile_name) = active_profile_name else {
            return Ok(None);
        };

        let mut layers = existing_layers.to_vec();
        insert_layer(&mut layers, role_layer(role_layer_toml.clone()));
        let merged_config = deserialize_effective_config(
            config,
            &ConfigLayerStack::new(
                layers,
                config.config_layer_stack.requirements().clone(),
                config.config_layer_stack.requirements_toml().clone(),
            )?,
        )?;
        let resolved_profile =
            merged_config.get_config_profile(Some(active_profile_name.to_string()))?;
        Ok(Some(ConfigLayerEntry::new(
            ConfigLayerSource::SessionFlags,
            TomlValue::try_from(resolved_profile)?,
        )))
    }

    fn deserialize_effective_config(
        config: &Config,
        config_layer_stack: &ConfigLayerStack,
    ) -> anyhow::Result<ConfigToml> {
        Ok(deserialize_config_toml_with_base(
            config_layer_stack.effective_config(),
            &config.thinwedge_home,
        )?)
    }

    fn existing_layers(config: &Config) -> Vec<ConfigLayerEntry> {
        config
            .config_layer_stack
            .get_layers(
                ConfigLayerStackOrdering::LowestPrecedenceFirst,
                /*include_disabled*/ true,
            )
            .into_iter()
            .cloned()
            .collect()
    }

    fn insert_layer(layers: &mut Vec<ConfigLayerEntry>, layer: ConfigLayerEntry) {
        let insertion_index =
            layers.partition_point(|existing_layer| existing_layer.name <= layer.name);
        layers.insert(insertion_index, layer);
    }

    fn role_layer(role_layer_toml: TomlValue) -> ConfigLayerEntry {
        ConfigLayerEntry::new(ConfigLayerSource::SessionFlags, role_layer_toml)
    }

    fn reload_overrides(config: &Config, preserve_current_provider: bool) -> ConfigOverrides {
        ConfigOverrides {
            cwd: Some(config.cwd.to_path_buf()),
            model_provider: preserve_current_provider.then(|| config.model_provider_id.clone()),
            thinwedge_linux_sandbox_exe: config.thinwedge_linux_sandbox_exe.clone(),
            main_execve_wrapper_exe: config.main_execve_wrapper_exe.clone(),
            ..Default::default()
        }
    }
}

pub(crate) mod spawn_tool_spec {
    use super::*;

    /// Builds the spawn-agent tool description text from built-in and configured roles.
    pub(crate) fn build(user_defined_agent_roles: &BTreeMap<String, AgentRoleConfig>) -> String {
        let built_in_roles = built_in::configs();
        build_from_configs(built_in_roles, user_defined_agent_roles)
    }

    // This function is not inlined for testing purpose.
    fn build_from_configs(
        built_in_roles: &BTreeMap<String, AgentRoleConfig>,
        user_defined_roles: &BTreeMap<String, AgentRoleConfig>,
    ) -> String {
        let mut seen = BTreeSet::new();
        let mut formatted_roles = Vec::new();
        for (name, declaration) in user_defined_roles {
            if seen.insert(name.as_str()) {
                formatted_roles.push(format_role(name, declaration));
            }
        }
        for (name, declaration) in built_in_roles {
            if seen.insert(name.as_str()) {
                formatted_roles.push(format_role(name, declaration));
            }
        }

        format!(
            "Optional type name for the new agent. If omitted, `{DEFAULT_ROLE_NAME}` is used.\nAvailable roles:\n{}",
            formatted_roles.join("\n"),
        )
    }

    fn format_role(name: &str, declaration: &AgentRoleConfig) -> String {
        if let Some(description) = &declaration.description {
            let locked_settings_note = declaration
                .config_file
                .as_ref()
                .and_then(|config_file| {
                    built_in::config_file_contents(config_file)
                        .map(str::to_owned)
                        .or_else(|| std::fs::read_to_string(config_file).ok())
                })
                .and_then(|contents| toml::from_str::<TomlValue>(&contents).ok())
                .map(|role_toml| {
                    let model = role_toml
                        .get("model")
                        .and_then(TomlValue::as_str);
                    let reasoning_effort = role_toml
                        .get("model_reasoning_effort")
                        .and_then(TomlValue::as_str);

                    match (model, reasoning_effort) {
                        (Some(model), Some(reasoning_effort)) => format!(
                            "\n- This role's model is set to `{model}` and its reasoning effort is set to `{reasoning_effort}`. These settings cannot be changed."
                        ),
                        (Some(model), None) => {
                            format!(
                                "\n- This role's model is set to `{model}` and cannot be changed."
                            )
                        }
                        (None, Some(reasoning_effort)) => {
                            format!(
                                "\n- This role's reasoning effort is set to `{reasoning_effort}` and cannot be changed."
                            )
                        }
                        (None, None) => String::new(),
                    }
                })
                .unwrap_or_default();
            format!("{name}: {{\n{description}{locked_settings_note}\n}}")
        } else {
            format!("{name}: no description")
        }
    }
}

mod built_in {
    use super::*;

    /// Returns the cached built-in role declarations defined in this module.
    pub(super) fn configs() -> &'static BTreeMap<String, AgentRoleConfig> {
        static CONFIG: LazyLock<BTreeMap<String, AgentRoleConfig>> = LazyLock::new(|| {
            BTreeMap::from([
                (
                    DEFAULT_ROLE_NAME.to_string(),
                    AgentRoleConfig {
                        description: Some(r#"Use `CFO` as the default coordinator role.
Typical tasks:
- Preserve the baseline default-agent behavior: general reasoning, tool use, orchestration, and final synthesis
- Frame the decision, success metric, and time horizon
- Coordinate work across specialized agents
- Synthesize research into a recommendation with explicit tradeoffs
Rules:
- Own the final recommendation, including what to do next and why.
- Delegate pricing strategy, packaging, willingness-to-pay, and unit economics work to `pricing_researcher` when the task becomes monetization-specific.
- Delegate competitive durability, strategic positioning, and moat analysis to `moat_researcher` when the task becomes defensibility-specific.
- Delegate statistical reasoning, experiment interpretation, feature analysis, or model-oriented data exploration to `data-scientist`.
- Delegate metric interpretation, trend explanation, and decision support from existing analytics outputs to `data-analyst`.
- Delegate ML system implementation across training pipelines, feature flow, model serving, or inference integration to `machine-learning-engineer`.
- Delegate technical cost structure, infrastructure economics, and cost-to-serve modeling to `aws_cost_engineer` when the task depends on architecture, cloud, GPU, storage, networking, training, or serving cost details.
- Keep the working plan coherent across delegated work.
- Force every delegated thread to return assumptions, evidence, risks, and a decision-ready conclusion.
- Use `llmcosts.*` for LLM market context and `infracosts.*` for AWS cost context.
- Do not drift into deep specialist work when a narrower role can produce a better answer faster."#.to_string()),
                        config_file: None,
                        nickname_candidates: Some(vec![
                            "Controller".to_string(),
                            "Steward".to_string(),
                            "Northstar".to_string(),
                        ]),
                        visible_skills: Some(vec![
                            "synthesis".to_string(),
                            "finance-decision-framing".to_string(),
                            "evidence-review".to_string(),
                            "risk-review".to_string(),
                        ]),
                    }
                ),
                (
                    "data-scientist".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            "Use when a task needs statistical reasoning, experiment interpretation, feature analysis, or model-oriented data exploration.".to_string(),
                        ),
                        config_file: Some(PathBuf::from("data-scientist.toml")),
                        nickname_candidates: None,
                        visible_skills: None,
                    }
                ),
                (
                    "data-analyst".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            "Use when a task needs data interpretation, metric breakdown, trend explanation, or decision support from existing analytics outputs.".to_string(),
                        ),
                        config_file: Some(PathBuf::from("data-analyst.toml")),
                        nickname_candidates: None,
                        visible_skills: None,
                    }
                ),
                (
                    "machine-learning-engineer".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            "Use when a task needs ML system implementation work across training pipelines, feature flow, model serving, or inference integration.".to_string(),
                        ),
                        config_file: Some(PathBuf::from("machine-learning-engineer.toml")),
                        nickname_candidates: None,
                        visible_skills: None,
                    }
                ),
                (
                    "pricing_researcher".to_string(),
                    AgentRoleConfig {
                        description: Some(r#"Use `pricing_researcher` for pricing analysis and model-driven market research.
Typical tasks:
- Compare pricing strategies, packaging structures, and monetization tradeoffs
- Run or inspect statistical model jobs related to pricing
- Produce a pricing memo with assumptions, evidence, sensitivity, and recommendation
Rules:
- Own willingness-to-pay, packaging, seat/usage economics, and price-performance positioning.
- Use `statisticalmodels.*` for job submission and eval inspection when structured evidence is needed.
- Use `trainingenvironments.*` when the task depends on a role-approved training environment.
- Use `llmcosts.*` for LLM market pricing/speed context from Artificial Analysis.
- Use `infracosts.*` when pricing conclusions depend on AWS infrastructure cost structure.
- Escalate to `aws_cost_engineer` when margin conclusions depend on detailed AWS service assumptions rather than high-level unit economics.
- Return a decision-ready pricing memo, not just notes."#.to_string()),
                        config_file: None,
                        nickname_candidates: Some(vec![
                            "Ratecard".to_string(),
                            "Yield".to_string(),
                            "Tariff".to_string(),
                        ]),
                        visible_skills: Some(vec![
                            "market-research".to_string(),
                            "quant-analysis".to_string(),
                            "pricing-packaging".to_string(),
                            "cohort-analysis".to_string(),
                            "willingness-to-pay".to_string(),
                        ]),
                    }
                ),
                (
                    "moat_researcher".to_string(),
                    AgentRoleConfig {
                        description: Some(r#"Use `moat_researcher` for competitive, strategic, and defensibility analysis.
Typical tasks:
- Analyze differentiation, defensibility, and strategic positioning
- Run or inspect statistical model jobs related to moat research
- Produce a strategy memo covering competitors, switching costs, data/network effects, and structural risk
Rules:
- Own competitive intensity, market structure, imitation risk, and durable advantage.
- Use `statisticalmodels.*` for job submission and eval inspection when structured evidence is needed.
- Use `trainingenvironments.*` when the task depends on a role-approved training environment.
- Use `llmcosts.*` for LLM market context and `infracosts.*` when moat conclusions depend on infrastructure cost structure.
- Call `aws_cost_engineer` when a moat claim depends on a structural cost advantage, infrastructure efficiency, or cloud-economics asymmetry.
- Distinguish clearly between temporary product lead, operational execution, and true structural moat."#.to_string()),
                        config_file: None,
                        nickname_candidates: Some(vec![
                            "Alpha".to_string(),
                            "Premium".to_string(),
                            "Edge".to_string(),
                        ]),
                        visible_skills: Some(vec![
                            "competitive-analysis".to_string(),
                            "market-research".to_string(),
                            "trend-analysis".to_string(),
                            "benchmark-evidence-capture".to_string(),
                        ]),
                    }
                ),
                (
                    "aws_cost_engineer".to_string(),
                    AgentRoleConfig {
                        description: Some(r#"Use `aws_cost_engineer` for AWS BOQs, infrastructure pricing, and service-level cost modeling.
Typical tasks:
- Build or inspect AWS line-item cost assumptions across EC2, storage, networking, and managed services
- Translate product requirements into AWS Price List filters and SKU-level pricing context
- Produce a decision-ready BOQ with cost drivers, tradeoffs, and uncertainty ranges
Rules:
- All other built-in roles may delegate AWS-specific cost structure, infra economics, or billing-detail questions here.
- Use `infracosts.*` as the primary first-party tool namespace for AWS pricing work.
- Use `llmcosts.*` only when the AWS analysis also depends on LLM market pricing context.
- Prefer precise service and filter assumptions over vague blended estimates.
- Make regions, usage assumptions, SKU filters, billing scope, and unresolved pricing gaps explicit in the output.
- Return an assumptions register covering profile, region, billing scope, time window, units, and quantities.
- Return a line-item BOQ or billing summary with source API, selected units, totals, uncertainty, and unresolved gaps.
- Return cost evidence and a decision-ready BOQ, but do not make the final business recommendation."#.to_string()),
                        config_file: None,
                        nickname_candidates: Some(vec![
                            "Basis".to_string(),
                            "Runrate".to_string(),
                            "Variance".to_string(),
                        ]),
                        visible_skills: Some(vec![
                            "cloud-architecture".to_string(),
                            "finops-aws-cost".to_string(),
                            "infrastructure-pricing".to_string(),
                            "terraform-iac-review".to_string(),
                        ]),
                    }
                ),
            ])
        });
        &CONFIG
    }

    /// Resolves a built-in role `config_file` path to embedded content.
    pub(super) fn config_file_contents(path: &Path) -> Option<&'static str> {
        match path.to_str()? {
            "data-scientist.toml" => Some(include_str!("builtins/data-scientist.toml")),
            "data-analyst.toml" => Some(include_str!("builtins/data-analyst.toml")),
            "machine-learning-engineer.toml" => {
                Some(include_str!("builtins/machine-learning-engineer.toml"))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "role_tests.rs"]
mod tests;
