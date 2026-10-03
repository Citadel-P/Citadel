use super::*;

pub(super) const MAXIMUM_TEAM_NAME_CHARACTERS: usize = 140;

pub(super) const MINIMUM_SEARCH_CHARACTERS: usize = 2;

pub(super) const MAXIMUM_PAGE_SIZE: i64 = 500;

pub(super) const MAXIMUM_SEARCH_RESULTS: i64 = 50;

#[derive(Clone)]
pub struct TeamReadService {
    store: Arc<dyn TeamReader>,
}

impl TeamReadService {
    #[must_use]
    pub fn new(store: Arc<dyn TeamReader>) -> Self {
        Self { store }
    }

    pub async fn list(
        &self,
        page: i64,
        page_size: i64,
        name: Option<&str>,
    ) -> Result<PagedResult<TeamDetails>, IdentityError> {
        if page < 1 {
            return Err(IdentityError::Validation(
                "Page must be greater than zero.".to_owned(),
            ));
        }
        if !(1..=MAXIMUM_PAGE_SIZE).contains(&page_size) {
            return Err(IdentityError::Validation(
                "Page size must be between 1 and 500.".to_owned(),
            ));
        }
        let name = name.map(str::trim).filter(|value| !value.is_empty());
        if name.is_some_and(|value| value.chars().count() > MAXIMUM_TEAM_NAME_CHARACTERS) {
            return Err(IdentityError::Validation(
                "Name must not exceed 140 characters.".to_owned(),
            ));
        }
        let offset = page
            .checked_sub(1)
            .and_then(|value| value.checked_mul(page_size))
            .ok_or_else(|| IdentityError::Validation("Page is too large.".to_owned()))?;
        let stored = self.store.list(name, page_size, offset).await?;
        Ok(PagedResult {
            items: stored.items,
            total_count: stored.total_items,
            page,
            page_size,
        })
    }

    pub async fn get(&self, id: Uuid) -> Result<TeamDetails, IdentityError> {
        self.store.get(id).await?.ok_or(IdentityError::NotFound)
    }

    pub async fn search(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<TeamSearchItemDetails>, IdentityError> {
        let query = query.trim();
        if !(MINIMUM_SEARCH_CHARACTERS..=MAXIMUM_TEAM_NAME_CHARACTERS)
            .contains(&query.chars().count())
        {
            return Err(IdentityError::Validation(
                "Search query must contain between 2 and 140 characters.".to_owned(),
            ));
        }
        if !(1..=MAXIMUM_SEARCH_RESULTS).contains(&limit) {
            return Err(IdentityError::Validation(
                "Search limit must be between 1 and 50.".to_owned(),
            ));
        }
        self.store.search(query, limit).await
    }
}

pub struct TeamMutationService {
    store: Arc<dyn TeamRepository>,
    entitlements: Arc<dyn EntitlementService>,
    clock: Arc<dyn Clock>,
}

impl TeamMutationService {
    #[must_use]
    pub fn new(
        store: Arc<dyn TeamRepository>,
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
        request: CreateTeam,
        actor_id: ActorId,
    ) -> Result<TeamDetails, IdentityError> {
        validate_name(&request.name)?;
        let team = NewTeamMutation {
            id: Uuid::now_v7(),
            actor_id: ActorId::new(Uuid::now_v7()),
            name: request.name.trim().to_owned(),
            created_by_actor_id: actor_id,
            created_at: self.clock.now(),
            user_ids: deduplicate_ids(request.user_ids)?,
            role_ids: deduplicate_ids(request.role_ids)?,
            resource_accesses: validate_resource_accesses(request.resource_accesses)?,
        };
        self.store
            .create(
                &team,
                self.entitlements.custom_access_control_enabled().await?,
            )
            .await
    }

    pub async fn patch(
        &self,
        id: Uuid,
        request: PatchTeam,
        actor_id: ActorId,
    ) -> Result<TeamDetails, IdentityError> {
        let patch = TeamPatchMutation {
            is_enabled: patch_value(request.is_enabled),
            user_ids: patch_value(request.user_ids)
                .map(deduplicate_ids)
                .transpose()?,
            role_ids: patch_value(request.role_ids)
                .map(deduplicate_ids)
                .transpose()?,
            resource_accesses: patch_value(request.resource_accesses)
                .map(validate_resource_accesses)
                .transpose()?,
        };
        self.store
            .patch(
                id,
                &patch,
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
    ) -> Result<TeamDetails, IdentityError> {
        validate_id(id, "Team ID")?;
        validate_name(name)?;
        self.store
            .rename(id, name.trim(), actor_id, self.clock.now())
            .await
    }

    pub async fn add_role(
        &self,
        id: Uuid,
        role_id: Uuid,
        actor_id: ActorId,
    ) -> Result<TeamDetails, IdentityError> {
        validate_id(id, "Team ID")?;
        validate_id(role_id, "Role ID")?;
        self.store
            .add_role(
                id,
                role_id,
                actor_id,
                self.clock.now(),
                self.entitlements.custom_access_control_enabled().await?,
            )
            .await
    }

    pub async fn remove_role(
        &self,
        id: Uuid,
        role_id: Uuid,
        actor_id: ActorId,
    ) -> Result<TeamDetails, IdentityError> {
        validate_id(id, "Team ID")?;
        validate_id(role_id, "Role ID")?;
        self.store
            .remove_role(id, role_id, actor_id, self.clock.now())
            .await
    }

    pub async fn add_member(
        &self,
        id: Uuid,
        member_actor_id: Uuid,
        actor_id: ActorId,
    ) -> Result<TeamDetails, IdentityError> {
        validate_id(id, "Team ID")?;
        validate_id(member_actor_id, "Member Actor ID")?;
        self.store
            .add_member(
                id,
                ActorId::new(member_actor_id),
                actor_id,
                self.clock.now(),
                self.entitlements.custom_access_control_enabled().await?,
            )
            .await
    }

    pub async fn remove_member(
        &self,
        id: Uuid,
        member_actor_id: Uuid,
        actor_id: ActorId,
    ) -> Result<TeamDetails, IdentityError> {
        validate_id(id, "Team ID")?;
        validate_id(member_actor_id, "Member Actor ID")?;
        self.store
            .remove_member(
                id,
                ActorId::new(member_actor_id),
                actor_id,
                self.clock.now(),
            )
            .await
    }

    pub async fn add_resource_access(
        &self,
        id: Uuid,
        access: ResourceAccessInput,
        actor_id: ActorId,
    ) -> Result<TeamDetails, IdentityError> {
        validate_id(id, "Team ID")?;
        let access = validate_resource_accesses(vec![access])?
            .pop()
            .expect("one validated access remains");
        self.store
            .add_resource_access(
                id,
                &access,
                actor_id,
                self.clock.now(),
                self.entitlements.custom_access_control_enabled().await?,
            )
            .await
    }

    pub async fn remove_resource_access(
        &self,
        id: Uuid,
        access: ResourceAccessInput,
        actor_id: ActorId,
    ) -> Result<TeamDetails, IdentityError> {
        validate_id(id, "Team ID")?;
        let access = validate_resource_accesses(vec![access])?
            .pop()
            .expect("one validated access remains");
        self.store
            .remove_resource_access(id, &access, actor_id, self.clock.now())
            .await
    }

    pub async fn delete(&self, mut ids: Vec<Uuid>, actor_id: ActorId) -> Result<(), IdentityError> {
        ids.sort_unstable();
        ids.dedup();
        if ids.is_empty() || ids.iter().any(Uuid::is_nil) {
            return Err(IdentityError::Validation(
                "Ids must not be empty and must contain valid Team IDs.".to_owned(),
            ));
        }
        self.store.delete(&ids, actor_id, self.clock.now()).await
    }
}

pub(super) fn patch_value<T>(field: PatchField<T>) -> Option<T> {
    match field {
        PatchField::Value(value) => Some(value),
        PatchField::Missing | PatchField::Null => None,
    }
}

pub(super) fn deduplicate_ids(mut ids: Vec<Uuid>) -> Result<Vec<Uuid>, IdentityError> {
    if ids.iter().any(Uuid::is_nil) {
        return Err(IdentityError::Validation(
            "Assignment IDs must not be empty UUIDs.".to_owned(),
        ));
    }
    ids.sort_unstable();
    ids.dedup();
    Ok(ids)
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

pub(super) fn validate_resource_accesses(
    accesses: Vec<ResourceAccessInput>,
) -> Result<Vec<ResourceAccessInput>, IdentityError> {
    crate::resource_access::validate_resource_accesses("Team", accesses)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[derive(Default)]
    struct RecordingStore {
        list_call: Mutex<Option<(Option<String>, i64, i64)>>,
    }

    impl TeamReader for RecordingStore {
        fn list<'a>(
            &'a self,
            name: Option<&'a str>,
            limit: i64,
            offset: i64,
        ) -> BoxFuture<'a, Result<StoredPage<TeamDetails>, IdentityError>> {
            Box::pin(async move {
                *self.list_call.lock().unwrap() = Some((name.map(str::to_owned), limit, offset));
                Ok(StoredPage {
                    total_items: 0,
                    items: Vec::new(),
                })
            })
        }

        fn get(&self, _id: Uuid) -> BoxFuture<'_, Result<Option<TeamDetails>, IdentityError>> {
            Box::pin(async { Ok(None) })
        }

        fn search<'a>(
            &'a self,
            _query: &'a str,
            _limit: i64,
        ) -> BoxFuture<'a, Result<Vec<TeamSearchItemDetails>, IdentityError>> {
            Box::pin(async { Ok(Vec::new()) })
        }
    }

    #[tokio::test]
    async fn list_trims_filters_and_bounds_the_offset() {
        let store = Arc::new(RecordingStore::default());
        let service = TeamReadService::new(store.clone());
        service.list(3, 25, Some(" ops ")).await.unwrap();
        assert_eq!(
            *store.list_call.lock().unwrap(),
            Some((Some("ops".to_owned()), 25, 50))
        );
        assert!(matches!(
            service.list(0, 50, None).await,
            Err(IdentityError::Validation(_))
        ));
    }

    #[test]
    fn assignment_and_access_validation_is_bounded_and_deduplicated() {
        let id = Uuid::now_v7();
        assert_eq!(deduplicate_ids(vec![id, id]).unwrap(), vec![id]);
        assert!(deduplicate_ids(vec![Uuid::nil()]).is_err());
        let access = ResourceAccessInput {
            resource_type: ResourceType::Deployment,
            resource_id: id,
            permission_level: PermissionLevel::Read,
            specific_permissions: vec![SpecificPermission::Logs],
        };
        assert!(validate_resource_accesses(vec![access.clone()]).is_ok());
        assert!(validate_resource_accesses(vec![access.clone(), access]).is_err());
    }
}
