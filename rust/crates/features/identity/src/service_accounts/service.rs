use super::*;

pub const DEFAULT_SERVICE_ACCOUNT_TOKEN_LIFETIME_DAYS: i64 = 90;

pub const MAXIMUM_SERVICE_ACCOUNT_TOKEN_LIFETIME_DAYS: i64 = 365;

pub const MAXIMUM_ACTIVE_SERVICE_ACCOUNT_TOKENS: i64 = 10;

pub const MAXIMUM_SERVICE_ACCOUNT_DESCRIPTION_CHARS: usize = 600;

pub struct ServiceAccountService {
    store: Arc<dyn ServiceAccountRepository>,
    tokens: Arc<dyn ServiceAccountTokenCodec>,
    entitlements: Arc<dyn EntitlementService>,
    clock: Arc<dyn Clock>,
    limits: ServiceAccountLimitsDetails,
}

impl ServiceAccountService {
    #[must_use]
    pub fn new(
        store: Arc<dyn ServiceAccountRepository>,
        tokens: Arc<dyn ServiceAccountTokenCodec>,
        entitlements: Arc<dyn EntitlementService>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            store,
            tokens,
            entitlements,
            clock,
            limits: ServiceAccountLimitsDetails::default(),
        }
    }

    pub fn with_limits(
        mut self,
        limits: ServiceAccountLimitsDetails,
    ) -> Result<Self, IdentityError> {
        if limits.default_token_lifetime_days <= 0
            || limits.maximum_token_lifetime_days < limits.default_token_lifetime_days
            || limits.maximum_token_lifetime_days > 365_000
            || limits.maximum_active_tokens_per_account <= 0
        {
            return Err(IdentityError::Validation("Service-account limits require positive values, a maximum lifetime at least as long as the default, and at most 365000 days.".into()));
        }
        self.limits = limits;
        Ok(self)
    }

    pub fn limits(&self) -> ServiceAccountLimitsDetails {
        self.limits
    }

    pub async fn list(
        &self,
        actor_id: ActorId,
        is_administrator: bool,
        include_archived: bool,
        name: Option<&str>,
        page: i64,
        page_size: i64,
    ) -> Result<PagedResult<ServiceAccountDetails>, IdentityError> {
        validate_page(page, page_size)?;
        if name.is_some_and(|value| value.chars().count() > 128) {
            return Err(IdentityError::Validation(
                "Name filter cannot exceed 128 characters.".to_owned(),
            ));
        }
        let stored = self
            .store
            .list(
                actor_id,
                is_administrator,
                include_archived,
                name,
                page_size,
                (page - 1) * page_size,
            )
            .await?;
        Ok(to_page(stored, page, page_size))
    }

    pub async fn get(&self, id: Uuid) -> Result<ServiceAccountDetails, IdentityError> {
        self.store.get(id).await?.ok_or(IdentityError::NotFound)
    }

    pub async fn usages(&self, id: Uuid) -> Result<Vec<RunAsActorUsageDetails>, IdentityError> {
        let account = self.get(id).await?;
        self.store.usages(account.actor_id).await
    }

    pub async fn create(
        &self,
        request: CreateServiceAccount,
        created_by_actor_id: ActorId,
    ) -> Result<ServiceAccountDetails, IdentityError> {
        self.require_entitlement().await?;
        validate_name(&request.name)?;
        let description = normalize_description(request.description)?;
        let resource_accesses = validate_resource_accesses(request.resource_accesses)?;
        let now = self.clock.now();
        self.store
            .create(&NewServiceAccount {
                id: Uuid::now_v7(),
                actor_id: ActorId::new(Uuid::now_v7()),
                name: request.name.trim().to_owned(),
                description,
                is_enabled: request.is_enabled,
                created_by_actor_id,
                created_at: now,
                team_ids: deduplicate(request.team_ids),
                role_ids: deduplicate(request.role_ids),
                resource_accesses,
            })
            .await
    }

    pub async fn update(
        &self,
        id: Uuid,
        request: UpdateServiceAccount,
        actor_id: ActorId,
    ) -> Result<ServiceAccountDetails, IdentityError> {
        if request.is_enabled == PatchField::Value(true) {
            self.require_entitlement().await?;
        }
        let description = match request.description {
            PatchField::Missing => PatchField::Missing,
            PatchField::Null => PatchField::Null,
            PatchField::Value(description) => normalize_description(Some(description))?
                .map_or(PatchField::Null, PatchField::Value),
        };
        let is_enabled = match request.is_enabled {
            PatchField::Missing => None,
            PatchField::Value(value) => Some(value),
            PatchField::Null => {
                return Err(IdentityError::Validation(
                    "isEnabled cannot be null.".to_owned(),
                ));
            }
        };
        self.store
            .update(id, &description, is_enabled, actor_id, self.clock.now())
            .await
    }

    pub async fn rename(
        &self,
        id: Uuid,
        name: &str,
        actor_id: ActorId,
    ) -> Result<ServiceAccountDetails, IdentityError> {
        validate_name(name)?;
        self.store
            .rename(id, name.trim(), actor_id, self.clock.now())
            .await
    }

    pub async fn archive(
        &self,
        mut ids: Vec<Uuid>,
        actor_id: ActorId,
    ) -> Result<(), IdentityError> {
        ids.sort_unstable();
        ids.dedup();
        if ids.is_empty() {
            return Err(IdentityError::Validation(
                "Ids must not be empty.".to_owned(),
            ));
        }
        self.store.archive(&ids, actor_id, self.clock.now()).await
    }

    pub async fn add_role(
        &self,
        account_id: Uuid,
        role_id: Uuid,
        actor_id: ActorId,
    ) -> Result<ServiceAccountDetails, IdentityError> {
        self.require_entitlement().await?;
        self.store
            .add_role(account_id, role_id, actor_id, self.clock.now())
            .await
    }

    pub async fn remove_role(
        &self,
        account_id: Uuid,
        role_id: Uuid,
        actor_id: ActorId,
    ) -> Result<ServiceAccountDetails, IdentityError> {
        self.store
            .remove_role(account_id, role_id, actor_id, self.clock.now())
            .await
    }

    pub async fn add_resource_access(
        &self,
        account_id: Uuid,
        request: AddServiceAccountResourceAccess,
        actor_id: ActorId,
    ) -> Result<ServiceAccountDetails, IdentityError> {
        self.require_entitlement().await?;
        let access = validate_resource_accesses(vec![ServiceAccountResourceAccess {
            id: Some(Uuid::now_v7()),
            resource_type: request.resource_type,
            resource_id: request.resource_id,
            resource_name: None,
            permission_level: request.permission_level,
            specific_permissions: request.specific_permissions,
        }])?
        .pop()
        .expect("one validated resource access remains");
        self.store
            .add_resource_access(account_id, &access, actor_id, self.clock.now())
            .await
    }

    pub async fn remove_resource_access(
        &self,
        account_id: Uuid,
        resource_access_id: Uuid,
        actor_id: ActorId,
    ) -> Result<ServiceAccountDetails, IdentityError> {
        self.store
            .remove_resource_access(account_id, resource_access_id, actor_id, self.clock.now())
            .await
    }

    pub async fn list_tokens(
        &self,
        account_id: Uuid,
        page: i64,
        page_size: i64,
    ) -> Result<PagedResult<ServiceAccountTokenDetails>, IdentityError> {
        validate_page(page, page_size)?;
        let stored = self
            .store
            .list_tokens(account_id, page_size, (page - 1) * page_size)
            .await?;
        Ok(to_page(stored, page, page_size))
    }

    pub async fn create_token(
        &self,
        account_id: Uuid,
        request: CreateServiceAccountToken,
        actor_id: ActorId,
    ) -> Result<CreatedServiceAccountTokenDetails, IdentityError> {
        self.require_entitlement().await?;
        validate_name(&request.name)?;
        if request.never_expires && request.expires_at_utc.is_some() {
            return Err(IdentityError::Validation(
                "A non-expiring token cannot also have an expiration date.".to_owned(),
            ));
        }
        let now = self.clock.now();
        let expires_at = if request.never_expires {
            None
        } else {
            Some(
                request
                    .expires_at_utc
                    .unwrap_or(now + Duration::days(self.limits.default_token_lifetime_days)),
            )
        };
        if expires_at.is_some_and(|expires| expires <= now) {
            return Err(IdentityError::Validation(
                "Token expiration must be in the future.".to_owned(),
            ));
        }
        if expires_at.is_some_and(|expires| {
            expires > now + Duration::days(self.limits.maximum_token_lifetime_days)
        }) {
            return Err(IdentityError::Validation(format!(
                "Token expiration cannot exceed {} days.",
                self.limits.maximum_token_lifetime_days
            )));
        }
        let id = Uuid::now_v7();
        let (plaintext, digest) = self.tokens.issue(id)?;
        let credential = self
            .store
            .create_token(
                &NewServiceAccountToken {
                    id,
                    service_account_id: account_id,
                    name: request.name.trim().to_owned(),
                    secret_hash: digest,
                    expires_at_utc: expires_at,
                    created_by_actor_id: actor_id,
                    created_at_utc: now,
                },
                self.limits.maximum_active_tokens_per_account,
            )
            .await?;
        Ok(CreatedServiceAccountTokenDetails {
            credential,
            token: plaintext,
        })
    }

    pub async fn revoke_token(
        &self,
        account_id: Uuid,
        token_id: Uuid,
        actor_id: ActorId,
    ) -> Result<(), IdentityError> {
        self.store
            .revoke_token(account_id, token_id, actor_id, self.clock.now())
            .await
    }

    async fn require_entitlement(&self) -> Result<(), IdentityError> {
        if self.entitlements.custom_access_control_enabled().await? {
            Ok(())
        } else {
            Err(IdentityError::LicenseRequired("custom-access-control"))
        }
    }
}

pub(super) const fn enabled_by_default() -> bool {
    true
}

pub(super) fn normalize_description(
    description: Option<String>,
) -> Result<Option<String>, IdentityError> {
    let description = description.and_then(|value| {
        let value = value.trim().to_owned();
        (!value.is_empty()).then_some(value)
    });
    if description
        .as_ref()
        .is_some_and(|value| value.chars().count() > MAXIMUM_SERVICE_ACCOUNT_DESCRIPTION_CHARS)
    {
        return Err(IdentityError::Validation(format!(
            "Description cannot exceed {MAXIMUM_SERVICE_ACCOUNT_DESCRIPTION_CHARS} characters."
        )));
    }
    Ok(description)
}

pub(super) fn validate_resource_accesses(
    accesses: Vec<ServiceAccountResourceAccess>,
) -> Result<Vec<ServiceAccountResourceAccess>, IdentityError> {
    let matrix = permission_matrix();
    let mut unique = std::collections::BTreeSet::new();
    for access in &accesses {
        let capability = &matrix[&access.resource_type];
        let known_specifics = access.specific_permissions.iter().all(|permission| {
            capability.specifics.iter().any(|(allowed, minimum)| {
                allowed == permission && access.permission_level.grants(*minimum)
            })
        });
        if access.permission_level == PermissionLevel::None
            || !capability.maximum_level.grants(access.permission_level)
            || !known_specifics
            || !unique.insert((access.resource_type, access.resource_id))
        {
            return Err(IdentityError::Validation(
                "Invalid or duplicate resource access permission.".to_owned(),
            ));
        }
    }
    Ok(accesses)
}

pub(super) fn validate_page(page: i64, page_size: i64) -> Result<(), IdentityError> {
    if page < 1 || !(1..=100).contains(&page_size) {
        return Err(IdentityError::Validation(
            "Page must be positive and pageSize must be between 1 and 100.".to_owned(),
        ));
    }
    Ok(())
}

pub(super) fn deduplicate(mut ids: Vec<Uuid>) -> Vec<Uuid> {
    ids.sort_unstable();
    ids.dedup();
    ids
}

pub(super) fn to_page<T>(
    stored: StoredPage<T>,
    current_page: i64,
    page_size: i64,
) -> PagedResult<T> {
    PagedResult {
        items: stored.items,
        total_count: stored.total_items,
        page: current_page,
        page_size,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_access_validation_rejects_unknown_specifics_and_duplicates() {
        let resource_id = Uuid::now_v7();
        assert!(
            validate_resource_accesses(vec![ServiceAccountResourceAccess {
                id: None,
                resource_type: ResourceType::ServiceAccount,
                resource_id,
                resource_name: None,
                permission_level: PermissionLevel::Read,
                specific_permissions: vec![SpecificPermission::ManageCredentials],
            }])
            .is_ok()
        );
        assert!(
            validate_resource_accesses(vec![ServiceAccountResourceAccess {
                id: None,
                resource_type: ResourceType::User,
                resource_id,
                resource_name: None,
                permission_level: PermissionLevel::Read,
                specific_permissions: vec![SpecificPermission::ManageCredentials],
            }])
            .is_err()
        );
    }
}
