//! Implements the MultiAgentV2 collaboration tool surface.

use crate::agent::AgentStatus;
use crate::agent::agent_resolver::resolve_agent_target;
use crate::function_tool::FunctionCallError;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolOutput;
use crate::tools::context::ToolPayload;
use crate::tools::context::boxed_tool_output;
use crate::tools::handlers::multi_agents_common::*;
use crate::tools::handlers::parse_arguments;
use crate::tools::registry::CoreToolRuntime;
use crate::tools::registry::ToolExecutor;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value as JsonValue;
use thinwedge_protocol::AgentPath;
use thinwedge_protocol::models::ResponseInputItem;
use thinwedge_protocol::openai_models::ReasoningEffort;
use thinwedge_protocol::protocol::CollabWaitingBeginEvent;
use thinwedge_protocol::protocol::CollabWaitingEndEvent;
use thinwedge_protocol::protocol::InterAgentCommunication;
use thinwedge_protocol::protocol::SubAgentActivityEvent;
use thinwedge_protocol::protocol::SubAgentActivityKind;
use thinwedge_protocol::user_input::UserInput;
use thinwedge_tools::ToolName;

pub(crate) use followup_task::Handler as FollowupTaskHandler;
pub(crate) use interrupt_agent::Handler as InterruptAgentHandler;
pub(crate) use list_agents::Handler as ListAgentsHandler;
pub(crate) use send_message::Handler as SendMessageHandler;
pub(crate) use spawn::Handler as SpawnAgentHandler;
pub(crate) use wait::Handler as WaitAgentHandler;

mod followup_task;
mod interrupt_agent;
mod list_agents;
mod message_tool;
mod send_message;
mod spawn;
pub(crate) mod wait;

pub(super) fn communication_from_tool_message(
    author: AgentPath,
    recipient: AgentPath,
    message: String,
) -> InterAgentCommunication {
    InterAgentCommunication::new_encrypted(
        author,
        recipient,
        Vec::new(),
        message,
        /*trigger_turn*/ true,
    )
}
