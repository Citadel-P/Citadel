use super::views::PlatformCapabilitiesView;
use crate::api::resources::capabilities::ResourceCapabilitiesView;
use citadel_platforms::EffectivePlatformPermission;
use citadel_primitives::{PermissionLevel, SpecificPermission};
use uuid::Uuid;
const ALL_LEVELS: i32 =
    PermissionLevel::Read as i32 | PermissionLevel::Write as i32 | PermissionLevel::Execute as i32;
const ALL_PLATFORM_SPECIFIC: i32 = SpecificPermission::Logs as i32
    | SpecificPermission::Inspect as i32
    | SpecificPermission::Pull as i32
    | SpecificPermission::Terminal as i32
    | SpecificPermission::ManageNodeAgents as i32;
pub(crate) fn resource_capabilities(
    permission: EffectivePlatformPermission,
) -> ResourceCapabilitiesView {
    let can_execute = permission.level_mask & PermissionLevel::Execute as i32 != 0;
    let can_write = can_execute || permission.level_mask & PermissionLevel::Write as i32 != 0;
    ResourceCapabilitiesView {
        can_read: can_write || permission.level_mask & PermissionLevel::Read as i32 != 0,
        can_write,
        can_execute,
    }
}

pub(crate) fn platform_capabilities(
    permission: EffectivePlatformPermission,
) -> PlatformCapabilitiesView {
    let common = resource_capabilities(permission);
    PlatformCapabilitiesView {
        can_read: common.can_read,
        can_write: common.can_write,
        can_execute: common.can_execute,
        can_view_logs: common.can_read
            && permission.specific_mask & SpecificPermission::Logs as i32 != 0,
        can_inspect: common.can_read
            && permission.specific_mask & SpecificPermission::Inspect as i32 != 0,
        can_open_terminal: common.can_read
            && permission.specific_mask & SpecificPermission::Terminal as i32 != 0,
        can_pull: common.can_read
            && permission.specific_mask & SpecificPermission::Pull as i32 != 0,
        can_manage_node_agents: common.can_execute
            && permission.specific_mask & SpecificPermission::ManageNodeAgents as i32 != 0,
    }
}

pub(crate) fn permission_for(
    permissions: &std::collections::BTreeMap<Uuid, EffectivePlatformPermission>,
    id: Uuid,
    is_administrator: bool,
) -> EffectivePlatformPermission {
    permissions.get(&id).copied().unwrap_or_else(|| {
        if is_administrator {
            EffectivePlatformPermission {
                level_mask: ALL_LEVELS,
                specific_mask: ALL_PLATFORM_SPECIFIC,
            }
        } else {
            EffectivePlatformPermission::default()
        }
    })
}
