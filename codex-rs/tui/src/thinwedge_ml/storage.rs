use super::ENVIRONMENTS_FILE_NAME;
use super::EVALS_DIR_NAME;
use super::JOBS_DIR_NAME;
use super::MODELS_FILE_NAME;
use super::THINWEDGE_DATA_DIR;
use super::types::StatisticalModelsRegistry;
use super::types::StoredEval;
use super::types::StoredJob;
use super::types::TrainingEnvironmentRecord;
use super::types::TrainingEnvironmentsRegistry;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::path::Path;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

pub(super) fn thinwedge_ml_dir(codex_home: &Path) -> PathBuf {
    codex_home.join(THINWEDGE_DATA_DIR)
}

pub(super) fn models_registry_path(codex_home: &Path) -> PathBuf {
    thinwedge_ml_dir(codex_home).join(MODELS_FILE_NAME)
}

pub(super) fn training_environments_registry_path(codex_home: &Path) -> PathBuf {
    thinwedge_ml_dir(codex_home).join(ENVIRONMENTS_FILE_NAME)
}

pub(super) fn jobs_dir(codex_home: &Path) -> PathBuf {
    thinwedge_ml_dir(codex_home).join(JOBS_DIR_NAME)
}

pub(super) fn evals_dir(codex_home: &Path) -> PathBuf {
    thinwedge_ml_dir(codex_home).join(EVALS_DIR_NAME)
}

pub(super) fn runtime_contexts_dir(codex_home: &Path) -> PathBuf {
    thinwedge_ml_dir(codex_home).join("runtime")
}

pub(super) async fn read_models_registry(
    codex_home: &Path,
) -> Result<StatisticalModelsRegistry, String> {
    read_json_or_default(&models_registry_path(codex_home)).await
}

pub(super) async fn read_training_environments_registry(
    codex_home: &Path,
) -> Result<TrainingEnvironmentsRegistry, String> {
    read_json_or_default(&training_environments_registry_path(codex_home)).await
}

pub(super) async fn write_training_environments_registry(
    codex_home: &Path,
    registry: &TrainingEnvironmentsRegistry,
) -> Result<(), String> {
    write_json_pretty(&training_environments_registry_path(codex_home), registry).await
}

pub(super) async fn write_job(codex_home: &Path, job: &StoredJob) -> Result<(), String> {
    write_json_pretty(&jobs_dir(codex_home).join(format!("{}.json", job.id)), job).await
}

pub(super) async fn read_job(codex_home: &Path, job_id: &str) -> Result<StoredJob, String> {
    read_json_required(&jobs_dir(codex_home).join(format!("{job_id}.json"))).await
}

pub(super) async fn read_eval(codex_home: &Path, eval_id: &str) -> Result<StoredEval, String> {
    read_json_required(&evals_dir(codex_home).join(format!("{eval_id}.json"))).await
}

pub(super) async fn read_jobs(codex_home: &Path) -> Result<Vec<StoredJob>, String> {
    read_json_directory::<StoredJob>(&jobs_dir(codex_home)).await
}

pub(super) async fn read_evals(codex_home: &Path) -> Result<Vec<StoredEval>, String> {
    read_json_directory::<StoredEval>(&evals_dir(codex_home)).await
}

pub(super) fn is_visible_to_role(visible_to_roles: &[String], agent_role: &str) -> bool {
    visible_to_roles.is_empty()
        || visible_to_roles
            .iter()
            .any(|role| role.eq_ignore_ascii_case(agent_role))
}

pub(super) fn has_all_tags(candidate_tags: &[String], required_tags: &[String]) -> bool {
    required_tags
        .iter()
        .all(|required_tag| candidate_tags.iter().any(|tag| tag == required_tag))
}

pub(super) async fn get_visible_environment(
    codex_home: &Path,
    agent_role: &str,
    environment_id: &str,
) -> Result<TrainingEnvironmentRecord, String> {
    let registry = read_training_environments_registry(codex_home).await?;
    registry
        .environments
        .into_iter()
        .find(|environment| {
            environment.id == environment_id
                && is_visible_to_role(&environment.visible_to_roles, agent_role)
        })
        .ok_or_else(|| format!("training environment `{environment_id}` was not found"))
}

pub(super) async fn ensure_environment_visible(
    codex_home: &Path,
    agent_role: &str,
    environment_id: &str,
) -> Result<(), String> {
    get_visible_environment(codex_home, agent_role, environment_id)
        .await
        .map(|_| ())
}

pub(super) async fn write_json_pretty<T>(path: &Path, value: &T) -> Result<(), String>
where
    T: Serialize,
{
    let parent = path
        .parent()
        .ok_or_else(|| format!("path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent)
        .await
        .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    let contents = serde_json::to_vec_pretty(value)
        .map_err(|err| format!("failed to encode {}: {err}", path.display()))?;
    let mut file = fs::File::create(path)
        .await
        .map_err(|err| format!("failed to create {}: {err}", path.display()))?;
    file.write_all(&contents)
        .await
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    file.write_all(b"\n")
        .await
        .map_err(|err| format!("failed to finalize {}: {err}", path.display()))
}

async fn read_json_or_default<T>(path: &Path) -> Result<T, String>
where
    T: DeserializeOwned + Default,
{
    match fs::try_exists(path).await {
        Ok(false) => Ok(T::default()),
        Ok(true) => read_json_required(path).await,
        Err(err) => Err(format!("failed to inspect {}: {err}", path.display())),
    }
}

async fn read_json_required<T>(path: &Path) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let contents = fs::read_to_string(path)
        .await
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|err| format!("failed to decode {}: {err}", path.display()))
}

async fn read_json_directory<T>(dir: &Path) -> Result<Vec<T>, String>
where
    T: DeserializeOwned,
{
    match fs::try_exists(dir).await {
        Ok(false) => return Ok(Vec::new()),
        Ok(true) => {}
        Err(err) => return Err(format!("failed to inspect {}: {err}", dir.display())),
    }
    let mut entries = fs::read_dir(dir)
        .await
        .map_err(|err| format!("failed to read {}: {err}", dir.display()))?;
    let mut values = Vec::new();
    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|err| format!("failed to iterate {}: {err}", dir.display()))?
    {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        values.push(read_json_required(&path).await?);
    }
    Ok(values)
}
