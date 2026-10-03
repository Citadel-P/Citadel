use crate::api::resources::capabilities::ResourceCapabilitiesView;
use citadel_builds::permissions::*;
use citadel_primitives::{EffectivePermission, PermissionPolicy};

pub(crate) fn pool_capabilities(permission: EffectivePermission) -> ResourceCapabilitiesView {
    ResourceCapabilitiesView {
        can_read: permission.allows(ReadBuildAgentPool::REQUIREMENT),
        can_write: permission.allows(WriteBuildAgentPool::REQUIREMENT),
        can_execute: permission.allows(ExecuteBuildAgentPool::REQUIREMENT),
    }
}

pub(crate) fn project_capabilities(permission: EffectivePermission) -> ResourceCapabilitiesView {
    ResourceCapabilitiesView {
        can_read: permission.allows(ReadBuild::REQUIREMENT),
        can_write: permission.allows(WriteBuild::REQUIREMENT),
        can_execute: permission.allows(ExecuteBuild::REQUIREMENT),
    }
}

pub(crate) fn granted(level: citadel_primitives::PermissionLevel) -> EffectivePermission {
    EffectivePermission::Granted {
        level,
        specifics: citadel_primitives::SpecificPermissions::EMPTY,
    }
}

#[cfg(test)]
mod tests {
    use crate::api::resources::builds::capabilities::*;
    use citadel_primitives::PermissionLevel;

    #[test]
    fn capability_hierarchy_is_ordinal_and_admin_is_explicit() {
        for (level, expected) in [
            (PermissionLevel::None, (false, false, false)),
            (PermissionLevel::Read, (true, false, false)),
            (PermissionLevel::Write, (true, true, false)),
            (PermissionLevel::Execute, (true, true, true)),
        ] {
            for view in [
                project_capabilities(granted(level)),
                pool_capabilities(granted(level)),
            ] {
                assert_eq!((view.can_read, view.can_write, view.can_execute), expected);
            }
        }
        let admin = project_capabilities(EffectivePermission::Administrator);
        assert!(admin.can_read && admin.can_write && admin.can_execute);
        for invalid in [-1, 3, 7, 127] {
            assert!(PermissionLevel::from_i32(invalid).is_none());
        }
    }
}
