use crate::{IdentityError, permission_matrix};
use citadel_primitives::{PermissionLevel, ResourceType, SpecificPermission};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceAccessInput {
    pub resource_type: ResourceType,
    pub resource_id: Uuid,
    pub permission_level: PermissionLevel,
    #[serde(default)]
    pub specific_permissions: Vec<SpecificPermission>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceAccessDetails {
    pub resource_type: ResourceType,
    pub resource_id: Uuid,
    pub resource_name: Option<String>,
    pub permission_level: PermissionLevel,
    pub specific_permissions: Option<Vec<SpecificPermission>>,
    pub id: Option<Uuid>,
}

pub(crate) fn validate_resource_accesses(
    owner: &str,
    accesses: Vec<ResourceAccessInput>,
) -> Result<Vec<ResourceAccessInput>, IdentityError> {
    let matrix = permission_matrix();
    let mut unique = std::collections::BTreeSet::new();
    for access in &accesses {
        if access.resource_id.is_nil() {
            return Err(IdentityError::Validation(
                "Resource ID must not be empty.".to_owned(),
            ));
        }
        let capability = &matrix[&access.resource_type];
        let specifics_are_allowed = access.specific_permissions.iter().all(|permission| {
            capability.specifics.iter().any(|(allowed, minimum)| {
                allowed == permission && access.permission_level.grants(*minimum)
            })
        });
        if access.permission_level == PermissionLevel::None
            || !capability.maximum_level.grants(access.permission_level)
            || !specifics_are_allowed
            || !unique.insert((access.resource_type, access.resource_id))
        {
            return Err(IdentityError::Validation(format!(
                "Invalid or duplicate {owner} resource access permission."
            )));
        }
    }
    Ok(accesses)
}
