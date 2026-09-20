use citadel_primitives::PermissionLevel;
pub(super) fn granted(level: PermissionLevel) -> citadel_primitives::EffectivePermission {
    citadel_primitives::EffectivePermission::Granted {
        level,
        specifics: citadel_primitives::SpecificPermissions::EMPTY,
    }
}

pub(super) fn capabilities(
    level: citadel_primitives::EffectivePermission,
) -> crate::capabilities::ResourceCapabilitiesView {
    use citadel_automation::permissions::*;
    use citadel_primitives::PermissionPolicy;
    crate::capabilities::ResourceCapabilitiesView {
        can_read: level.allows(ReadAutomationAction::REQUIREMENT),
        can_write: level.allows(WriteAutomationAction::REQUIREMENT),
        can_execute: level.allows(ExecuteAutomationAction::REQUIREMENT),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
