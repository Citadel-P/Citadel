//! Operation policies shared by API authorization and capability projection.
use citadel_domain::{PermissionLevel, ResourceType, permission_policy};
permission_policy!(ReadBuild, ResourceType::Build, PermissionLevel::Read);
permission_policy!(WriteBuild, ResourceType::Build, PermissionLevel::Write);
permission_policy!(ExecuteBuild, ResourceType::Build, PermissionLevel::Execute);
permission_policy!(
    ReadBuildAgentPool,
    ResourceType::BuildAgentPool,
    PermissionLevel::Read
);
permission_policy!(
    WriteBuildAgentPool,
    ResourceType::BuildAgentPool,
    PermissionLevel::Write
);
permission_policy!(
    ExecuteBuildAgentPool,
    ResourceType::BuildAgentPool,
    PermissionLevel::Execute
);
