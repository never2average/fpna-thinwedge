use super::DEFAULT_ROLE_NAME;
use super::cost_context;
use super::execution::ExecutionContext;
use super::execution::execute_action;
use super::storage::ensure_environment_visible;
use super::storage::get_visible_environment;
use super::storage::has_all_tags;
use super::storage::is_visible_to_role;
use super::storage::read_eval;
use super::storage::read_evals;
use super::storage::read_job;
use super::storage::read_jobs;
use super::storage::read_models_registry;
use super::storage::read_training_environments_registry;
use super::storage::write_job;
use super::storage::write_training_environments_registry;
use super::types::EnvironmentByIdArgs;
use super::types::EnvironmentStatus;
use super::types::ExecutableActionBinding;
use super::types::GetEvalArgs;
use super::types::GetJobArgs;
use super::types::GetModelArgs;
use super::types::JobStatus;
use super::types::JobType;
use super::types::ListEvalsArgs;
use super::types::ListJobsArgs;
use super::types::ListModelsArgs;
use super::types::ListTrainingEnvironmentsArgs;
use super::types::StatisticalModelRecord;
use super::types::StoredJob;
use super::types::SubmitJobArgs;
use super::types::TrainingEnvironmentRecord;
use chrono::Utc;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value as JsonValue;
use serde_json::json;
use std::path::Path;
use thinwedge_app_server_protocol::DynamicToolCallOutputContentItem;
use thinwedge_app_server_protocol::DynamicToolCallParams;
use thinwedge_app_server_protocol::DynamicToolCallResponse;
use thinwedge_app_server_protocol::DynamicToolSpec;
use uuid::Uuid;

pub(crate) fn dynamic_tool_specs() -> Vec<DynamicToolSpec> {
    let mut specs = vec![
        DynamicToolSpec {
            namespace: Some("statisticalmodels".to_string()),
            name: "list".to_string(),
            description: "List statistical models available to the current ThinWedge role."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "tags": { "type": "array", "items": { "type": "string" } },
                    "limit": { "type": "integer", "minimum": 1 }
                },
                "additionalProperties": false
            }),
            defer_loading: false,
        },
        DynamicToolSpec {
            namespace: Some("statisticalmodels".to_string()),
            name: "get".to_string(),
            description: "Read one statistical model definition by modelId.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "modelId": { "type": "string" }
                },
                "required": ["modelId"],
                "additionalProperties": false
            }),
            defer_loading: false,
        },
        DynamicToolSpec {
            namespace: Some("statisticalmodels".to_string()),
            name: "submitJob".to_string(),
            description:
                "Submit a training or batch-inference job through ThinWedge's filesystem registry and execution scripts."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "modelId": { "type": "string" },
                    "type": { "type": "string", "enum": ["training", "batchInference"] },
                    "environmentId": { "type": "string" },
                    "payload": {}
                },
                "required": ["modelId", "type"],
                "additionalProperties": false
            }),
            defer_loading: false,
        },
        DynamicToolSpec {
            namespace: Some("statisticalmodels".to_string()),
            name: "listJobs".to_string(),
            description: "List queued or historical statistical-model jobs recorded in ThinWedge's local indexes."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "modelId": { "type": "string" },
                    "status": {
                        "type": "string",
                        "enum": ["queued", "running", "completed", "failed", "cancelled"]
                    },
                    "limit": { "type": "integer", "minimum": 1 }
                },
                "additionalProperties": false
            }),
            defer_loading: false,
        },
        DynamicToolSpec {
            namespace: Some("statisticalmodels".to_string()),
            name: "getJob".to_string(),
            description: "Inspect one statistical-model job and its recorded execution summary."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "jobId": { "type": "string" }
                },
                "required": ["jobId"],
                "additionalProperties": false
            }),
            defer_loading: false,
        },
        DynamicToolSpec {
            namespace: Some("statisticalmodels".to_string()),
            name: "listEvals".to_string(),
            description: "List evaluation reports stored on the local filesystem.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "modelId": { "type": "string" },
                    "jobId": { "type": "string" },
                    "limit": { "type": "integer", "minimum": 1 }
                },
                "additionalProperties": false
            }),
            defer_loading: false,
        },
        DynamicToolSpec {
            namespace: Some("statisticalmodels".to_string()),
            name: "getEval".to_string(),
            description: "Inspect one evaluation report by evalId.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "evalId": { "type": "string" }
                },
                "required": ["evalId"],
                "additionalProperties": false
            }),
            defer_loading: false,
        },
        DynamicToolSpec {
            namespace: Some("trainingenvironments".to_string()),
            name: "list".to_string(),
            description: "List training environments visible to the current ThinWedge role."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "status": {
                        "type": "string",
                        "enum": ["stopped", "starting", "running", "stopping", "failed", "terminated"]
                    },
                    "tags": { "type": "array", "items": { "type": "string" } },
                    "limit": { "type": "integer", "minimum": 1 }
                },
                "additionalProperties": false
            }),
            defer_loading: false,
        },
        DynamicToolSpec {
            namespace: Some("trainingenvironments".to_string()),
            name: "get".to_string(),
            description: "Read one training environment definition by environmentId.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "environmentId": { "type": "string" }
                },
                "required": ["environmentId"],
                "additionalProperties": false
            }),
            defer_loading: false,
        },
        DynamicToolSpec {
            namespace: Some("trainingenvironments".to_string()),
            name: "launch".to_string(),
            description: "Launch or resume a training environment through ThinWedge's configured execution script."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "environmentId": { "type": "string" }
                },
                "required": ["environmentId"],
                "additionalProperties": false
            }),
            defer_loading: false,
        },
        DynamicToolSpec {
            namespace: Some("trainingenvironments".to_string()),
            name: "attach".to_string(),
            description: "Inspect live attach details for a training environment through ThinWedge's execution script."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "environmentId": { "type": "string" }
                },
                "required": ["environmentId"],
                "additionalProperties": false
            }),
            defer_loading: false,
        },
        DynamicToolSpec {
            namespace: Some("trainingenvironments".to_string()),
            name: "stop".to_string(),
            description: "Stop or terminate a training environment through ThinWedge's configured execution script."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "environmentId": { "type": "string" }
                },
                "required": ["environmentId"],
                "additionalProperties": false
            }),
            defer_loading: false,
        },
    ];
    specs.extend(cost_context::dynamic_tool_specs());
    specs
}

pub(crate) async fn handle_dynamic_tool_call(
    thinwedge_home: &Path,
    agent_role: Option<&str>,
    params: DynamicToolCallParams,
) -> DynamicToolCallResponse {
    let role = agent_role.unwrap_or(DEFAULT_ROLE_NAME);
    let result = match (params.namespace.as_deref(), params.tool.as_str()) {
        (Some("statisticalmodels"), "list") => {
            list_models(thinwedge_home, role, params.arguments).await
        }
        (Some("statisticalmodels"), "get") => {
            get_model(thinwedge_home, role, params.arguments).await
        }
        (Some("statisticalmodels"), "submitJob") => {
            submit_job(thinwedge_home, role, params.arguments).await
        }
        (Some("statisticalmodels"), "listJobs") => {
            list_jobs(thinwedge_home, params.arguments).await
        }
        (Some("statisticalmodels"), "getJob") => get_job(thinwedge_home, params.arguments).await,
        (Some("statisticalmodels"), "listEvals") => {
            list_evals(thinwedge_home, params.arguments).await
        }
        (Some("statisticalmodels"), "getEval") => get_eval(thinwedge_home, params.arguments).await,
        (Some("trainingenvironments"), "list") => {
            list_training_environments(thinwedge_home, role, params.arguments).await
        }
        (Some("trainingenvironments"), "get") => {
            get_training_environment(thinwedge_home, role, params.arguments).await
        }
        (Some("trainingenvironments"), "launch") => {
            set_training_environment_status(
                thinwedge_home,
                role,
                params.arguments,
                EnvironmentStatus::Running,
            )
            .await
        }
        (Some("trainingenvironments"), "attach") => {
            attach_training_environment(thinwedge_home, role, params.arguments).await
        }
        (Some("trainingenvironments"), "stop") => {
            set_training_environment_status(
                thinwedge_home,
                role,
                params.arguments,
                EnvironmentStatus::Stopped,
            )
            .await
        }
        (Some("llmcosts"), tool) | (Some("infracosts"), tool) => {
            let namespace = params.namespace.as_deref().unwrap_or_default();
            cost_context::handle_dynamic_tool_call(namespace, tool, params.arguments).await
        }
        (namespace, tool) => Err(format!(
            "Unsupported ThinWedge dynamic tool `{}`",
            namespace.map_or_else(
                || tool.to_string(),
                |namespace| format!("{namespace}.{tool}")
            )
        )),
    };

    match result {
        Ok(text) => success_response(text),
        Err(err) => error_response(err),
    }
}

async fn list_models(
    thinwedge_home: &Path,
    agent_role: &str,
    arguments: JsonValue,
) -> Result<String, String> {
    let args: ListModelsArgs = parse_arguments(arguments)?;
    let mut models = read_models_registry(thinwedge_home).await?.models;
    models.retain(|model| {
        is_visible_to_role(&model.visible_to_roles, agent_role)
            && has_all_tags(&model.tags, &args.tags)
    });
    if let Some(limit) = args.limit {
        models.truncate(limit);
    }
    pretty_json(&json!({ "models": models }))
}

async fn get_model(
    thinwedge_home: &Path,
    agent_role: &str,
    arguments: JsonValue,
) -> Result<String, String> {
    let args: GetModelArgs = parse_arguments(arguments)?;
    let registry = read_models_registry(thinwedge_home).await?;
    let model = registry
        .models
        .into_iter()
        .find(|model| {
            model.id == args.model_id && is_visible_to_role(&model.visible_to_roles, agent_role)
        })
        .ok_or_else(|| format!("statistical model `{}` was not found", args.model_id))?;
    let available_actions = model_available_actions(&model);
    let tool_commands = model_tool_commands(&model);
    pretty_json(&json!({
        "model": model,
        "availableActions": available_actions,
        "toolCommands": tool_commands,
    }))
}

async fn submit_job(
    thinwedge_home: &Path,
    agent_role: &str,
    arguments: JsonValue,
) -> Result<String, String> {
    let args: SubmitJobArgs = parse_arguments(arguments)?;
    let models = read_models_registry(thinwedge_home).await?;
    let model = models
        .models
        .into_iter()
        .find(|model| {
            model.id == args.model_id && is_visible_to_role(&model.visible_to_roles, agent_role)
        })
        .ok_or_else(|| format!("statistical model `{}` was not found", args.model_id))?;
    let binding = binding_for_job_type(&model, &args.job_type)?.clone();

    let selected_environment_id = args
        .environment_id
        .clone()
        .or(model.default_environment_id.clone());
    if let Some(environment_id) = selected_environment_id.as_deref() {
        ensure_environment_visible(thinwedge_home, agent_role, environment_id).await?;
    }
    let selected_environment = if let Some(environment_id) = selected_environment_id.as_deref() {
        Some(get_visible_environment(thinwedge_home, agent_role, environment_id).await?)
    } else {
        None
    };

    let now = Utc::now().timestamp();
    let mut job = StoredJob {
        id: Uuid::new_v4().to_string(),
        model_id: args.model_id,
        job_type: args.job_type,
        environment_id: selected_environment_id,
        status: JobStatus::Queued,
        created_at: now,
        updated_at: now,
        submitted_by_role: agent_role.to_string(),
        payload: args.payload,
        execution: None,
    };
    write_job(thinwedge_home, &job).await?;
    job.status = JobStatus::Running;
    write_job(thinwedge_home, &job).await?;
    let execution = execute_action(
        thinwedge_home,
        &binding,
        selected_environment
            .as_ref()
            .and_then(|environment| environment.working_directory.as_deref()),
        &ExecutionContext {
            action: model_action_name(&job.job_type).to_string(),
            agent_role: agent_role.to_string(),
            inference: model.inference.clone(),
            repository: model.repository.clone(),
            model: Some(
                serde_json::to_value(&model)
                    .map_err(|err| format!("failed to encode ThinWedge model: {err}"))?,
            ),
            environment: selected_environment
                .as_ref()
                .map(serde_json::to_value)
                .transpose()
                .map_err(|err| format!("failed to encode ThinWedge environment: {err}"))?,
            job: Some(
                serde_json::to_value(&job)
                    .map_err(|err| format!("failed to encode ThinWedge job: {err}"))?,
            ),
            payload: Some(job.payload.clone()),
        },
    )
    .await?;
    job.execution = Some(execution.clone());
    job.status = if execution.success {
        JobStatus::Completed
    } else {
        JobStatus::Failed
    };
    job.updated_at = Utc::now().timestamp();
    write_job(thinwedge_home, &job).await?;
    pretty_json(&json!({
        "job": job,
        "execution": execution,
        "message": "ThinWedge executed the model action from the filesystem registry and captured the recorded execution summary."
    }))
}

async fn list_jobs(thinwedge_home: &Path, arguments: JsonValue) -> Result<String, String> {
    let args: ListJobsArgs = parse_arguments(arguments)?;
    let mut jobs = read_jobs(thinwedge_home).await?;
    jobs.retain(|job| {
        args.model_id
            .as_deref()
            .is_none_or(|model_id| job.model_id == model_id)
            && args
                .status
                .as_ref()
                .is_none_or(|status| &job.status == status)
    });
    jobs.sort_by_key(|job| std::cmp::Reverse(job.created_at));
    if let Some(limit) = args.limit {
        jobs.truncate(limit);
    }
    pretty_json(&json!({ "jobs": jobs }))
}

async fn get_job(thinwedge_home: &Path, arguments: JsonValue) -> Result<String, String> {
    let args: GetJobArgs = parse_arguments(arguments)?;
    pretty_json(&read_job(thinwedge_home, &args.job_id).await?)
}

async fn list_evals(thinwedge_home: &Path, arguments: JsonValue) -> Result<String, String> {
    let args: ListEvalsArgs = parse_arguments(arguments)?;
    let mut evals = read_evals(thinwedge_home).await?;
    evals.retain(|eval| {
        args.model_id
            .as_deref()
            .is_none_or(|model_id| eval.model_id.as_deref() == Some(model_id))
            && args
                .job_id
                .as_deref()
                .is_none_or(|job_id| eval.job_id.as_deref() == Some(job_id))
    });
    evals.sort_by_key(|eval| std::cmp::Reverse(eval.created_at));
    if let Some(limit) = args.limit {
        evals.truncate(limit);
    }
    pretty_json(&json!({ "evals": evals }))
}

async fn get_eval(thinwedge_home: &Path, arguments: JsonValue) -> Result<String, String> {
    let args: GetEvalArgs = parse_arguments(arguments)?;
    pretty_json(&read_eval(thinwedge_home, &args.eval_id).await?)
}

async fn list_training_environments(
    thinwedge_home: &Path,
    agent_role: &str,
    arguments: JsonValue,
) -> Result<String, String> {
    let args: ListTrainingEnvironmentsArgs = parse_arguments(arguments)?;
    let mut environments = read_training_environments_registry(thinwedge_home)
        .await?
        .environments;
    environments.retain(|environment| {
        is_visible_to_role(&environment.visible_to_roles, agent_role)
            && args
                .status
                .as_ref()
                .is_none_or(|status| &environment.status == status)
            && has_all_tags(&environment.tags, &args.tags)
    });
    if let Some(limit) = args.limit {
        environments.truncate(limit);
    }
    pretty_json(&json!({ "trainingEnvironments": environments }))
}

async fn get_training_environment(
    thinwedge_home: &Path,
    agent_role: &str,
    arguments: JsonValue,
) -> Result<String, String> {
    let args: EnvironmentByIdArgs = parse_arguments(arguments)?;
    let environment =
        get_visible_environment(thinwedge_home, agent_role, &args.environment_id).await?;
    let available_actions = environment_available_actions(&environment);
    let tool_commands = environment_tool_commands(&environment);
    pretty_json(&json!({
        "environment": environment,
        "availableActions": available_actions,
        "toolCommands": tool_commands,
    }))
}

async fn attach_training_environment(
    thinwedge_home: &Path,
    agent_role: &str,
    arguments: JsonValue,
) -> Result<String, String> {
    let args: EnvironmentByIdArgs = parse_arguments(arguments)?;
    let environment =
        get_visible_environment(thinwedge_home, agent_role, &args.environment_id).await?;
    let binding = environment.tools.attach.as_ref().cloned().ok_or_else(|| {
        format!(
            "training environment `{}` does not define an attach action",
            environment.id
        )
    })?;
    let execution = execute_action(
        thinwedge_home,
        &binding,
        environment.working_directory.as_deref(),
        &ExecutionContext {
            action: "attach".to_string(),
            agent_role: agent_role.to_string(),
            inference: None,
            repository: environment.repository.clone(),
            model: None,
            environment: Some(
                serde_json::to_value(&environment)
                    .map_err(|err| format!("failed to encode ThinWedge environment: {err}"))?,
            ),
            job: None,
            payload: None,
        },
    )
    .await?;
    pretty_json(&json!({
        "environment": environment,
        "execution": execution,
        "message": "ThinWedge executed the environment attach action and returned the recorded attach summary."
    }))
}

async fn set_training_environment_status(
    thinwedge_home: &Path,
    agent_role: &str,
    arguments: JsonValue,
    status: EnvironmentStatus,
) -> Result<String, String> {
    let args: EnvironmentByIdArgs = parse_arguments(arguments)?;
    let mut registry = read_training_environments_registry(thinwedge_home).await?;
    let environment = registry
        .environments
        .iter_mut()
        .find(|environment| {
            environment.id == args.environment_id
                && is_visible_to_role(&environment.visible_to_roles, agent_role)
        })
        .ok_or_else(|| {
            format!(
                "training environment `{}` was not found",
                args.environment_id
            )
        })?;
    let environment_snapshot = environment.clone();
    let binding = environment_binding_for_status(&environment_snapshot, &status)?.clone();
    let execution = execute_action(
        thinwedge_home,
        &binding,
        environment_snapshot.working_directory.as_deref(),
        &ExecutionContext {
            action: environment_action_name(&status).to_string(),
            agent_role: agent_role.to_string(),
            inference: None,
            repository: environment_snapshot.repository.clone(),
            model: None,
            environment: Some(
                serde_json::to_value(&environment_snapshot)
                    .map_err(|err| format!("failed to encode ThinWedge environment: {err}"))?,
            ),
            job: None,
            payload: None,
        },
    )
    .await?;
    if !execution.success {
        return pretty_json(&json!({
            "environment": environment,
            "execution": execution,
            "message": "ThinWedge executed the environment action, but the command failed."
        }));
    }
    environment.status = environment_status_from_execution(&execution).unwrap_or(status);
    environment.updated_at = Some(Utc::now().timestamp());
    let updated_environment = environment.clone();
    write_training_environments_registry(thinwedge_home, &registry).await?;
    pretty_json(&json!({
        "environment": updated_environment,
        "execution": execution,
        "message": "ThinWedge executed the environment action and refreshed the persisted environment state from the recorded summary."
    }))
}

fn success_response(text: String) -> DynamicToolCallResponse {
    DynamicToolCallResponse {
        content_items: vec![DynamicToolCallOutputContentItem::InputText { text }],
        success: true,
    }
}

fn error_response(text: String) -> DynamicToolCallResponse {
    DynamicToolCallResponse {
        content_items: vec![DynamicToolCallOutputContentItem::InputText { text }],
        success: false,
    }
}

fn pretty_json<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string_pretty(value)
        .map_err(|err| format!("failed to encode ThinWedge ML response: {err}"))
}

fn parse_arguments<T>(arguments: JsonValue) -> Result<T, String>
where
    T: DeserializeOwned,
{
    serde_json::from_value(arguments)
        .map_err(|err| format!("invalid ThinWedge ML tool arguments: {err}"))
}

fn binding_for_job_type<'a>(
    model: &'a StatisticalModelRecord,
    job_type: &JobType,
) -> Result<&'a ExecutableActionBinding, String> {
    let binding = match job_type {
        JobType::Training => model.tools.submit_training.as_ref(),
        JobType::BatchInference => model.tools.submit_batch_inference.as_ref(),
    };
    binding.ok_or_else(|| {
        format!(
            "statistical model `{}` does not define a {} action",
            model.id,
            model_action_name(job_type)
        )
    })
}

fn environment_binding_for_status<'a>(
    environment: &'a TrainingEnvironmentRecord,
    status: &EnvironmentStatus,
) -> Result<&'a ExecutableActionBinding, String> {
    let binding = match status {
        EnvironmentStatus::Starting | EnvironmentStatus::Stopping | EnvironmentStatus::Failed => {
            return Err(format!(
                "training environment `{}` does not support a direct `{}` action",
                environment.id,
                environment_action_name(status)
            ));
        }
        EnvironmentStatus::Running => environment.tools.launch.as_ref(),
        EnvironmentStatus::Stopped => environment.tools.stop.as_ref(),
        EnvironmentStatus::Terminated => environment.tools.stop.as_ref(),
    };
    binding.ok_or_else(|| {
        format!(
            "training environment `{}` does not define a {} action",
            environment.id,
            environment_action_name(status)
        )
    })
}

fn model_action_name(job_type: &JobType) -> &'static str {
    match job_type {
        JobType::Training => "submitTraining",
        JobType::BatchInference => "submitBatchInference",
    }
}

fn environment_action_name(status: &EnvironmentStatus) -> &'static str {
    match status {
        EnvironmentStatus::Starting => "launch",
        EnvironmentStatus::Running => "launch",
        EnvironmentStatus::Stopping => "stop",
        EnvironmentStatus::Stopped => "stop",
        EnvironmentStatus::Failed => "status",
        EnvironmentStatus::Terminated => "stop",
    }
}

fn environment_status_from_execution(
    execution: &super::types::StoredExecutionResult,
) -> Option<EnvironmentStatus> {
    let status = execution.summary_json.as_ref()?.get("status")?.as_str()?;
    match status {
        "starting" => Some(EnvironmentStatus::Starting),
        "running" => Some(EnvironmentStatus::Running),
        "stopping" => Some(EnvironmentStatus::Stopping),
        "stopped" => Some(EnvironmentStatus::Stopped),
        "failed" => Some(EnvironmentStatus::Failed),
        "terminated" => Some(EnvironmentStatus::Terminated),
        _ => None,
    }
}

fn model_available_actions(model: &StatisticalModelRecord) -> Vec<&'static str> {
    let mut actions = Vec::new();
    if model
        .tools
        .submit_training
        .as_ref()
        .is_some_and(|binding| binding.enabled)
    {
        actions.push("submitTraining");
    }
    if model
        .tools
        .submit_batch_inference
        .as_ref()
        .is_some_and(|binding| binding.enabled)
    {
        actions.push("submitBatchInference");
    }
    actions
}

fn environment_available_actions(environment: &TrainingEnvironmentRecord) -> Vec<&'static str> {
    let mut actions = Vec::new();
    if environment
        .tools
        .launch
        .as_ref()
        .is_some_and(|binding| binding.enabled)
    {
        actions.push("launch");
    }
    if environment
        .tools
        .attach
        .as_ref()
        .is_some_and(|binding| binding.enabled)
    {
        actions.push("attach");
    }
    if environment
        .tools
        .stop
        .as_ref()
        .is_some_and(|binding| binding.enabled)
    {
        actions.push("stop");
    }
    actions
}

fn model_tool_commands(model: &StatisticalModelRecord) -> Vec<JsonValue> {
    let mut commands = Vec::new();
    if let Some(binding) = model.tools.submit_training.as_ref() {
        commands.push(action_command_json("submitTraining", binding));
    }
    if let Some(binding) = model.tools.submit_batch_inference.as_ref() {
        commands.push(action_command_json("submitBatchInference", binding));
    }
    commands
}

fn environment_tool_commands(environment: &TrainingEnvironmentRecord) -> Vec<JsonValue> {
    let mut commands = Vec::new();
    if let Some(binding) = environment.tools.launch.as_ref() {
        commands.push(action_command_json("launch", binding));
    }
    if let Some(binding) = environment.tools.attach.as_ref() {
        commands.push(action_command_json("attach", binding));
    }
    if let Some(binding) = environment.tools.stop.as_ref() {
        commands.push(action_command_json("stop", binding));
    }
    commands
}

fn action_command_json(action: &str, binding: &ExecutableActionBinding) -> JsonValue {
    json!({
        "name": action,
        "enabled": binding.enabled,
        "shell": binding.shell,
        "command": binding.command,
        "workingDirectory": binding.working_directory,
        "environment": binding.environment,
    })
}
