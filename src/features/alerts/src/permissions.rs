use citadel_primitives::{PermissionLevel, ResourceType, permission_policy};
permission_policy!(
    ReadAlertChannel,
    ResourceType::AlertChannel,
    PermissionLevel::Read
);
permission_policy!(
    WriteAlertChannel,
    ResourceType::AlertChannel,
    PermissionLevel::Write
);
permission_policy!(
    ExecuteAlertChannel,
    ResourceType::AlertChannel,
    PermissionLevel::Execute
);
permission_policy!(ReadAlertRule, ResourceType::Alert, PermissionLevel::Read);
permission_policy!(WriteAlertRule, ResourceType::Alert, PermissionLevel::Write);
permission_policy!(
    ExecuteAlertRule,
    ResourceType::Alert,
    PermissionLevel::Execute
);
permission_policy!(ReadAlertEvent, ResourceType::Alert, PermissionLevel::Read);
permission_policy!(WriteAlertEvent, ResourceType::Alert, PermissionLevel::Write);
permission_policy!(
    ExecuteAlertEvent,
    ResourceType::Alert,
    PermissionLevel::Execute
);
