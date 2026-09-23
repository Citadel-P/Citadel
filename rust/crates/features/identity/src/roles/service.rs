use super::*;

#[derive(Clone)]
pub struct RoleReadService {
    store: Arc<dyn RoleReader>,
}

impl RoleReadService {
    #[must_use]
    pub fn new(store: Arc<dyn RoleReader>) -> Self {
        Self { store }
    }

    pub async fn list(&self) -> Result<Vec<RoleDetails>, IdentityError> {
        self.store.list().await
    }

    pub async fn get(&self, id: Uuid) -> Result<RoleDetails, IdentityError> {
        self.store.get(id).await?.ok_or(IdentityError::NotFound)
    }
}

pub struct RoleMutationService {
    store: Arc<dyn RoleRepository>,
    entitlements: Arc<dyn EntitlementService>,
    clock: Arc<dyn Clock>,
}

impl RoleMutationService {
    #[must_use]
    pub fn new(
        store: Arc<dyn RoleRepository>,
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
        request: CreateRole,
        actor_id: ActorId,
    ) -> Result<RoleDetails, IdentityError> {
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
        request: PatchRolePermissions,
        actor_id: ActorId,
    ) -> Result<RoleDetails, IdentityError> {
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
    ) -> Result<RoleDetails, IdentityError> {
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
    current: &[RolePermissionDetails],
    proposed: &[RolePermissionDetails],
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

pub(super) fn validate_role_permissions(
    permissions: Vec<RolePermissionInput>,
) -> Result<Vec<RolePermissionDetails>, IdentityError> {
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
        normalized.push(RolePermissionDetails {
            resource_type: permission.resource_type,
            permission_level: permission.permission_level,
            specific_permissions,
        });
    }
    normalized.sort_by_key(|permission| permission.resource_type);
    Ok(normalized)
}

pub(super) fn validate_id(id: Uuid, field: &str) -> Result<(), IdentityError> {
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
        let current = vec![RolePermissionDetails {
            resource_type: ResourceType::Deployment,
            permission_level: PermissionLevel::Execute,
            specific_permissions: vec![SpecificPermission::Apply, SpecificPermission::Logs],
        }];
        let reduced = vec![RolePermissionDetails {
            resource_type: ResourceType::Deployment,
            permission_level: PermissionLevel::Read,
            specific_permissions: vec![SpecificPermission::Logs],
        }];
        let expanded = vec![RolePermissionDetails {
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
