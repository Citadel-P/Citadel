//! Deployment policies shared by entry points and authoritative persistence checks.
use citadel_primitives::{PermissionLevel, ResourceType, SpecificPermission, permission_policy};

permission_policy!(
    ReadDeployment,
    ResourceType::Deployment,
    PermissionLevel::Read
);
permission_policy!(
    CreateDeployment,
    ResourceType::Deployment,
    PermissionLevel::Write
);
permission_policy!(
    WriteDeployment,
    ResourceType::Deployment,
    PermissionLevel::Write
);
permission_policy!(
    DeleteDeployment,
    ResourceType::Deployment,
    PermissionLevel::Execute
);
permission_policy!(
    ApplyDeployment,
    ResourceType::Deployment,
    PermissionLevel::Read,
    SpecificPermission::Apply
);
permission_policy!(
    ViewDeploymentLogs,
    ResourceType::Deployment,
    PermissionLevel::Read,
    SpecificPermission::Logs
);
permission_policy!(
    InspectDeployment,
    ResourceType::Deployment,
    PermissionLevel::Read,
    SpecificPermission::Inspect
);
permission_policy!(
    OpenDeploymentTerminal,
    ResourceType::Deployment,
    PermissionLevel::Read,
    SpecificPermission::Terminal
);

permission_policy!(
    ReadDeploymentBindings,
    ResourceType::Deployment,
    PermissionLevel::Read,
    SpecificPermission::ResourceBindings
);

// Copying source bindings into a new Deployment also requires destination write.
permission_policy!(
    WriteDeploymentBindings,
    ResourceType::Deployment,
    PermissionLevel::Write,
    SpecificPermission::ResourceBindings
);
