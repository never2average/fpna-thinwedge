use serde::Deserialize;
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(super) struct StatisticalModelsRegistry {
    #[serde(default)]
    pub(super) models: Vec<StatisticalModelRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct StatisticalModelRecord {
    pub(super) id: String,
    #[serde(default)]
    pub(super) name: Option<String>,
    #[serde(default)]
    pub(super) description: Option<String>,
    #[serde(default)]
    pub(super) tags: Vec<String>,
    #[serde(default)]
    pub(super) visible_to_roles: Vec<String>,
    #[serde(default)]
    pub(super) default_environment_id: Option<String>,
    #[serde(default)]
    pub(super) inference: Option<InferenceTarget>,
    #[serde(default)]
    pub(super) repository: Option<ModelRepository>,
    #[serde(default)]
    pub(super) tools: StatisticalModelTools,
    #[serde(default)]
    pub(super) metadata: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(super) struct TrainingEnvironmentsRegistry {
    #[serde(default)]
    pub(super) environments: Vec<TrainingEnvironmentRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TrainingEnvironmentRecord {
    pub(super) id: String,
    #[serde(default)]
    pub(super) name: Option<String>,
    #[serde(default)]
    pub(super) description: Option<String>,
    #[serde(default)]
    pub(super) tags: Vec<String>,
    #[serde(default)]
    pub(super) visible_to_roles: Vec<String>,
    #[serde(default)]
    pub(super) status: EnvironmentStatus,
    #[serde(default)]
    pub(super) working_directory: Option<PathBuf>,
    #[serde(default)]
    pub(super) attach_instructions: Option<String>,
    #[serde(default)]
    pub(super) launch_command: Option<String>,
    #[serde(default)]
    pub(super) repository: Option<ModelRepository>,
    #[serde(default)]
    pub(super) tools: TrainingEnvironmentTools,
    #[serde(default)]
    pub(super) metadata: TrainingEnvironmentMetadata,
    #[serde(default)]
    pub(super) updated_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(super) struct TrainingEnvironmentMetadata {
    #[serde(default)]
    pub(super) provider: Option<String>,
    #[serde(default)]
    pub(super) runpod: Option<RunpodEnvironmentConfig>,
    #[serde(flatten)]
    pub(super) extra: BTreeMap<String, JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RunpodEnvironmentConfig {
    #[serde(default)]
    pub(super) template_id: Option<String>,
    #[serde(default)]
    pub(super) name: Option<String>,
    #[serde(default)]
    pub(super) image_name: Option<String>,
    #[serde(default)]
    pub(super) cloud_type: Option<String>,
    #[serde(default)]
    pub(super) gpu_type_id: Option<String>,
    #[serde(default)]
    pub(super) gpu_count: Option<u32>,
    #[serde(default)]
    pub(super) volume_mount_path: Option<String>,
    #[serde(default)]
    pub(super) workspace_path: Option<String>,
    #[serde(default)]
    pub(super) exposed_http_port: Option<u16>,
    #[serde(default)]
    pub(super) supports_ssh: bool,
    #[serde(default)]
    pub(super) container_disk_in_gb: Option<u32>,
    #[serde(default)]
    pub(super) volume_in_gb: Option<u32>,
    #[serde(default)]
    pub(super) network_volume_id: Option<String>,
    #[serde(default)]
    pub(super) data_center_ids: Vec<String>,
    #[serde(default)]
    pub(super) support_public_ip: Option<bool>,
    #[serde(default)]
    pub(super) docker_args: Vec<String>,
    #[serde(default)]
    pub(super) env: BTreeMap<String, String>,
    #[serde(default)]
    pub(super) stop_mode: Option<RunpodStopMode>,
    #[serde(default)]
    pub(super) startup_timeout_sec: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(super) enum RunpodStopMode {
    #[default]
    Stop,
    Terminate,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(super) struct StatisticalModelTools {
    #[serde(default)]
    pub(super) submit_training: Option<ExecutableActionBinding>,
    #[serde(default)]
    pub(super) submit_batch_inference: Option<ExecutableActionBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(super) struct TrainingEnvironmentTools {
    #[serde(default)]
    pub(super) launch: Option<ExecutableActionBinding>,
    #[serde(default)]
    pub(super) attach: Option<ExecutableActionBinding>,
    #[serde(default)]
    pub(super) stop: Option<ExecutableActionBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ExecutableActionBinding {
    #[serde(default = "default_enabled")]
    pub(super) enabled: bool,
    pub(super) command: String,
    #[serde(default)]
    pub(super) shell: Option<String>,
    #[serde(default)]
    pub(super) working_directory: Option<PathBuf>,
    #[serde(default)]
    pub(super) environment: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct InferenceTarget {
    pub(super) provider_id: String,
    pub(super) model_name: String,
    #[serde(default)]
    pub(super) base_url: Option<String>,
    #[serde(default)]
    pub(super) api_key_env: Option<String>,
    #[serde(default)]
    pub(super) wire_api: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ModelRepository {
    pub(super) root: PathBuf,
    #[serde(default)]
    pub(super) config_path: Option<PathBuf>,
    #[serde(default)]
    pub(super) ref_name: Option<String>,
    #[serde(default)]
    pub(super) entrypoint: Option<String>,
    #[serde(default)]
    pub(super) batch_entrypoint: Option<String>,
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(super) enum EnvironmentStatus {
    #[default]
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed,
    Terminated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(super) enum JobType {
    Training,
    BatchInference,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(super) enum JobStatus {
    #[default]
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct StoredJob {
    pub(super) id: String,
    pub(super) model_id: String,
    #[serde(rename = "type")]
    pub(super) job_type: JobType,
    #[serde(default)]
    pub(super) environment_id: Option<String>,
    pub(super) status: JobStatus,
    pub(super) created_at: i64,
    pub(super) updated_at: i64,
    pub(super) submitted_by_role: String,
    pub(super) payload: JsonValue,
    #[serde(default)]
    pub(super) execution: Option<StoredExecutionResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct StoredEval {
    pub(super) id: String,
    #[serde(default)]
    pub(super) model_id: Option<String>,
    #[serde(default)]
    pub(super) job_id: Option<String>,
    pub(super) status: JobStatus,
    pub(super) created_at: i64,
    #[serde(default)]
    pub(super) summary: Option<String>,
    #[serde(default)]
    pub(super) metrics: JsonValue,
    #[serde(default)]
    pub(super) artifact_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct StoredExecutionResult {
    pub(super) action: String,
    pub(super) command: String,
    #[serde(default)]
    pub(super) shell: Option<String>,
    #[serde(default)]
    pub(super) working_directory: Option<PathBuf>,
    #[serde(default)]
    pub(super) exit_code: Option<i32>,
    pub(super) success: bool,
    #[serde(default)]
    pub(super) stdout: String,
    #[serde(default)]
    pub(super) stderr: String,
    #[serde(default)]
    pub(super) summary_json: Option<JsonValue>,
    pub(super) ran_at: i64,
    pub(super) context_path: PathBuf,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ListModelsArgs {
    #[serde(default)]
    pub(super) tags: Vec<String>,
    pub(super) limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct GetModelArgs {
    pub(super) model_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SubmitJobArgs {
    pub(super) model_id: String,
    #[serde(rename = "type")]
    pub(super) job_type: JobType,
    pub(super) environment_id: Option<String>,
    #[serde(default)]
    pub(super) payload: JsonValue,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ListJobsArgs {
    pub(super) model_id: Option<String>,
    pub(super) status: Option<JobStatus>,
    pub(super) limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct GetJobArgs {
    pub(super) job_id: String,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ListEvalsArgs {
    pub(super) model_id: Option<String>,
    pub(super) job_id: Option<String>,
    pub(super) limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct GetEvalArgs {
    pub(super) eval_id: String,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ListTrainingEnvironmentsArgs {
    pub(super) status: Option<EnvironmentStatus>,
    #[serde(default)]
    pub(super) tags: Vec<String>,
    pub(super) limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct EnvironmentByIdArgs {
    pub(super) environment_id: String,
}
