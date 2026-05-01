use super::spec::dynamic_tool_specs;
use super::spec::handle_dynamic_tool_call;
use super::storage::models_registry_path;
use super::storage::read_jobs;
use super::storage::read_training_environments_registry;
use super::storage::training_environments_registry_path;
use super::storage::write_json_pretty;
use super::types::EnvironmentStatus;
use super::types::ExecutableActionBinding;
use super::types::InferenceTarget;
use super::types::JobStatus;
use super::types::JobType;
use super::types::ModelRepository;
use super::types::RunpodEnvironmentConfig;
use super::types::RunpodStopMode;
use super::types::StatisticalModelRecord;
use super::types::StatisticalModelTools;
use super::types::StatisticalModelsRegistry;
use super::types::TrainingEnvironmentMetadata;
use super::types::TrainingEnvironmentRecord;
use super::types::TrainingEnvironmentTools;
use super::types::TrainingEnvironmentsRegistry;
use thinwedge_app_server_protocol::DynamicToolCallOutputContentItem;
use thinwedge_app_server_protocol::DynamicToolCallParams;
use pretty_assertions::assert_eq;
use serde_json::Value as JsonValue;
use serde_json::json;
use std::collections::BTreeMap;
use std::path::PathBuf;
use tempfile::TempDir;

async fn seed_models(home: &TempDir) {
    write_json_pretty(
        &models_registry_path(home.path()),
        &StatisticalModelsRegistry {
            models: vec![
                StatisticalModelRecord {
                    id: "model-pricing".to_string(),
                    name: Some("Pricing Model".to_string()),
                    description: Some("Pricing research model".to_string()),
                    tags: vec!["pricing".to_string(), "gpu".to_string()],
                    visible_to_roles: vec!["pricing_researcher".to_string()],
                    default_environment_id: Some("env-pricing".to_string()),
                    inference: Some(InferenceTarget {
                        provider_id: "openrouter".to_string(),
                        model_name: "openai/gpt-4.1-mini".to_string(),
                        base_url: Some("https://openrouter.ai/api/v1".to_string()),
                        api_key_env: Some("OPENROUTER_API_KEY".to_string()),
                        wire_api: Some("chatCompletions".to_string()),
                    }),
                    repository: Some(ModelRepository {
                        root: PathBuf::from("/tmp/model-pricing-repo"),
                        config_path: Some(PathBuf::from("/tmp/model-pricing-repo/config.yaml")),
                        ref_name: Some("main".to_string()),
                        entrypoint: Some("scripts/train.sh".to_string()),
                        batch_entrypoint: Some("scripts/run_batch_inference.py".to_string()),
                    }),
                    tools: StatisticalModelTools {
                        submit_training: Some(ExecutableActionBinding {
                            enabled: true,
                            command: "printf 'trained:%s:%s:%s' \"$THINWEDGE_MODEL_ID\" \"$THINWEDGE_INFERENCE_PROVIDER\" \"$THINWEDGE_MODEL_REPOSITORY_ROOT\"".to_string(),
                            shell: None,
                            working_directory: None,
                            environment: BTreeMap::new(),
                        }),
                        submit_batch_inference: None,
                    },
                    metadata: json!({"family": "xgboost"}),
                },
                StatisticalModelRecord {
                    id: "model-general".to_string(),
                    name: Some("General Model".to_string()),
                    description: None,
                    tags: vec!["general".to_string()],
                    visible_to_roles: Vec::new(),
                    default_environment_id: None,
                    inference: None,
                    repository: None,
                    tools: StatisticalModelTools::default(),
                    metadata: JsonValue::Null,
                },
            ],
        },
    )
    .await
    .expect("write models");
}

async fn seed_environments(home: &TempDir) {
    write_json_pretty(
        &training_environments_registry_path(home.path()),
        &TrainingEnvironmentsRegistry {
            environments: vec![
                TrainingEnvironmentRecord {
                    id: "env-pricing".to_string(),
                    name: Some("Pricing GPU".to_string()),
                    description: None,
                    tags: vec!["pricing".to_string(), "gpu".to_string()],
                    visible_to_roles: vec!["pricing_researcher".to_string()],
                    status: EnvironmentStatus::Stopped,
                    working_directory: Some(PathBuf::from("/tmp/pricing")),
                    attach_instructions: Some("ssh pricing".to_string()),
                    launch_command: None,
                    repository: Some(ModelRepository {
                        root: PathBuf::from("/tmp/env-pricing-repo"),
                        config_path: None,
                        ref_name: Some("gpu-main".to_string()),
                        entrypoint: Some("ops/launch.sh".to_string()),
                        batch_entrypoint: None,
                    }),
                    tools: TrainingEnvironmentTools {
                        launch: Some(ExecutableActionBinding {
                            enabled: true,
                            command: "printf '{\"status\":\"running\",\"environmentId\":\"%s\",\"workspacePath\":\"/workspace/thinwedge/env-pricing\"}' \"$THINWEDGE_ENVIRONMENT_ID\"".to_string(),
                            shell: None,
                            working_directory: None,
                            environment: BTreeMap::new(),
                        }),
                        attach: Some(ExecutableActionBinding {
                            enabled: true,
                            command: "printf 'attach:%s:%s' \"$THINWEDGE_ENVIRONMENT_ID\" \"$THINWEDGE_MODEL_REPOSITORY_ROOT\"".to_string(),
                            shell: None,
                            working_directory: None,
                            environment: BTreeMap::new(),
                        }),
                        stop: Some(ExecutableActionBinding {
                            enabled: true,
                            command: "printf 'stop:%s:%s' \"$THINWEDGE_ENVIRONMENT_ID\" \"$THINWEDGE_MODEL_REPOSITORY_ROOT\"".to_string(),
                            shell: None,
                            working_directory: None,
                            environment: BTreeMap::new(),
                        }),
                    },
                    metadata: TrainingEnvironmentMetadata {
                        provider: Some("runpod".to_string()),
                        runpod: Some(RunpodEnvironmentConfig {
                            template_id: Some("tmpl-thinwedge-pricing".to_string()),
                            name: Some("thinwedge-pricing-env".to_string()),
                            image_name: None,
                            cloud_type: Some("SECURE".to_string()),
                            gpu_type_id: Some("NVIDIA A100 80GB PCIe".to_string()),
                            gpu_count: Some(1),
                            volume_mount_path: Some("/workspace".to_string()),
                            workspace_path: Some("/workspace/thinwedge/env-pricing".to_string()),
                            exposed_http_port: Some(8000),
                            supports_ssh: true,
                            container_disk_in_gb: Some(50),
                            volume_in_gb: Some(100),
                            network_volume_id: None,
                            data_center_ids: Vec::new(),
                            support_public_ip: Some(true),
                            docker_args: Vec::new(),
                            env: BTreeMap::new(),
                            stop_mode: Some(RunpodStopMode::Stop),
                            startup_timeout_sec: Some(900),
                        }),
                        extra: BTreeMap::new(),
                    },
                    updated_at: None,
                },
                TrainingEnvironmentRecord {
                    id: "env-general".to_string(),
                    name: Some("General GPU".to_string()),
                    description: None,
                    tags: vec!["general".to_string(), "gpu".to_string()],
                    visible_to_roles: vec!["CFO".to_string()],
                    status: EnvironmentStatus::Running,
                    working_directory: None,
                    attach_instructions: None,
                    launch_command: None,
                    repository: None,
                    tools: TrainingEnvironmentTools::default(),
                    metadata: TrainingEnvironmentMetadata::default(),
                    updated_at: None,
                },
            ],
        },
    )
    .await
    .expect("write envs");
}

#[tokio::test]
async fn dynamic_tool_specs_register_thinwedge_namespaces() {
    let specs = dynamic_tool_specs();
    assert_eq!(
        specs
            .iter()
            .filter_map(|spec| spec.namespace.as_deref())
            .count(),
        29
    );
    assert!(specs.iter().any(|spec| {
        spec.namespace.as_deref() == Some("statisticalmodels") && spec.name == "submitJob"
    }));
    assert!(specs.iter().any(|spec| {
        spec.namespace.as_deref() == Some("trainingenvironments") && spec.name == "launch"
    }));
    assert!(specs.iter().any(|spec| {
        spec.namespace.as_deref() == Some("llmcosts") && spec.name == "listModels"
    }));
    assert!(specs.iter().any(|spec| {
        spec.namespace.as_deref() == Some("infracosts") && spec.name == "getAwsCostAndUsage"
    }));
    assert!(specs.iter().any(|spec| {
        spec.namespace.as_deref() == Some("infracosts") && spec.name == "searchAwsServices"
    }));
    assert!(specs.iter().any(|spec| {
        spec.namespace.as_deref() == Some("infracosts") && spec.name == "getAwsVmPrice"
    }));
    assert!(specs.iter().any(|spec| {
        spec.namespace.as_deref() == Some("infracosts") && spec.name == "listBillingViews"
    }));
    assert!(specs.iter().any(|spec| {
        spec.namespace.as_deref() == Some("infracosts") && spec.name == "queryAwsByService"
    }));
    assert!(specs.iter().any(|spec| {
        spec.namespace.as_deref() == Some("infracosts") && spec.name == "estimateAwsBoq"
    }));
}

#[tokio::test]
async fn list_training_environments_filters_by_role() {
    let home = TempDir::new().expect("tempdir");
    seed_environments(&home).await;

    let response = handle_dynamic_tool_call(
        home.path(),
        Some("pricing_researcher"),
        DynamicToolCallParams {
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            namespace: Some("trainingenvironments".to_string()),
            tool: "list".to_string(),
            arguments: json!({}),
        },
    )
    .await;

    assert!(response.success);
    let [DynamicToolCallOutputContentItem::InputText { text }] = response.content_items.as_slice()
    else {
        panic!("expected one text item");
    };
    assert!(text.contains("env-pricing"));
    assert!(!text.contains("env-general"));
}

#[tokio::test]
async fn submit_job_writes_local_job_file() {
    let home = TempDir::new().expect("tempdir");
    seed_models(&home).await;
    seed_environments(&home).await;

    let response = handle_dynamic_tool_call(
        home.path(),
        Some("pricing_researcher"),
        DynamicToolCallParams {
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            namespace: Some("statisticalmodels".to_string()),
            tool: "submitJob".to_string(),
            arguments: json!({
                "modelId": "model-pricing",
                "type": "training",
                "payload": {"epochs": 3}
            }),
        },
    )
    .await;

    assert!(response.success);
    let jobs = read_jobs(home.path()).await.expect("jobs should load");
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].model_id, "model-pricing");
    assert_eq!(jobs[0].job_type, JobType::Training);
    assert_eq!(jobs[0].status, JobStatus::Completed);
    assert_eq!(jobs[0].submitted_by_role, "pricing_researcher");
    assert_eq!(
        jobs[0]
            .execution
            .as_ref()
            .expect("job execution should be recorded")
            .stdout,
        "trained:model-pricing:openrouter:/tmp/model-pricing-repo"
    );
    assert_eq!(
        jobs[0]
            .execution
            .as_ref()
            .expect("job execution should be recorded")
            .summary_json,
        None
    );
}

#[tokio::test]
async fn launch_updates_local_environment_state() {
    let home = TempDir::new().expect("tempdir");
    seed_environments(&home).await;

    let response = handle_dynamic_tool_call(
        home.path(),
        Some("pricing_researcher"),
        DynamicToolCallParams {
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            namespace: Some("trainingenvironments".to_string()),
            tool: "launch".to_string(),
            arguments: json!({"environmentId": "env-pricing"}),
        },
    )
    .await;

    assert!(response.success);
    let registry = read_training_environments_registry(home.path())
        .await
        .expect("registry should load");
    assert_eq!(registry.environments[0].status, EnvironmentStatus::Running);
    let [DynamicToolCallOutputContentItem::InputText { text }] = response.content_items.as_slice()
    else {
        panic!("expected one text item");
    };
    assert!(text.contains("\"status\": \"running\""));
}

#[tokio::test]
async fn attach_runs_environment_action_command() {
    let home = TempDir::new().expect("tempdir");
    seed_environments(&home).await;

    let response = handle_dynamic_tool_call(
        home.path(),
        Some("pricing_researcher"),
        DynamicToolCallParams {
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            namespace: Some("trainingenvironments".to_string()),
            tool: "attach".to_string(),
            arguments: json!({"environmentId": "env-pricing"}),
        },
    )
    .await;

    assert!(response.success);
    let [DynamicToolCallOutputContentItem::InputText { text }] = response.content_items.as_slice()
    else {
        panic!("expected one text item");
    };
    assert!(text.contains("attach:env-pricing:/tmp/env-pricing-repo"));
}

#[tokio::test]
async fn get_missing_eval_returns_failure() {
    let home = TempDir::new().expect("tempdir");

    let response = handle_dynamic_tool_call(
        home.path(),
        Some("CFO"),
        DynamicToolCallParams {
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            namespace: Some("statisticalmodels".to_string()),
            tool: "getEval".to_string(),
            arguments: json!({"evalId": "missing"}),
        },
    )
    .await;

    assert!(!response.success);
    let [DynamicToolCallOutputContentItem::InputText { text }] = response.content_items.as_slice()
    else {
        panic!("expected one text item");
    };
    assert!(text.contains("missing.json"));
}
