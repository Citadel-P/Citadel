use super::views::DeploymentCapabilities;
use citadel_deployments::permissions::*;
use citadel_domain::{EffectivePermission, PermissionPolicy, SpecificPermission};

pub(super) fn capabilities(permission: EffectivePermission) -> DeploymentCapabilities {
    DeploymentCapabilities {
        can_read: permission.allows(ReadDeployment::REQUIREMENT),
        can_write: permission.allows(WriteDeployment::REQUIREMENT),
        can_execute: permission.allows(DeleteDeployment::REQUIREMENT),
        can_view_logs: permission.allows(ViewDeploymentLogs::REQUIREMENT),
        can_inspect: permission.allows(InspectDeployment::REQUIREMENT),
        can_apply: permission.allows(ApplyDeployment::REQUIREMENT),
        // Legacy wire capability; Deployment has no Pull operation in the policy matrix.
        can_pull: match permission {
            EffectivePermission::Administrator => true,
            EffectivePermission::Granted { specifics, .. } => {
                specifics.contains(SpecificPermission::Pull)
            }
        },
        can_open_terminal: permission.allows(OpenDeploymentTerminal::REQUIREMENT),
        can_view_resource_bindings: permission.allows(ReadDeploymentBindings::REQUIREMENT),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deployment_policy_parity_and_capabilities() {
        let matrix = citadel_identity::permission_matrix();
        let capability = matrix
            .get(&citadel_domain::ResourceType::Deployment)
            .unwrap();
        for requirement in [
            ApplyDeployment::REQUIREMENT,
            ViewDeploymentLogs::REQUIREMENT,
            InspectDeployment::REQUIREMENT,
            OpenDeploymentTerminal::REQUIREMENT,
            ReadDeploymentBindings::REQUIREMENT,
        ] {
            assert!(
                capability
                    .specifics
                    .contains(&(requirement.specific.unwrap(), requirement.level))
            );
        }
        assert_eq!(
            ApplyDeployment::REQUIREMENT.level,
            citadel_domain::PermissionLevel::Read
        );
        let read_apply = EffectivePermission::Granted {
            level: citadel_domain::PermissionLevel::Read,
            specifics: SpecificPermission::Apply.into(),
        };
        let view = capabilities(read_apply);
        assert!(view.can_apply && view.can_read);
        assert!(!view.can_execute && !view.can_write && !view.can_view_logs);
        assert!(
            !capabilities(EffectivePermission::Granted {
                level: citadel_domain::PermissionLevel::None,
                specifics: SpecificPermission::Apply.into()
            })
            .can_apply
        );
        assert!(
            !capabilities(EffectivePermission::Granted {
                level: citadel_domain::PermissionLevel::Execute,
                specifics: citadel_domain::SpecificPermissions::EMPTY
            })
            .can_apply
        );
        let admin = capabilities(EffectivePermission::Administrator);
        assert!(
            admin.can_read
                && admin.can_write
                && admin.can_execute
                && admin.can_apply
                && admin.can_pull
                && admin.can_inspect
                && admin.can_open_terminal
                && admin.can_view_logs
                && admin.can_view_resource_bindings
        );
    }
}
