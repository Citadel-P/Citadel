// Preserve historical read-based UI hints; logs and inspection enforce their specific policies.
use super::views::SwarmServiceCapabilities;
use citadel_primitives::{EffectivePermission, PermissionPolicy};
use citadel_swarm_services::permissions::*;
pub(super) fn capabilities(permission: EffectivePermission) -> SwarmServiceCapabilities {
    SwarmServiceCapabilities {
        can_read: permission.allows(ReadSwarmService::REQUIREMENT),
        can_write: permission.allows(WriteSwarmService::REQUIREMENT),
        can_execute: permission.allows(DeleteSwarmService::REQUIREMENT),
        can_apply: permission.allows(ApplySwarmService::REQUIREMENT),
        can_view_logs: permission.allows(ReadSwarmService::REQUIREMENT),
        can_inspect: permission.allows(ReadSwarmService::REQUIREMENT),
        can_view_resource_bindings: permission.allows(ReadSwarmServiceBindings::REQUIREMENT),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use citadel_primitives::{PermissionLevel, SpecificPermission, SpecificPermissions};

    #[test]
    fn read_apply_and_write_scale_remain_distinct() {
        let read = EffectivePermission::Granted {
            level: PermissionLevel::Read,
            specifics: SpecificPermission::Apply.into(),
        };
        let view = capabilities(read);
        assert!(view.can_read && view.can_apply && view.can_view_logs && view.can_inspect);
        assert!(!view.can_write && !view.can_execute && !view.can_view_resource_bindings);
        assert!(!read.allows(ScaleSwarmService::REQUIREMENT));
        assert!(!read.allows(InspectSwarmService::REQUIREMENT));
        let write = EffectivePermission::Granted {
            level: PermissionLevel::Write,
            specifics: SpecificPermission::Apply.into(),
        };
        assert!(write.allows(ScaleSwarmService::REQUIREMENT));
        let execute_without_apply = EffectivePermission::Granted {
            level: PermissionLevel::Execute,
            specifics: SpecificPermissions::from_bits_retain(0),
        };
        assert!(capabilities(execute_without_apply).can_execute);
        assert!(!capabilities(execute_without_apply).can_apply);
    }
}
