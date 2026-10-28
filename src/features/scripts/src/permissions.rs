use citadel_primitives::{PermissionLevel, ResourceType, permission_policy};
permission_policy!(ReadScript, ResourceType::Script, PermissionLevel::Read);
permission_policy!(WriteScript, ResourceType::Script, PermissionLevel::Write);
permission_policy!(
    ExecuteScript,
    ResourceType::Script,
    PermissionLevel::Execute
);

permission_policy!(
    ExecuteScriptAgent,
    ResourceType::Platform,
    PermissionLevel::Execute
);
