use super::storage::runtime_contexts_dir;
use super::storage::write_json_pretty;
use super::types::ExecutableActionBinding;
use super::types::InferenceTarget;
use super::types::ModelRepository;
use super::types::StoredExecutionResult;
use chrono::Utc;
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::path::Path;
use std::path::PathBuf;
use tokio::process::Command;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ExecutionContext {
    pub(super) action: String,
    pub(super) agent_role: String,
    #[serde(default)]
    pub(super) inference: Option<InferenceTarget>,
    #[serde(default)]
    pub(super) repository: Option<ModelRepository>,
    #[serde(default)]
    pub(super) model: Option<JsonValue>,
    #[serde(default)]
    pub(super) environment: Option<JsonValue>,
    #[serde(default)]
    pub(super) job: Option<JsonValue>,
    #[serde(default)]
    pub(super) payload: Option<JsonValue>,
}

pub(super) async fn execute_action(
    thinwedge_home: &Path,
    binding: &ExecutableActionBinding,
    fallback_working_directory: Option<&Path>,
    context: &ExecutionContext,
) -> Result<StoredExecutionResult, String> {
    if !binding.enabled {
        return Err(format!("ThinWedge action `{}` is disabled", context.action));
    }

    let context_path = runtime_contexts_dir(thinwedge_home).join(format!(
        "{}-{}.json",
        sanitize_file_segment(&context.action),
        Uuid::new_v4()
    ));
    write_json_pretty(&context_path, context).await?;

    let working_directory = binding
        .working_directory
        .clone()
        .or_else(|| fallback_working_directory.map(Path::to_path_buf));
    let shell = binding
        .shell
        .clone()
        .unwrap_or_else(|| "/bin/sh".to_string());

    let mut command = Command::new(&shell);
    command.arg("-lc").arg(&binding.command);
    if let Some(working_directory) = working_directory.as_ref() {
        command.current_dir(working_directory);
    }
    for (key, value) in &binding.environment {
        command.env(key, value);
    }
    command.env("THINWEDGE_THINWEDGE_HOME", thinwedge_home);
    command.env("THINWEDGE_CONTEXT_JSON", &context_path);
    command.env("THINWEDGE_ACTION", &context.action);
    command.env("THINWEDGE_AGENT_ROLE", &context.agent_role);
    if let Some(inference) = context.inference.as_ref() {
        command.env("THINWEDGE_INFERENCE_PROVIDER", &inference.provider_id);
        command.env("THINWEDGE_INFERENCE_MODEL", &inference.model_name);
        if let Some(base_url) = inference.base_url.as_ref() {
            command.env("THINWEDGE_INFERENCE_BASE_URL", base_url);
        }
        if let Some(api_key_env) = inference.api_key_env.as_ref() {
            command.env("THINWEDGE_INFERENCE_API_KEY_ENV", api_key_env);
            if let Ok(api_key) = std::env::var(api_key_env) {
                command.env(api_key_env, api_key);
            }
        }
        if let Some(wire_api) = inference.wire_api.as_ref() {
            command.env("THINWEDGE_INFERENCE_WIRE_API", wire_api);
        }
    }
    if let Some(repository) = context.repository.as_ref() {
        command.env("THINWEDGE_MODEL_REPOSITORY_ROOT", &repository.root);
        if let Some(config_path) = repository.config_path.as_ref() {
            command.env("THINWEDGE_MODEL_REPOSITORY_CONFIG", config_path);
        }
        if let Some(ref_name) = repository.ref_name.as_ref() {
            command.env("THINWEDGE_MODEL_REPOSITORY_REF", ref_name);
        }
        if let Some(entrypoint) = repository.entrypoint.as_ref() {
            command.env("THINWEDGE_MODEL_REPOSITORY_ENTRYPOINT", entrypoint);
        }
        if let Some(batch_entrypoint) = repository.batch_entrypoint.as_ref() {
            command.env(
                "THINWEDGE_MODEL_REPOSITORY_BATCH_ENTRYPOINT",
                batch_entrypoint,
            );
        }
    }
    if let Some(payload) = context.payload.as_ref() {
        command.env("THINWEDGE_PAYLOAD_JSON", payload.to_string());
    }
    if let Some(job) = context.job.as_ref() {
        if let Some(job_id) = job.get("id").and_then(JsonValue::as_str) {
            command.env("THINWEDGE_JOB_ID", job_id);
        }
        if let Some(job_type) = job.get("type").and_then(JsonValue::as_str) {
            command.env("THINWEDGE_JOB_TYPE", job_type);
        }
    }
    if let Some(model) = context.model.as_ref()
        && let Some(model_id) = model.get("id").and_then(JsonValue::as_str)
    {
        command.env("THINWEDGE_MODEL_ID", model_id);
    }
    if let Some(environment) = context.environment.as_ref()
        && let Some(environment_id) = environment.get("id").and_then(JsonValue::as_str)
    {
        command.env("THINWEDGE_ENVIRONMENT_ID", environment_id);
    }

    let output = command.output().await.map_err(|err| {
        format!(
            "failed to execute ThinWedge action `{}`: {err}",
            context.action
        )
    })?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Ok(StoredExecutionResult {
        action: context.action.clone(),
        command: binding.command.clone(),
        shell: Some(shell),
        working_directory,
        exit_code: output.status.code(),
        success: output.status.success(),
        summary_json: serde_json::from_str::<JsonValue>(&stdout).ok(),
        stdout,
        stderr,
        ran_at: Utc::now().timestamp(),
        context_path,
    })
}

fn sanitize_file_segment(segment: &str) -> String {
    segment
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}
