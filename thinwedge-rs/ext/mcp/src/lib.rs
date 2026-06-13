use thinwedge_core::config::Config;
use thinwedge_extension_api::ExtensionFuture;
use thinwedge_extension_api::ExtensionRegistryBuilder;
use thinwedge_extension_api::McpServerContribution;
use thinwedge_extension_api::McpServerContributionContext;
use thinwedge_extension_api::McpServerContributor;
use thinwedge_mcp::THINWEDGE_APPS_MCP_SERVER_NAME;
use thinwedge_mcp::hosted_plugin_runtime_mcp_server_config;

struct HostedPluginRuntimeExtension;

impl McpServerContributor<Config> for HostedPluginRuntimeExtension {
    fn id(&self) -> &'static str {
        "hosted_plugin_runtime"
    }

    fn contribute<'a>(
        &'a self,
        context: McpServerContributionContext<'a, Config>,
    ) -> ExtensionFuture<'a, Vec<McpServerContribution>> {
        Box::pin(async move {
            let config = context.config();
            let name = THINWEDGE_APPS_MCP_SERVER_NAME.to_string();
            if !config.features.enabled(thinwedge_features::Feature::Apps) {
                return vec![McpServerContribution::Remove { name }];
            }

            vec![McpServerContribution::Set {
                name,
                config: Box::new(hosted_plugin_runtime_mcp_server_config(
                    &config.chatgpt_base_url,
                    config.apps_mcp_product_sku.as_deref(),
                )),
            }]
        })
    }
}

pub fn install(builder: &mut ExtensionRegistryBuilder<Config>) {
    builder.mcp_server_contributor(std::sync::Arc::new(HostedPluginRuntimeExtension));
}
