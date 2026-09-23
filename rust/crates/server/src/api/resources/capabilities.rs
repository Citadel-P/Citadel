use citadel_identity::PermissionGrant;
use citadel_primitives::PermissionLevel;
use serde::Serialize;

#[derive(Debug, Default, Serialize, utoipa::ToSchema)]
#[schema(as = server::capabilities::ResourceCapabilities)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourceCapabilities {
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
}

impl From<PermissionGrant> for ResourceCapabilities {
    fn from(permission: PermissionGrant) -> Self {
        Self {
            can_read: permission.level.grants(PermissionLevel::Read),
            can_write: permission.level.grants(PermissionLevel::Write),
            can_execute: permission.level.grants(PermissionLevel::Execute),
        }
    }
}

impl From<Option<PermissionGrant>> for ResourceCapabilities {
    fn from(permission: Option<PermissionGrant>) -> Self {
        permission.map_or_else(Self::default, Self::from)
    }
}

#[derive(Debug, Default, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCapabilitiesView {
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
}
