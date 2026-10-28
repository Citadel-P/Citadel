use citadel_primitives::PermissionLevel;

pub(crate) fn granted(level: PermissionLevel) -> citadel_primitives::EffectivePermission {
    citadel_primitives::EffectivePermission::Granted {
        level,
        specifics: citadel_primitives::SpecificPermissions::EMPTY,
    }
}

pub(crate) fn capabilities(
    level: citadel_primitives::EffectivePermission,
) -> crate::api::resources::capabilities::ResourceCapabilitiesView {
    use citadel_primitives::PermissionPolicy;
    use citadel_scripts::permissions::*;
    crate::api::resources::capabilities::ResourceCapabilitiesView {
        can_read: level.allows(ReadScript::REQUIREMENT),
        can_write: level.allows(WriteScript::REQUIREMENT),
        can_execute: level.allows(ExecuteScript::REQUIREMENT),
    }
}

#[cfg(test)]
mod tests {
    use crate::api::resources::scripts::capabilities::*;
    use citadel_primitives::PermissionLevel;

    #[test]
    fn capability_hierarchy_is_ordinal_and_admin_is_explicit() {
        for (level, expected) in [
            (PermissionLevel::None, (false, false, false)),
            (PermissionLevel::Read, (true, false, false)),
            (PermissionLevel::Write, (true, true, false)),
            (PermissionLevel::Execute, (true, true, true)),
        ] {
            let view = capabilities(granted(level));
            assert_eq!((view.can_read, view.can_write, view.can_execute), expected);
        }
        let admin = capabilities(citadel_primitives::EffectivePermission::Administrator);
        assert!(admin.can_read && admin.can_write && admin.can_execute);
        for invalid in [-1, 3, 7, 127] {
            assert!(PermissionLevel::from_i32(invalid).is_none());
        }
    }
}
