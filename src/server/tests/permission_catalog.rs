//! Catalog grant minimums may be lower than an operation requirement: grants combine.
use citadel_primitives::{
    EffectivePermission, PermissionLevel, PermissionPolicy, SpecificPermissions,
};

#[test]
fn stack_and_build_operation_permissions_can_be_configured_without_weakening_operations() {
    let catalog = citadel_identity::permission_matrix();
    for requirement in [
        citadel_backups::permissions::RestoreBackupPolicy::REQUIREMENT,
        citadel_stacks::permissions::OpenStackTerminal::REQUIREMENT,
        citadel_stacks::permissions::PullStack::REQUIREMENT,
        citadel_stacks::permissions::ApplyStack::REQUIREMENT,
        citadel_stacks::permissions::RollbackStack::REQUIREMENT,
        citadel_builds::permissions::QueueBuildRun::REQUIREMENT,
        citadel_builds::permissions::CancelBuildRun::REQUIREMENT,
    ] {
        let specific = requirement.specific.unwrap();
        assert!(
            catalog[&requirement.resource_type]
                .specifics
                .iter()
                .any(
                    |(allowed, minimum)| *allowed == specific && requirement.level.grants(*minimum)
                )
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
                specifics: specific.into()
            }
            .allows(requirement)
        );
        assert!(
            EffectivePermission::Granted {
                level: requirement.level,
                specifics: specific.into()
            }
            .allows(requirement)
        );
    }
    assert!(
        !EffectivePermission::Granted {
            level: PermissionLevel::Read,
            specifics: citadel_primitives::SpecificPermission::Apply.into()
        }
        .allows(citadel_stacks::permissions::ApplyStack::REQUIREMENT)
    );
}
