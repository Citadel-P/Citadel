//! Operation policies shared by API authorization and capability projection.
use citadel_primitives::{PermissionLevel, ResourceType, SpecificPermission, permission_policy};
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

permission_policy!(
    QueueBuildRun,
    ResourceType::Build,
    PermissionLevel::Read,
    SpecificPermission::Apply
);
permission_policy!(
    CancelBuildRun,
    ResourceType::Build,
    PermissionLevel::Read,
    SpecificPermission::Apply
);

#[cfg(test)]
mod tests {
    use super::*;
    use citadel_primitives::{EffectivePermission, PermissionPolicy, SpecificPermissions};

    #[test]
    fn build_queue_and_cancel_require_both_read_and_apply() {
        for requirement in [QueueBuildRun::REQUIREMENT, CancelBuildRun::REQUIREMENT] {
            assert!(EffectivePermission::Administrator.allows(requirement));
            assert!(
                EffectivePermission::Granted {
                    level: PermissionLevel::Read,
                    specifics: SpecificPermission::Apply.into()
                }
                .allows(requirement)
            );
            assert!(
                !EffectivePermission::Granted {
                    level: PermissionLevel::Execute,
                    specifics: SpecificPermissions::EMPTY
                }
                .allows(requirement)
            );
            assert!(
                !EffectivePermission::Granted {
                    level: PermissionLevel::None,
                    specifics: SpecificPermission::Apply.into()
                }
                .allows(requirement)
            );
        }
    }
}
