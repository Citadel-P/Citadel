pub use citadel_automation::AutomationRunStatus;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
pub enum AutomationRunTrigger {
    Manual,
    Test,
    Schedule,
    Webhook,
}

#[derive(utoipa::ToSchema)]
#[serde(tag = "kind")]
#[allow(dead_code)]
pub enum AutomationExecutorSchema {
    Core,
    Agent {
        #[serde(rename = "agentId")]
        agent_id: uuid::Uuid,
    },
    Automatic,
}
