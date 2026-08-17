use std::collections::BTreeSet;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use citadel_domain::{ActorId, PermissionLevel, ResourceType, RoleType, SpecificPermission};
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    Clock, EntitlementService, IdentityError, PatchField, permission_matrix, validate_name,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RolePermissionView {
    pub resource_type: ResourceType,
    pub permission_level: PermissionLevel,
    #[serde(default)]
    pub specific_permissions: Vec<SpecificPermission>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RolePermissionInput {
    pub resource_type: ResourceType,
    pub permission_level: PermissionLevel,
    pub specific_permissions: Option<Vec<SpecificPermission>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleView {
    pub id: Uuid,
    pub name: String,
    pub role_type: RoleType,
    pub permissions: Vec<RolePermissionView>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRoleRequest {
    pub name: String,
    pub permissions: Option<Vec<RolePermissionInput>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchRolePermissionsRequest {
    #[serde(default)]
    pub permissions: PatchField<Vec<RolePermissionInput>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameRoleRequest {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteRolesRequest {
    pub ids: Vec<Uuid>,
}

#[derive(Debug, Clone)]
pub struct NewRoleMutation {
    pub id: Uuid,
    pub name: String,
    pub permissions: Vec<RolePermissionView>,
    pub changed_by_actor_id: ActorId,
    pub changed_at: DateTime<Utc>,
}

pub trait RoleReadStore: Send + Sync {
    fn list(&self) -> BoxFuture<'_, Result<Vec<RoleView>, IdentityError>>;

    fn get(&self, id: Uuid) -> BoxFuture<'_, Result<Option<RoleView>, IdentityError>>;
}

pub trait RoleMutationStore: Send + Sync {
    fn create<'a>(
        &'a self,
        role: &'a NewRoleMutation,
    ) -> BoxFuture<'a, Result<RoleView, IdentityError>>;

    fn patch_permissions<'a>(
        &'a self,
        id: Uuid,
        permissions: Option<&'a [RolePermissionView]>,
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
        custom_access_control_enabled: bool,
    ) -> BoxFuture<'a, Result<RoleView, IdentityError>>;

    fn rename<'a>(
        &'a self,
        id: Uuid,
        name: &'a str,
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<RoleView, IdentityError>>;

    fn delete<'a>(
        &'a self,
        ids: &'a [Uuid],
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), IdentityError>>;
}

#[derive(Clone)]
pub struct RoleReadService {
    store: Arc<dyn RoleReadStore>,
}

impl RoleReadService {
    #[must_use]
    pub fn new(store: Arc<dyn RoleReadStore>) -> Self {
        Self { store }
    }

    pub async fn list(&self) -> Result<Vec<RoleView>, IdentityError> {
        self.store.list().await
    }

    pub async fn get(&self, id: Uuid) -> Result<RoleView, IdentityError> {
        self.store.get(id).await?.ok_or(IdentityError::NotFound)
    }
}

pub struct RoleMutationService {
    store: Arc<dyn RoleMutationStore>,
    entitlements: Arc<dyn EntitlementService>,
    clock: Arc<dyn Clock>,
}

impl RoleMutationService {
    #[must_use]
    pub fn new(
        store: Arc<dyn RoleMutationStore>,
        entitlements: Arc<dyn EntitlementService>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            store,
            entitlements,
            clock,
        }
    }

    pub async fn create(
        &self,
        request: CreateRoleRequest,
        actor_id: ActorId,
    ) -> Result<RoleView, IdentityError> {
        validate_name(&request.name)?;
        let permissions = request
            .permissions
            .ok_or_else(|| IdentityError::Validation("Permissions must be provided.".to_owned()))?;
        let permissions = validate_role_permissions(permissions)?;
        if !self.entitlements.custom_access_control_enabled().await? {
            return Err(IdentityError::LicenseRequired("custom-access-control"));
        }
        self.store
            .create(&NewRoleMutation {
                id: Uuid::now_v7(),
                name: request.name.trim().to_owned(),
                permissions,
                changed_by_actor_id: actor_id,
                changed_at: self.clock.now(),
            })
            .await
    }

    pub async fn patch_permissions(
        &self,
        id: Uuid,
        request: PatchRolePermissionsRequest,
        actor_id: ActorId,
    ) -> Result<RoleView, IdentityError> {
        let permissions = match request.permissions {
            PatchField::Value(permissions) => Some(validate_role_permissions(permissions)?),
            PatchField::Missing => None,
            PatchField::Null => {
                return Err(IdentityError::Validation(
                    "Permissions must not be null.".to_owned(),
                ));
            }
        };
        self.store
            .patch_permissions(
                id,
                permissions.as_deref(),
                actor_id,
                self.clock.now(),
                self.entitlements.custom_access_control_enabled().await?,
            )
            .await
    }

    pub async fn rename(
        &self,
        id: Uuid,
        name: &str,
        actor_id: ActorId,
    ) -> Result<RoleView, IdentityError> {
        validate_id(id, "Role ID")?;
        validate_name(name)?;
        self.store
            .rename(id, name.trim(), actor_id, self.clock.now())
            .await
    }

    pub async fn delete(&self, mut ids: Vec<Uuid>, actor_id: ActorId) -> Result<(), IdentityError> {
        ids.sort_unstable();
        ids.dedup();
        if ids.is_empty() || ids.iter().any(Uuid::is_nil) {
            return Err(IdentityError::Validation(
                "Ids must not be empty and must contain valid Role IDs.".to_owned(),
            ));
        }
        self.store.delete(&ids, actor_id, self.clock.now()).await
    }
}

pub fn role_permissions_expand(
    current: &[RolePermissionView],
    proposed: &[RolePermissionView],
) -> bool {
    proposed.iter().any(|candidate| {
        !current.iter().any(|existing| {
            existing.resource_type == candidate.resource_type
                && existing.permission_level.grants(candidate.permission_level)
                && candidate
                    .specific_permissions
                    .iter()
                    .all(|permission| existing.specific_permissions.contains(permission))
        })
    })
}

fn validate_role_permissions(
    permissions: Vec<RolePermissionInput>,
) -> Result<Vec<RolePermissionView>, IdentityError> {
    let matrix = permission_matrix();
    let mut resources = BTreeSet::new();
    let mut normalized = Vec::with_capacity(permissions.len());
    for permission in permissions {
        let mut specific_permissions = permission.specific_permissions.unwrap_or_default();
        specific_permissions.sort_by_key(|value| *value as i32);
        specific_permissions.dedup();
        let capability = &matrix[&permission.resource_type];
        if !resources.insert(permission.resource_type)
            || !capability.maximum_level.grants(permission.permission_level)
            || !specific_permissions.iter().all(|specific| {
                capability.specifics.iter().any(|(allowed, minimum)| {
                    allowed == specific && permission.permission_level.grants(*minimum)
                })
            })
        {
            return Err(IdentityError::Validation(
                "Invalid or duplicate Role permission.".to_owned(),
            ));
        }
        normalized.push(RolePermissionView {
            resource_type: permission.resource_type,
            permission_level: permission.permission_level,
            specific_permissions,
        });
    }
    normalized.sort_by_key(|permission| permission.resource_type);
    Ok(normalized)
}

fn validate_id(id: Uuid, field: &str) -> Result<(), IdentityError> {
    if id.is_nil() {
        Err(IdentityError::Validation(format!(
            "{field} must not be empty."
        )))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_validation_normalizes_specifics_and_rejects_duplicate_resources() {
        let normalized = validate_role_permissions(vec![RolePermissionInput {
            resource_type: ResourceType::Deployment,
            permission_level: PermissionLevel::Read,
            specific_permissions: Some(vec![SpecificPermission::Logs, SpecificPermission::Logs]),
        }])
        .unwrap();
        assert_eq!(
            normalized[0].specific_permissions,
            vec![SpecificPermission::Logs]
        );

        let duplicate = validate_role_permissions(vec![
            RolePermissionInput {
                resource_type: ResourceType::Registry,
                permission_level: PermissionLevel::Read,
                specific_permissions: None,
            },
            RolePermissionInput {
                resource_type: ResourceType::Registry,
                permission_level: PermissionLevel::Write,
                specific_permissions: None,
            },
        ]);
        assert!(matches!(duplicate, Err(IdentityError::Validation(_))));
    }

    #[test]
    fn permission_expansion_distinguishes_reduction_from_added_authority() {
        let current = vec![RolePermissionView {
            resource_type: ResourceType::Deployment,
            permission_level: PermissionLevel::Execute,
            specific_permissions: vec![SpecificPermission::Apply, SpecificPermission::Logs],
        }];
        let reduced = vec![RolePermissionView {
            resource_type: ResourceType::Deployment,
            permission_level: PermissionLevel::Read,
            specific_permissions: vec![SpecificPermission::Logs],
        }];
        let expanded = vec![RolePermissionView {
            resource_type: ResourceType::Deployment,
            permission_level: PermissionLevel::Execute,
            specific_permissions: vec![
                SpecificPermission::Apply,
                SpecificPermission::Logs,
                SpecificPermission::Terminal,
            ],
        }];

        assert!(!role_permissions_expand(&current, &reduced));
        assert!(role_permissions_expand(&current, &expanded));
    }

    #[test]
    fn invalid_permission_combinations_fail_before_storage() {
        let invalid = validate_role_permissions(vec![RolePermissionInput {
            resource_type: ResourceType::Role,
            permission_level: PermissionLevel::Read,
            specific_permissions: Some(vec![SpecificPermission::Logs]),
        }]);
        assert!(matches!(invalid, Err(IdentityError::Validation(_))));
    }
}
