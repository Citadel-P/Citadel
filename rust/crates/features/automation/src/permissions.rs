use citadel_primitives::{PermissionLevel, ResourceType, permission_policy};
permission_policy!(
    ReadAutomationAction,
    ResourceType::AutomationAction,
    PermissionLevel::Read
);
permission_policy!(
    WriteAutomationAction,
    ResourceType::AutomationAction,
    PermissionLevel::Write
);
permission_policy!(
    ExecuteAutomationAction,
    ResourceType::AutomationAction,
    PermissionLevel::Execute
);
