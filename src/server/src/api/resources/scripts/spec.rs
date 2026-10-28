pub use citadel_scripts::ScriptRunStatus;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
pub enum ScriptRunTrigger {
    Manual,
    Test,
    Schedule,
    Webhook,
}

#[derive(utoipa::ToSchema)]
#[serde(tag = "kind")]
#[allow(dead_code)]
pub enum ScriptExecutorSchema {
    Core,
    Agent {
        #[serde(rename = "agentId")]
        agent_id: uuid::Uuid,
    },
    Automatic,
}
