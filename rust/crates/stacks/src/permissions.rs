//! Named requirements shared by inbound and transactional workload checks.
use citadel_domain::{PermissionLevel, ResourceType, SpecificPermission, permission_policy};
// Stack Apply/rollback preserve the existing Execute + Apply contract.
permission_policy!(ReadStack, ResourceType::Stack, PermissionLevel::Read);
permission_policy!(CreateStack, ResourceType::Stack, PermissionLevel::Write);
permission_policy!(WriteStack, ResourceType::Stack, PermissionLevel::Write);
permission_policy!(DeleteStack, ResourceType::Stack, PermissionLevel::Execute);
permission_policy!(
    ApplyStack,
    ResourceType::Stack,
    PermissionLevel::Execute,
    SpecificPermission::Apply
);
permission_policy!(
    ReadStackBindings,
    ResourceType::Stack,
    PermissionLevel::Read,
    SpecificPermission::ResourceBindings
);
permission_policy!(
    WriteStackBindings,
    ResourceType::Stack,
    PermissionLevel::Write,
    SpecificPermission::ResourceBindings
);
permission_policy!(
    ViewStackReleases,
    ResourceType::Stack,
    PermissionLevel::Read,
    SpecificPermission::Releases
);
permission_policy!(
    ViewStackLogs,
    ResourceType::Stack,
    PermissionLevel::Read,
    SpecificPermission::Logs
);
permission_policy!(
    InspectStack,
    ResourceType::Stack,
    PermissionLevel::Read,
    SpecificPermission::Inspect
);
permission_policy!(
    OpenStackTerminal,
    ResourceType::Stack,
    PermissionLevel::Read,
    SpecificPermission::Terminal
);
permission_policy!(
    PullStack,
    ResourceType::Stack,
    PermissionLevel::Read,
    SpecificPermission::Pull
);
permission_policy!(
    ChangeStackState,
    ResourceType::Stack,
    PermissionLevel::Execute
);
permission_policy!(
    ReconcileStack,
    ResourceType::Stack,
    PermissionLevel::Execute
);
permission_policy!(
    RollbackStack,
    ResourceType::Stack,
    PermissionLevel::Execute,
    SpecificPermission::Apply
);
