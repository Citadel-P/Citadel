use crate::api::resources::stacks::views::StackCapabilities;
use citadel_primitives::{EffectivePermission, PermissionPolicy};
use citadel_stacks::permissions::*;

pub(crate) fn capabilities(permission: EffectivePermission) -> StackCapabilities {
    StackCapabilities {
        can_read: permission.allows(ReadStack::REQUIREMENT),
        can_write: permission.allows(WriteStack::REQUIREMENT),
        can_execute: permission.allows(DeleteStack::REQUIREMENT),
        can_apply: permission.allows(ApplyStack::REQUIREMENT),
        can_view_logs: permission.allows(ViewStackLogs::REQUIREMENT),
        can_inspect: permission.allows(InspectStack::REQUIREMENT),
        can_view_resource_bindings: permission.allows(ReadStackBindings::REQUIREMENT),
        can_delete: permission.allows(DeleteStack::REQUIREMENT),
        can_open_terminal: permission.allows(OpenStackTerminal::REQUIREMENT),
        can_pull: permission.allows(PullStack::REQUIREMENT),
        can_view_releases: permission.allows(ViewStackReleases::REQUIREMENT),
    }
}

#[cfg(test)]
mod tests {
    use crate::api::resources::stacks::capabilities::*;
    use citadel_primitives::{PermissionLevel, SpecificPermission, SpecificPermissions};

    #[test]
    fn apply_and_release_permissions_preserve_the_stack_contract() {
        let read = capabilities(EffectivePermission::Granted {
            level: PermissionLevel::Read,
            specifics: SpecificPermissions::from_bits_retain(
                SpecificPermission::Apply as u32 | SpecificPermission::Releases as u32,
            ),
        });
        assert!(read.can_read && read.can_view_releases);
        assert!(!read.can_apply && !read.can_write && !read.can_delete);
        let execute = capabilities(EffectivePermission::Granted {
            level: PermissionLevel::Execute,
            specifics: SpecificPermission::Apply.into(),
        });
        assert!(execute.can_read && execute.can_write && execute.can_delete && execute.can_apply);
        assert!(!execute.can_view_releases && !execute.can_view_resource_bindings);
        let admin = capabilities(EffectivePermission::Administrator);
        assert!(admin.can_apply && admin.can_view_releases && admin.can_open_terminal);
    }
}
