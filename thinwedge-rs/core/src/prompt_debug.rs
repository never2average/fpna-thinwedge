use std::sync::Arc;

use thinwedge_exec_server::EnvironmentManager;
use thinwedge_exec_server::ExecServerRuntimePaths;
use thinwedge_extension_api::UserInstructionsProvider;
use thinwedge_login::AuthManager;
use thinwedge_protocol::error::Result as ThinWedgeResult;
use thinwedge_protocol::error::ThinWedgeErr;
use thinwedge_protocol::models::ResponseItem;
use thinwedge_protocol::protocol::SessionSource;
use thinwedge_protocol::user_input::UserInput;
use tokio_util::sync::CancellationToken;

use crate::config::Config;
use crate::resolve_installation_id;
use crate::session::session::Session;
use crate::session::turn::build_prompt;
use crate::session::turn::built_tools;
use crate::state_db_bridge::StateDbHandle;
use crate::thread_manager::ThreadManager;
use crate::thread_manager::thread_store_from_config;
use thinwedge_extension_api::empty_extension_registry;

/// Build the model-visible `input` list for a single debug turn.
#[doc(hidden)]
pub async fn build_prompt_input(
    mut config: Config,
    input: Vec<UserInput>,
    state_db: Option<StateDbHandle>,
    user_instructions_provider: Arc<dyn UserInstructionsProvider>,
) -> ThinWedgeResult<Vec<ResponseItem>> {
    config.ephemeral = true;

    let auth_manager =
        AuthManager::shared_from_config(&config, /*enable_thinwedge_api_key_env*/ false).await;

    let local_runtime_paths = ExecServerRuntimePaths::from_optional_paths(
        config.thinwedge_self_exe.clone(),
        config.thinwedge_linux_sandbox_exe.clone(),
    )?;

    let thread_store = thread_store_from_config(&config, state_db.clone());
    let installation_id = resolve_installation_id(&config.thinwedge_home).await?;
    let thread_manager = ThreadManager::new(
        &config,
        Arc::clone(&auth_manager),
        SessionSource::Exec,
        Arc::new(
            EnvironmentManager::from_thinwedge_home(
                config.thinwedge_home.clone(),
                Some(local_runtime_paths),
            )
            .await
            .map_err(|err| ThinWedgeErr::Fatal(err.to_string()))?,
        ),
        empty_extension_registry(),
        user_instructions_provider,
        /*analytics_events_client*/ None,
        thread_store,
        state_db.clone(),
        installation_id,
        /*attestation_provider*/ None,
    );
    let thread = thread_manager.start_thread(config).await?;

    let output =
        build_prompt_input_from_session(thread.thread.thinwedge.session.as_ref(), input).await;
    let shutdown = thread.thread.shutdown_and_wait().await;
    let _removed = thread_manager.remove_thread(&thread.thread_id).await;

    shutdown?;
    output
}

pub(crate) async fn build_prompt_input_from_session(
    sess: &Session,
    input: Vec<UserInput>,
) -> ThinWedgeResult<Vec<ResponseItem>> {
    let turn_context = sess.new_default_turn().await;
    sess.record_context_updates_and_set_reference_context_item(turn_context.as_ref())
        .await;

    if !input.is_empty() {
        let response_item = sess.response_item_from_user_input(turn_context.as_ref(), input);
        sess.record_conversation_items(turn_context.as_ref(), std::slice::from_ref(&response_item))
            .await;
    }

    let prompt_input = sess
        .clone_history()
        .await
        .for_prompt(&turn_context.model_info.input_modalities);
    let router = built_tools(sess, turn_context.as_ref(), &CancellationToken::new()).await?;
    let base_instructions = sess.get_base_instructions().await;
    let prompt = build_prompt(
        prompt_input,
        router.as_ref(),
        turn_context.as_ref(),
        base_instructions,
    );

    Ok(prompt.input)
}
