pub use citadel_automation::AutomationRunStatus;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
pub enum AutomationRunTrigger {
    Manual,
    Test,
    Schedule,
    Webhook,
}
