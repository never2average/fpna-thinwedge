mod auth;
mod catalog;
mod mantle;

use std::path::PathBuf;
use std::sync::Arc;

use thinwedge_api::Provider;
use thinwedge_api::SharedAuthProvider;
use thinwedge_login::AuthManager;
use thinwedge_login::ThinWedgeAuth;
use thinwedge_model_provider_info::ModelProviderAwsAuthInfo;
use thinwedge_model_provider_info::ModelProviderInfo;
use thinwedge_models_manager::collaboration_mode_presets::CollaborationModesConfig;
use thinwedge_models_manager::manager::SharedModelsManager;
use thinwedge_models_manager::manager::StaticModelsManager;
use thinwedge_protocol::account::ProviderAccount;
use thinwedge_protocol::error::Result;
use thinwedge_protocol::thinwedge_models::ModelsResponse;

use crate::provider::ModelProvider;
use crate::provider::ProviderAccountResult;
use crate::provider::ProviderAccountState;
use crate::provider::ProviderCapabilities;
use auth::resolve_provider_auth;
use auth::resolve_region;
pub(crate) use catalog::static_model_catalog;
use mantle::base_url;

/// Runtime provider for Amazon Bedrock's ThinWedge-compatible Mantle endpoint.
#[derive(Clone, Debug)]
pub(crate) struct AmazonBedrockModelProvider {
    pub(crate) info: ModelProviderInfo,
    pub(crate) aws: ModelProviderAwsAuthInfo,
}

impl AmazonBedrockModelProvider {
    pub(crate) fn new(provider_info: ModelProviderInfo) -> Self {
        let aws = provider_info
            .aws
            .clone()
            .unwrap_or(ModelProviderAwsAuthInfo {
                profile: None,
                region: None,
            });
        Self {
            info: provider_info,
            aws,
        }
    }
}

#[async_trait::async_trait]
impl ModelProvider for AmazonBedrockModelProvider {
    fn info(&self) -> &ModelProviderInfo {
        &self.info
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            namespace_tools: false,
            image_generation: false,
            web_search: false,
        }
    }

    fn auth_manager(&self) -> Option<Arc<AuthManager>> {
        None
    }

    async fn auth(&self) -> Option<ThinWedgeAuth> {
        None
    }

    fn account_state(&self) -> ProviderAccountResult {
        Ok(ProviderAccountState {
            account: Some(ProviderAccount::AmazonBedrock),
            requires_thinwedge_auth: false,
        })
    }

    async fn api_provider(&self) -> Result<Provider> {
        let region = resolve_region(&self.aws).await?;
        let mut api_provider_info = self.info.clone();
        api_provider_info.base_url = Some(base_url(&region)?);
        api_provider_info.to_api_provider(/*auth_mode*/ None)
    }

    async fn api_auth(&self) -> Result<SharedAuthProvider> {
        resolve_provider_auth(&self.aws).await
    }

    fn models_manager(
        &self,
        _thinwedge_home: PathBuf,
        config_model_catalog: Option<ModelsResponse>,
        collaboration_modes_config: CollaborationModesConfig,
    ) -> SharedModelsManager {
        Arc::new(StaticModelsManager::new(
            /*auth_manager*/ None,
            config_model_catalog.unwrap_or_else(static_model_catalog),
            collaboration_modes_config,
        ))
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn api_provider_for_bedrock_bearer_token_uses_configured_region_endpoint() {
        let region = "eu-central-1";
        let mut api_provider_info =
            ModelProviderInfo::create_amazon_bedrock_provider(/*aws*/ None);
        api_provider_info.base_url = Some(base_url(region).expect("supported region"));
        let api_provider = api_provider_info
            .to_api_provider(/*auth_mode*/ None)
            .expect("api provider should build");

        assert_eq!(
            api_provider.base_url,
            "https://bedrock-mantle.eu-central-1.api.aws/thinwedge/v1"
        );
    }

    #[test]
    fn capabilities_disable_unsupported_launch_features() {
        let provider = AmazonBedrockModelProvider::new(
            ModelProviderInfo::create_amazon_bedrock_provider(/*aws*/ None),
        );

        assert_eq!(
            provider.capabilities(),
            ProviderCapabilities {
                namespace_tools: false,
                image_generation: false,
                web_search: false,
            }
        );
    }
}
