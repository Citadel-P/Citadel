use citadel_activities::IdentityResourceAccessSnapshot;
use citadel_identity::{IdentityError, ResourceAccessDetails, ResourceAccessInput};
use citadel_primitives::{PermissionLevel, ResourceType, SpecificPermission};
use serde::Deserialize;
use uuid::Uuid;

pub(super) fn map_accesses(
    value: serde_json::Value,
) -> Result<Vec<ResourceAccessDetails>, IdentityError> {
    serde_json::from_value::<Vec<PersistedAccess>>(value)
        .map_err(|error| IdentityError::Storage(error.to_string()))?
        .into_iter()
        .map(|access| {
            let resource_type = ResourceType::from_i32(access.resource_type).ok_or_else(|| {
                IdentityError::Storage(format!(
                    "unknown persisted ResourceType value {}",
                    access.resource_type
                ))
            })?;
            let permission_level =
                PermissionLevel::from_i32(access.permission_level).ok_or_else(|| {
                    IdentityError::Storage(format!(
                        "unknown persisted PermissionLevel value {}",
                        access.permission_level
                    ))
                })?;
            Ok(ResourceAccessDetails {
                resource_type,
                resource_id: access.resource_id,
                resource_name: access.resource_name,
                permission_level,
                specific_permissions: Some(
                    SpecificPermission::ALL
                        .into_iter()
                        .filter(|permission| access.specific_permissions & *permission as i32 != 0)
                        .collect(),
                ),
                id: Some(access.id),
            })
        })
        .collect()
}

pub(super) fn expands_resource_access(
    current: &[IdentityResourceAccessSnapshot],
    proposed: &[ResourceAccessInput],
) -> bool {
    proposed.iter().any(|access| {
        !current.iter().any(|existing| {
            existing.resource_type == access.resource_type
                && existing.resource_id == access.resource_id
                && existing.permission_level == access.permission_level
                && existing.specific_permissions == specific_mask(&access.specific_permissions)
        })
    })
}

pub(super) fn specific_mask(permissions: &[SpecificPermission]) -> i32 {
    permissions
        .iter()
        .fold(0, |mask, permission| mask | *permission as i32)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedAccess {
    id: Uuid,
    resource_type: i32,
    resource_id: Uuid,
    resource_name: Option<String>,
    permission_level: i32,
    specific_permissions: i32,
}
