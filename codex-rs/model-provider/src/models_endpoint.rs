use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use codex_api::ModelsClient;
use codex_api::RequestTelemetry;
use codex_api::ReqwestTransport;
use codex_api::TransportError;
use codex_api::auth_header_telemetry;
use codex_api::map_api_error;
use codex_feedback::FeedbackRequestTags;
use codex_feedback::emit_feedback_request_tags_with_auth_env;
use codex_login::AuthEnvTelemetry;
use codex_login::AuthManager;
use codex_login::CodexAuth;
use codex_login::collect_auth_env_telemetry;
use codex_login::default_client::build_reqwest_client;
use codex_model_provider_info::ModelProviderInfo;
use codex_models_manager::manager::ModelsCacheProviderIdentity;
use codex_models_manager::manager::ModelsEndpointClient;
use codex_models_manager::model_info::model_info_from_slug;
use codex_otel::TelemetryAuthMode;
use codex_protocol::error::CodexErr;
use codex_protocol::error::Result as CoreResult;
use codex_protocol::openai_models::InputModality;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ModelVisibility;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::openai_models::ReasoningEffortPreset;
use codex_response_debug_context::extract_response_debug_context;
use codex_response_debug_context::telemetry_transport_error_message;
use http::HeaderMap;
use serde::Deserialize;
use tokio::time::timeout;

use crate::auth::resolve_provider_auth;

const MODELS_REFRESH_TIMEOUT: Duration = Duration::from_secs(5);
const MODELS_ENDPOINT: &str = "/models";

/// Provider-owned OpenAI-compatible `/models` endpoint.
#[derive(Debug)]
pub(crate) struct OpenAiModelsEndpoint {
    provider_info: ModelProviderInfo,
    auth_manager: Option<Arc<AuthManager>>,
}

impl OpenAiModelsEndpoint {
    pub(crate) fn new(
        provider_info: ModelProviderInfo,
        auth_manager: Option<Arc<AuthManager>>,
    ) -> Self {
        Self {
            provider_info,
            auth_manager,
        }
    }

    async fn auth(&self) -> Option<CodexAuth> {
        match self.auth_manager.as_ref() {
            Some(auth_manager) => auth_manager.auth().await,
            None => None,
        }
    }

    fn auth_env(&self) -> AuthEnvTelemetry {
        let codex_api_key_env_enabled = self
            .auth_manager
            .as_ref()
            .is_some_and(|auth_manager| auth_manager.codex_api_key_env_enabled());
        collect_auth_env_telemetry(&self.provider_info, codex_api_key_env_enabled)
    }

    async fn provider_identity(&self) -> CoreResult<ModelsCacheProviderIdentity> {
        let auth = self.auth().await;
        let api_provider = self
            .provider_info
            .to_api_provider(auth.as_ref().map(CodexAuth::auth_mode))?;
        Ok(ModelsCacheProviderIdentity {
            name: self.provider_info.name.clone(),
            base_url: api_provider.base_url.trim_end_matches('/').to_string(),
        })
    }

    async fn list_openai_compatible_models(
        &self,
        client_version: &str,
    ) -> CoreResult<(Vec<ModelInfo>, Option<String>)> {
        let auth = self.auth().await;
        let auth_mode = auth.as_ref().map(CodexAuth::auth_mode);
        let api_provider = self.provider_info.to_api_provider(auth_mode)?;
        let api_auth = resolve_provider_auth(auth.as_ref(), &self.provider_info)?;
        let transport = ReqwestTransport::new(build_reqwest_client());
        let auth_telemetry = auth_header_telemetry(api_auth.as_ref());
        let request_telemetry: Arc<dyn RequestTelemetry> = Arc::new(ModelsRequestTelemetry {
            auth_mode: auth_mode.map(|mode| TelemetryAuthMode::from(mode).to_string()),
            auth_header_attached: auth_telemetry.attached,
            auth_header_name: auth_telemetry.name,
            auth_env: self.auth_env(),
        });
        let client = ModelsClient::new(transport, api_provider, api_auth)
            .with_telemetry(Some(request_telemetry));

        timeout(
            MODELS_REFRESH_TIMEOUT,
            client.list_models(client_version, HeaderMap::new()),
        )
        .await
        .map_err(|_| CodexErr::Timeout)?
        .map_err(map_api_error)
    }

    async fn list_openrouter_models(
        &self,
        client_version: &str,
    ) -> CoreResult<(Vec<ModelInfo>, Option<String>)> {
        match self.list_openai_compatible_models(client_version).await {
            Ok(models) => Ok(models),
            Err(err) if self.provider_info.is_openrouter() => {
                tracing::info!("falling back to OpenRouter models schema adapter: {err}");
                self.list_openrouter_models_fallback().await
            }
            Err(err) => Err(err),
        }
    }

    async fn list_openrouter_models_fallback(
        &self,
    ) -> CoreResult<(Vec<ModelInfo>, Option<String>)> {
        let auth = self.auth().await;
        let api_provider = self
            .provider_info
            .to_api_provider(auth.as_ref().map(CodexAuth::auth_mode))?;
        let api_auth = resolve_provider_auth(auth.as_ref(), &self.provider_info)?;
        let mut headers = HeaderMap::new();
        api_auth.add_auth_headers(&mut headers);
        let response = build_reqwest_client()
            .get(format!(
                "{}/models",
                api_provider.base_url.trim_end_matches('/')
            ))
            .headers(headers)
            .send()
            .await
            .map_err(|err| CodexErr::Stream(err.to_string(), None))?;
        let etag = response
            .headers()
            .get(http::header::ETAG)
            .and_then(|value| value.to_str().ok())
            .map(ToString::to_string);
        let status = response.status();
        let body = response
            .bytes()
            .await
            .map_err(|err| CodexErr::Stream(err.to_string(), None))?;
        if !status.is_success() {
            return Err(CodexErr::Stream(
                format!(
                    "OpenRouter /models request failed: {status}: {}",
                    String::from_utf8_lossy(&body)
                ),
                None,
            ));
        }
        let response: OpenRouterModelsResponse = serde_json::from_slice(&body).map_err(|err| {
            CodexErr::Stream(
                format!("failed to decode OpenRouter models response: {err}"),
                None,
            )
        })?;
        Ok((
            response
                .data
                .into_iter()
                .map(openrouter_model_to_model_info)
                .collect(),
            etag,
        ))
    }
}

#[async_trait]
impl ModelsEndpointClient for OpenAiModelsEndpoint {
    fn has_command_auth(&self) -> bool {
        self.provider_info.has_command_auth()
    }

    fn uses_bundled_catalog(&self) -> bool {
        self.provider_info.is_openai()
    }

    async fn uses_codex_backend(&self) -> bool {
        self.auth()
            .await
            .as_ref()
            .is_some_and(CodexAuth::uses_codex_backend)
    }

    async fn supports_remote_refresh(&self) -> bool {
        !self.provider_info.requires_openai_auth || self.auth().await.is_some()
    }

    async fn cache_identity(&self) -> ModelsCacheProviderIdentity {
        self.provider_identity()
            .await
            .unwrap_or_else(|_| ModelsCacheProviderIdentity {
                name: self.provider_info.name.clone(),
                base_url: self
                    .provider_info
                    .base_url
                    .clone()
                    .unwrap_or_default()
                    .trim_end_matches('/')
                    .to_string(),
            })
    }

    async fn list_models(
        &self,
        client_version: &str,
    ) -> CoreResult<(Vec<ModelInfo>, Option<String>)> {
        let _timer =
            codex_otel::start_global_timer("codex.remote_models.fetch_update.duration_ms", &[]);
        self.list_openrouter_models(client_version).await
    }
}

#[derive(Debug, Deserialize)]
struct OpenRouterModelsResponse {
    data: Vec<OpenRouterModel>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterModel {
    id: String,
    name: Option<String>,
    description: Option<String>,
    context_length: Option<i64>,
    architecture: Option<OpenRouterArchitecture>,
    supported_parameters: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterArchitecture {
    input_modalities: Option<Vec<String>>,
}

fn openrouter_model_to_model_info(model: OpenRouterModel) -> ModelInfo {
    let OpenRouterModel {
        id,
        name,
        description,
        context_length,
        architecture,
        supported_parameters,
    } = model;
    let mut fallback = model_info_from_slug(&id);
    fallback.slug = id.clone();
    fallback.display_name = name.unwrap_or_else(|| id.clone());
    fallback.description = description;
    fallback.visibility = ModelVisibility::List;
    fallback.supported_in_api = true;
    fallback.priority = 100;
    fallback.context_window = context_length;
    fallback.max_context_window = context_length;
    fallback.auto_compact_token_limit = None;
    fallback.supports_parallel_tool_calls = supported_parameters
        .as_ref()
        .is_some_and(|params| params.iter().any(|param| param == "tools"));
    fallback.supports_search_tool = supported_parameters
        .as_ref()
        .is_some_and(|params| params.iter().any(|param| param == "web_search"));
    fallback.supported_reasoning_levels = if supported_parameters
        .as_ref()
        .is_some_and(|params| params.iter().any(|param| param == "reasoning"))
    {
        vec![
            ReasoningEffortPreset {
                effort: ReasoningEffort::Low,
                description: "Low".to_string(),
            },
            ReasoningEffortPreset {
                effort: ReasoningEffort::Medium,
                description: "Medium".to_string(),
            },
            ReasoningEffortPreset {
                effort: ReasoningEffort::High,
                description: "High".to_string(),
            },
        ]
    } else {
        Vec::new()
    };
    fallback.default_reasoning_level = fallback
        .supported_reasoning_levels
        .iter()
        .find(|preset| preset.effort == ReasoningEffort::Medium)
        .map(|preset| preset.effort);
    if let Some(architecture) = architecture
        && let Some(input_modalities) = architecture.input_modalities
    {
        let mapped_modalities: Vec<InputModality> = input_modalities
            .into_iter()
            .filter_map(|modality| match modality.as_str() {
                "text" => Some(InputModality::Text),
                "image" => Some(InputModality::Image),
                _ => None,
            })
            .collect();
        if !mapped_modalities.is_empty() {
            fallback.input_modalities = mapped_modalities;
        }
    }
    fallback.used_fallback_model_metadata = false;
    fallback
}

#[derive(Clone)]
struct ModelsRequestTelemetry {
    auth_mode: Option<String>,
    auth_header_attached: bool,
    auth_header_name: Option<&'static str>,
    auth_env: AuthEnvTelemetry,
}

impl RequestTelemetry for ModelsRequestTelemetry {
    fn on_request(
        &self,
        attempt: u64,
        status: Option<http::StatusCode>,
        error: Option<&TransportError>,
        duration: Duration,
    ) {
        let success = status.is_some_and(|code| code.is_success()) && error.is_none();
        let error_message = error.map(telemetry_transport_error_message);
        let response_debug = error
            .map(extract_response_debug_context)
            .unwrap_or_default();
        let status = status.map(|status| status.as_u16());
        tracing::event!(
            target: "codex_otel.log_only",
            tracing::Level::INFO,
            event.name = "codex.api_request",
            duration_ms = %duration.as_millis(),
            http.response.status_code = status,
            success = success,
            error.message = error_message.as_deref(),
            attempt = attempt,
            endpoint = MODELS_ENDPOINT,
            auth.header_attached = self.auth_header_attached,
            auth.header_name = self.auth_header_name,
            auth.env_openai_api_key_present = self.auth_env.openai_api_key_env_present,
            auth.env_codex_api_key_present = self.auth_env.codex_api_key_env_present,
            auth.env_codex_api_key_enabled = self.auth_env.codex_api_key_env_enabled,
            auth.env_provider_key_name = self.auth_env.provider_env_key_name.as_deref(),
            auth.env_provider_key_present = self.auth_env.provider_env_key_present,
            auth.env_refresh_token_url_override_present = self.auth_env.refresh_token_url_override_present,
            auth.request_id = response_debug.request_id.as_deref(),
            auth.cf_ray = response_debug.cf_ray.as_deref(),
            auth.error = response_debug.auth_error.as_deref(),
            auth.error_code = response_debug.auth_error_code.as_deref(),
            auth.mode = self.auth_mode.as_deref(),
        );
        tracing::event!(
            target: "codex_otel.trace_safe",
            tracing::Level::INFO,
            event.name = "codex.api_request",
            duration_ms = %duration.as_millis(),
            http.response.status_code = status,
            success = success,
            error.message = error_message.as_deref(),
            attempt = attempt,
            endpoint = MODELS_ENDPOINT,
            auth.header_attached = self.auth_header_attached,
            auth.header_name = self.auth_header_name,
            auth.env_openai_api_key_present = self.auth_env.openai_api_key_env_present,
            auth.env_codex_api_key_present = self.auth_env.codex_api_key_env_present,
            auth.env_codex_api_key_enabled = self.auth_env.codex_api_key_env_enabled,
            auth.env_provider_key_name = self.auth_env.provider_env_key_name.as_deref(),
            auth.env_provider_key_present = self.auth_env.provider_env_key_present,
            auth.env_refresh_token_url_override_present = self.auth_env.refresh_token_url_override_present,
            auth.request_id = response_debug.request_id.as_deref(),
            auth.cf_ray = response_debug.cf_ray.as_deref(),
            auth.error = response_debug.auth_error.as_deref(),
            auth.error_code = response_debug.auth_error_code.as_deref(),
            auth.mode = self.auth_mode.as_deref(),
        );
        emit_feedback_request_tags_with_auth_env(
            &FeedbackRequestTags {
                endpoint: MODELS_ENDPOINT,
                auth_header_attached: self.auth_header_attached,
                auth_header_name: self.auth_header_name,
                auth_mode: self.auth_mode.as_deref(),
                auth_retry_after_unauthorized: None,
                auth_recovery_mode: None,
                auth_recovery_phase: None,
                auth_connection_reused: None,
                auth_request_id: response_debug.request_id.as_deref(),
                auth_cf_ray: response_debug.cf_ray.as_deref(),
                auth_error: response_debug.auth_error.as_deref(),
                auth_error_code: response_debug.auth_error_code.as_deref(),
                auth_recovery_followup_success: None,
                auth_recovery_followup_status: None,
            },
            &self.auth_env,
        );
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU64;

    use super::*;
    use codex_protocol::config_types::ModelProviderAuthInfo;

    fn provider_info_with_command_auth() -> ModelProviderInfo {
        ModelProviderInfo {
            auth: Some(ModelProviderAuthInfo {
                command: "print-token".to_string(),
                args: Vec::new(),
                timeout_ms: NonZeroU64::new(5_000).expect("timeout should be non-zero"),
                refresh_interval_ms: 300_000,
                cwd: std::env::current_dir()
                    .expect("current dir should be available")
                    .try_into()
                    .expect("current dir should be absolute"),
            }),
            requires_openai_auth: false,
            ..ModelProviderInfo::create_openai_provider(/*base_url*/ None)
        }
    }

    #[test]
    fn command_auth_provider_reports_command_auth_without_cached_auth() {
        let endpoint = OpenAiModelsEndpoint::new(
            provider_info_with_command_auth(),
            /*auth_manager*/ None,
        );

        assert!(endpoint.has_command_auth());
    }

    #[test]
    fn provider_without_command_auth_reports_no_command_auth() {
        let endpoint = OpenAiModelsEndpoint::new(
            ModelProviderInfo::create_openai_provider(/*base_url*/ None),
            /*auth_manager*/ None,
        );

        assert!(!endpoint.has_command_auth());
    }
}
