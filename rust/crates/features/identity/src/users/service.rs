use super::*;

pub(super) const MAXIMUM_USER_NAME_CHARACTERS: usize = 140;

pub(super) const MINIMUM_SEARCH_CHARACTERS: usize = 2;

pub(super) const MAXIMUM_PAGE_SIZE: i64 = 500;

pub(super) const MAXIMUM_SEARCH_RESULTS: i64 = 50;

#[derive(Clone)]
pub struct UserReadService {
    store: Arc<dyn UserReader>,
}

impl UserReadService {
    #[must_use]
    pub fn new(store: Arc<dyn UserReader>) -> Self {
        Self { store }
    }

    pub async fn list(
        &self,
        page: i64,
        page_size: i64,
        name: Option<&str>,
    ) -> Result<PagedResult<UserDetails>, IdentityError> {
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
        if name.is_some_and(|value| value.chars().count() > MAXIMUM_USER_NAME_CHARACTERS) {
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

    pub async fn get(&self, id: Uuid) -> Result<UserDetails, IdentityError> {
        self.store.get(id).await?.ok_or(IdentityError::NotFound)
    }

    pub async fn search(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<UserSearchItemDetails>, IdentityError> {
        let query = query.trim();
        if !(MINIMUM_SEARCH_CHARACTERS..=MAXIMUM_USER_NAME_CHARACTERS)
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

pub struct UserMutationService {
    store: Arc<dyn UserRepository>,
    password_hasher: Arc<dyn PasswordHasher>,
    password_policy: PasswordPolicy,
    entitlements: Arc<dyn EntitlementService>,
    clock: Arc<dyn Clock>,
}

impl UserMutationService {
    #[must_use]
    pub fn new(
        store: Arc<dyn UserRepository>,
        password_hasher: Arc<dyn PasswordHasher>,
        entitlements: Arc<dyn EntitlementService>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            store,
            password_hasher,
            password_policy: PasswordPolicy::default(),
            entitlements,
            clock,
        }
    }

    #[must_use]
    pub fn with_password_policy(mut self, policy: PasswordPolicy) -> Self {
        self.password_policy = policy;
        self
    }

    pub async fn create(
        &self,
        request: CreateUser,
        actor_id: ActorId,
    ) -> Result<UserDetails, IdentityError> {
        validate_name(&request.name)?;
        validate_email(&request.email)?;
        self.password_policy.validate(
            &request.password,
            Some(&request.name),
            Some(&request.email),
        )?;
        let resource_accesses = validate_user_resource_accesses(request.resource_accesses)?;
        let user = NewUserMutation {
            id: Uuid::now_v7(),
            actor_id: ActorId::new(Uuid::now_v7()),
            name: request.name.trim().to_owned(),
            email: request.email.trim().to_owned(),
            password_hash: self.password_hasher.hash(&request.password)?,
            is_enabled: request.is_enabled,
            created_by_actor_id: actor_id,
            created_at: self.clock.now(),
            team_ids: deduplicate_ids(request.team_ids)?,
            role_ids: deduplicate_ids(request.role_ids)?,
            resource_accesses,
        };
        self.store
            .create(
                &user,
                self.entitlements.custom_access_control_enabled().await?,
            )
            .await
    }

    pub async fn patch(
        &self,
        id: Uuid,
        request: PatchUser,
        actor_id: ActorId,
    ) -> Result<UserDetails, IdentityError> {
        let email = patch_value(request.email);
        if let Some(email) = &email {
            validate_email(email)?;
        }
        let password = patch_value(request.password);
        if let Some(password) = &password {
            let current = self
                .store
                .password_context(id)
                .await?
                .ok_or(IdentityError::NotFound)?;
            self.password_policy.validate(
                password,
                Some(&current.name),
                Some(email.as_deref().unwrap_or(&current.email)),
            )?;
        }
        let patch = UserPatchMutation {
            email: email.map(|value| value.trim().to_owned()),
            password_hash: password
                .as_deref()
                .map(|value| self.password_hasher.hash(value))
                .transpose()?,
            is_enabled: patch_value(request.is_enabled),
            team_ids: patch_value(request.team_ids)
                .map(deduplicate_ids)
                .transpose()?,
            role_ids: patch_value(request.role_ids)
                .map(deduplicate_ids)
                .transpose()?,
            resource_accesses: patch_value(request.resource_accesses)
                .map(validate_user_resource_accesses)
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
    ) -> Result<UserDetails, IdentityError> {
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
    ) -> Result<UserDetails, IdentityError> {
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
    ) -> Result<UserDetails, IdentityError> {
        validate_id(role_id, "Role ID")?;
        self.store
            .remove_role(id, role_id, actor_id, self.clock.now())
            .await
    }

    pub async fn add_resource_access(
        &self,
        id: Uuid,
        request: ResourceAccessInput,
        actor_id: ActorId,
    ) -> Result<UserDetails, IdentityError> {
        let access = validate_user_resource_accesses(vec![request])?
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
        request: ResourceAccessInput,
        actor_id: ActorId,
    ) -> Result<UserDetails, IdentityError> {
        let access = validate_user_resource_accesses(vec![request])?
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
                "Ids must not be empty and must contain valid User IDs.".to_owned(),
            ));
        }
        self.store.delete(&ids, actor_id, self.clock.now()).await
    }
}

pub(super) const fn enabled_by_default() -> bool {
    true
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

pub(super) fn validate_user_resource_accesses(
    accesses: Vec<ResourceAccessInput>,
) -> Result<Vec<ResourceAccessInput>, IdentityError> {
    crate::resource_access::validate_resource_accesses("User", accesses)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[derive(Default)]
    struct RecordingStore {
        list_call: Mutex<Option<(Option<String>, i64, i64)>>,
        search_call: Mutex<Option<(String, i64)>>,
    }

    impl UserReader for RecordingStore {
        fn list<'a>(
            &'a self,
            name: Option<&'a str>,
            limit: i64,
            offset: i64,
        ) -> BoxFuture<'a, Result<StoredPage<UserDetails>, IdentityError>> {
            Box::pin(async move {
                *self.list_call.lock().unwrap() = Some((name.map(str::to_owned), limit, offset));
                Ok(StoredPage {
                    total_items: 1,
                    items: vec![user()],
                })
            })
        }

        fn get(&self, _id: Uuid) -> BoxFuture<'_, Result<Option<UserDetails>, IdentityError>> {
            Box::pin(async { Ok(None) })
        }

        fn search<'a>(
            &'a self,
            query: &'a str,
            limit: i64,
        ) -> BoxFuture<'a, Result<Vec<UserSearchItemDetails>, IdentityError>> {
            Box::pin(async move {
                *self.search_call.lock().unwrap() = Some((query.to_owned(), limit));
                Ok(vec![UserSearchItemDetails {
                    id: Uuid::nil(),
                    name: "owner".to_owned(),
                    email: "owner@example.test".to_owned(),
                }])
            })
        }
    }

    #[tokio::test]
    async fn list_trims_filters_and_calculates_a_bounded_offset() {
        let store = Arc::new(RecordingStore::default());
        let service = UserReadService::new(store.clone());

        let page = service.list(3, 25, Some(" owner ")).await.unwrap();

        assert_eq!(page.page, 3);
        assert_eq!(page.page_size, 25);
        assert_eq!(page.total_count, 1);
        assert_eq!(
            *store.list_call.lock().unwrap(),
            Some((Some("owner".to_owned()), 25, 50))
        );
    }

    #[tokio::test]
    async fn list_rejects_invalid_bounds_before_reading_storage() {
        let store = Arc::new(RecordingStore::default());
        let service = UserReadService::new(store.clone());

        for result in [
            service.list(0, 50, None).await,
            service.list(1, 0, None).await,
            service.list(1, 501, None).await,
            service.list(i64::MAX, 500, None).await,
            service.list(1, 50, Some(&"x".repeat(141))).await,
        ] {
            assert!(matches!(result, Err(IdentityError::Validation(_))));
        }
        assert!(store.list_call.lock().unwrap().is_none());
    }

    #[tokio::test]
    async fn search_trims_the_query_and_enforces_dotnet_bounds() {
        let store = Arc::new(RecordingStore::default());
        let service = UserReadService::new(store.clone());

        let users = service.search(" owner ", 20).await.unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(
            *store.search_call.lock().unwrap(),
            Some(("owner".to_owned(), 20))
        );
        for result in [
            service.search("x", 20).await,
            service.search(&"x".repeat(141), 20).await,
            service.search("owner", 0).await,
            service.search("owner", 51).await,
        ] {
            assert!(matches!(result, Err(IdentityError::Validation(_))));
        }
    }

    #[tokio::test]
    async fn get_maps_a_missing_projection_to_not_found() {
        let service = UserReadService::new(Arc::new(RecordingStore::default()));
        assert!(matches!(
            service.get(Uuid::now_v7()).await,
            Err(IdentityError::NotFound)
        ));
    }

    fn user() -> UserDetails {
        UserDetails {
            id: Uuid::nil(),
            name: "owner".to_owned(),
            email: "owner@example.test".to_owned(),
            actor_id: ActorId::new(Uuid::nil()),
            is_enabled: true,
            teams: Some(Vec::new()),
            roles: Some(Vec::new()),
            resource_accesses: None,
        }
    }

    #[test]
    fn mutation_assignment_ids_are_deduplicated_and_reject_empty_ids() {
        let id = Uuid::now_v7();
        assert_eq!(deduplicate_ids(vec![id, id]).unwrap(), vec![id]);
        assert!(matches!(
            deduplicate_ids(vec![Uuid::nil()]),
            Err(IdentityError::Validation(_))
        ));
    }

    #[test]
    fn mutation_resource_access_validation_rejects_duplicates_and_invalid_specifics() {
        let resource_id = Uuid::now_v7();
        let valid = ResourceAccessInput {
            resource_type: ResourceType::Deployment,
            resource_id,
            permission_level: PermissionLevel::Read,
            specific_permissions: vec![SpecificPermission::Logs],
        };
        assert!(validate_user_resource_accesses(vec![valid.clone()]).is_ok());
        assert!(matches!(
            validate_user_resource_accesses(vec![valid.clone(), valid]),
            Err(IdentityError::Validation(_))
        ));
        assert!(matches!(
            validate_user_resource_accesses(vec![ResourceAccessInput {
                resource_type: ResourceType::User,
                resource_id,
                permission_level: PermissionLevel::Read,
                specific_permissions: vec![SpecificPermission::Logs],
            }]),
            Err(IdentityError::Validation(_))
        ));
    }

    #[test]
    fn merge_patch_null_fields_preserve_existing_user_values() {
        assert_eq!(patch_value::<String>(PatchField::Missing), None);
        assert_eq!(patch_value::<String>(PatchField::Null), None);
        assert_eq!(
            patch_value(PatchField::Value("new@example.test".to_owned())),
            Some("new@example.test".to_owned())
        );
    }
}
