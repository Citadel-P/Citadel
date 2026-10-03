use citadel_identity::PermissionGrant;
use citadel_primitives::PermissionLevel;
use serde::Serialize;

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, serde::Deserialize, utoipa::ToSchema,
)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCapabilitiesView {
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
}

impl From<PermissionGrant> for ResourceCapabilitiesView {
    fn from(permission: PermissionGrant) -> Self {
        Self {
            can_read: permission.level.grants(PermissionLevel::Read),
            can_write: permission.level.grants(PermissionLevel::Write),
            can_execute: permission.level.grants(PermissionLevel::Execute),
        }
    }
}

impl From<Option<PermissionGrant>> for ResourceCapabilitiesView {
    fn from(permission: Option<PermissionGrant>) -> Self {
        permission.map_or_else(Self::default, Self::from)
    }
}

impl From<ResourceCapabilitiesView> for citadel_identity::ResourceCapabilities {
    fn from(value: ResourceCapabilitiesView) -> Self {
        Self {
            can_read: value.can_read,
            can_write: value.can_write,
            can_execute: value.can_execute,
        }
    }
}

impl From<citadel_identity::ResourceCapabilities> for ResourceCapabilitiesView {
    fn from(value: citadel_identity::ResourceCapabilities) -> Self {
        Self {
            can_read: value.can_read,
            can_write: value.can_write,
            can_execute: value.can_execute,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use citadel_primitives::{ResourceType, SpecificPermissions};

    #[test]
    fn collection_projection_preserves_permission_levels_and_absent_grants() {
        assert_eq!(
            ResourceCapabilitiesView::from(None),
            ResourceCapabilitiesView::default()
        );
        for (level, read, write, execute) in [
            (PermissionLevel::Read, true, false, false),
            (PermissionLevel::Write, true, true, false),
            (PermissionLevel::Execute, true, true, true),
        ] {
            let view = ResourceCapabilitiesView::from(Some(PermissionGrant {
                resource_type: ResourceType::Deployment,
                level,
                specifics: SpecificPermissions::EMPTY,
            }));
            assert_eq!(
                serde_json::to_value(view).unwrap(),
                serde_json::json!({"canRead":read,"canWrite":write,"canExecute":execute})
            );
        }
    }
}
