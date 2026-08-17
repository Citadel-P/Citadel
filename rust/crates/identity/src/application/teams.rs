use std::collections::BTreeSet;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use citadel_domain::{ActorId, ActorType, PermissionLevel, ResourceType, SpecificPermission};
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    Clock, EntitlementService, IdentityError, PagedResult, PatchField, ResourceInfo, StoredPage,
    permission_matrix, validate_name,
};

const MAXIMUM_TEAM_NAME_CHARACTERS: usize = 140;
const MINIMUM_SEARCH_CHARACTERS: usize = 2;
const MAXIMUM_PAGE_SIZE: i64 = 500;
const MAXIMUM_SEARCH_RESULTS: i64 = 50;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamView {
    pub id: Uuid,
    pub name: String,
    pub actor_id: ActorId,
    pub is_enabled: bool,
    pub total_members: i32,
    pub users: Option<Vec<ResourceInfo>>,
    pub roles: Option<Vec<ResourceInfo>>,
    pub resource_accesses: Option<Vec<TeamResourceAccessView>>,
    pub members: Option<Vec<TeamMemberView>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamMemberView {
    pub actor_id: ActorId,
    pub resource_id: Uuid,
    pub name: String,
    pub principal_type: ActorType,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamSearchItemView {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamResourceAccessInput {
    pub resource_type: ResourceType,
    pub resource_id: Uuid,
    pub permission_level: PermissionLevel,
    #[serde(default)]
    pub specific_permissions: Vec<SpecificPermission>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamResourceAccessView {
    pub resource_type: ResourceType,
    pub resource_id: Uuid,
    pub resource_name: Option<String>,
    pub permission_level: PermissionLevel,
    pub specific_permissions: Option<Vec<SpecificPermission>>,
    pub id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTeamRequest {
    pub name: String,
    #[serde(default)]
    pub user_ids: Vec<Uuid>,
    #[serde(default)]
    pub role_ids: Vec<Uuid>,
    #[serde(default)]
    pub resource_accesses: Vec<TeamResourceAccessInput>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchTeamRequest {
    #[serde(default)]
    pub is_enabled: PatchField<bool>,
    #[serde(default)]
    pub user_ids: PatchField<Vec<Uuid>>,
    #[serde(default)]
    pub role_ids: PatchField<Vec<Uuid>>,
    #[serde(default)]
    pub resource_accesses: PatchField<Vec<TeamResourceAccessInput>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameTeamRequest {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddTeamRoleRequest {
    pub role_id: Uuid,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddTeamMemberRequest {
    pub member_actor_id: Uuid,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteTeamsRequest {
    pub ids: Vec<Uuid>,
}

#[derive(Debug, Clone)]
pub struct NewTeamMutation {
    pub id: Uuid,
    pub actor_id: ActorId,
    pub name: String,
    pub created_by_actor_id: ActorId,
    pub created_at: DateTime<Utc>,
    pub user_ids: Vec<Uuid>,
    pub role_ids: Vec<Uuid>,
    pub resource_accesses: Vec<TeamResourceAccessInput>,
}

#[derive(Debug, Clone, Default)]
pub struct TeamPatchMutation {
    pub is_enabled: Option<bool>,
    pub user_ids: Option<Vec<Uuid>>,
    pub role_ids: Option<Vec<Uuid>>,
    pub resource_accesses: Option<Vec<TeamResourceAccessInput>>,
}

pub trait TeamReadStore: Send + Sync {
    fn list<'a>(
        &'a self,
        name: Option<&'a str>,
        limit: i64,
        offset: i64,
    ) -> BoxFuture<'a, Result<StoredPage<TeamView>, IdentityError>>;

    fn get(&self, id: Uuid) -> BoxFuture<'_, Result<Option<TeamView>, IdentityError>>;

    fn search<'a>(
        &'a self,
        query: &'a str,
        limit: i64,
    ) -> BoxFuture<'a, Result<Vec<TeamSearchItemView>, IdentityError>>;
}

pub trait TeamMutationStore: Send + Sync {
    fn create<'a>(
        &'a self,
        team: &'a NewTeamMutation,
        custom_access_control_enabled: bool,
    ) -> BoxFuture<'a, Result<TeamView, IdentityError>>;

    fn patch<'a>(
        &'a self,
        id: Uuid,
        patch: &'a TeamPatchMutation,
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
        custom_access_control_enabled: bool,
    ) -> BoxFuture<'a, Result<TeamView, IdentityError>>;

    fn rename<'a>(
        &'a self,
        id: Uuid,
        name: &'a str,
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<TeamView, IdentityError>>;

    fn add_role(
        &self,
        id: Uuid,
        role_id: Uuid,
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
        custom_access_control_enabled: bool,
    ) -> BoxFuture<'_, Result<TeamView, IdentityError>>;

    fn remove_role(
        &self,
        id: Uuid,
        role_id: Uuid,
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<TeamView, IdentityError>>;

    fn add_member(
        &self,
        id: Uuid,
        member_actor_id: ActorId,
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
        custom_access_control_enabled: bool,
    ) -> BoxFuture<'_, Result<TeamView, IdentityError>>;

    fn remove_member(
        &self,
        id: Uuid,
        member_actor_id: ActorId,
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<TeamView, IdentityError>>;

    fn add_resource_access<'a>(
        &'a self,
        id: Uuid,
        access: &'a TeamResourceAccessInput,
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
        custom_access_control_enabled: bool,
    ) -> BoxFuture<'a, Result<TeamView, IdentityError>>;

    fn remove_resource_access<'a>(
        &'a self,
        id: Uuid,
        access: &'a TeamResourceAccessInput,
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<TeamView, IdentityError>>;

    fn delete<'a>(
        &'a self,
        ids: &'a [Uuid],
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), IdentityError>>;
}

#[derive(Clone)]
pub struct TeamReadService {
    store: Arc<dyn TeamReadStore>,
}

impl TeamReadService {
    #[must_use]
    pub fn new(store: Arc<dyn TeamReadStore>) -> Self {
        Self { store }
    }

    pub async fn list(
        &self,
        page: i64,
        page_size: i64,
        name: Option<&str>,
    ) -> Result<PagedResult<TeamView>, IdentityError> {
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

    pub async fn get(&self, id: Uuid) -> Result<TeamView, IdentityError> {
        self.store.get(id).await?.ok_or(IdentityError::NotFound)
    }

    pub async fn search(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<TeamSearchItemView>, IdentityError> {
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
    store: Arc<dyn TeamMutationStore>,
    entitlements: Arc<dyn EntitlementService>,
    clock: Arc<dyn Clock>,
}

impl TeamMutationService {
    #[must_use]
    pub fn new(
        store: Arc<dyn TeamMutationStore>,
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
        request: CreateTeamRequest,
        actor_id: ActorId,
    ) -> Result<TeamView, IdentityError> {
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
        request: PatchTeamRequest,
        actor_id: ActorId,
    ) -> Result<TeamView, IdentityError> {
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
    ) -> Result<TeamView, IdentityError> {
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
    ) -> Result<TeamView, IdentityError> {
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
    ) -> Result<TeamView, IdentityError> {
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
    ) -> Result<TeamView, IdentityError> {
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
    ) -> Result<TeamView, IdentityError> {
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
        access: TeamResourceAccessInput,
        actor_id: ActorId,
    ) -> Result<TeamView, IdentityError> {
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
        access: TeamResourceAccessInput,
        actor_id: ActorId,
    ) -> Result<TeamView, IdentityError> {
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

fn patch_value<T>(field: PatchField<T>) -> Option<T> {
    match field {
        PatchField::Value(value) => Some(value),
        PatchField::Missing | PatchField::Null => None,
    }
}

fn deduplicate_ids(mut ids: Vec<Uuid>) -> Result<Vec<Uuid>, IdentityError> {
    if ids.iter().any(Uuid::is_nil) {
        return Err(IdentityError::Validation(
            "Assignment IDs must not be empty UUIDs.".to_owned(),
        ));
    }
    ids.sort_unstable();
    ids.dedup();
    Ok(ids)
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

fn validate_resource_accesses(
    accesses: Vec<TeamResourceAccessInput>,
) -> Result<Vec<TeamResourceAccessInput>, IdentityError> {
    let matrix = permission_matrix();
    let mut unique = BTreeSet::new();
    for access in &accesses {
        validate_id(access.resource_id, "Resource ID")?;
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
            return Err(IdentityError::Validation(
                "Invalid or duplicate Team resource access permission.".to_owned(),
            ));
        }
    }
    Ok(accesses)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[derive(Default)]
    struct RecordingStore {
        list_call: Mutex<Option<(Option<String>, i64, i64)>>,
    }

    impl TeamReadStore for RecordingStore {
        fn list<'a>(
            &'a self,
            name: Option<&'a str>,
            limit: i64,
            offset: i64,
        ) -> BoxFuture<'a, Result<StoredPage<TeamView>, IdentityError>> {
            Box::pin(async move {
                *self.list_call.lock().unwrap() = Some((name.map(str::to_owned), limit, offset));
                Ok(StoredPage {
                    total_items: 0,
                    items: Vec::new(),
                })
            })
        }

        fn get(&self, _id: Uuid) -> BoxFuture<'_, Result<Option<TeamView>, IdentityError>> {
            Box::pin(async { Ok(None) })
        }

        fn search<'a>(
            &'a self,
            _query: &'a str,
            _limit: i64,
        ) -> BoxFuture<'a, Result<Vec<TeamSearchItemView>, IdentityError>> {
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
        let access = TeamResourceAccessInput {
            resource_type: ResourceType::Deployment,
            resource_id: id,
            permission_level: PermissionLevel::Read,
            specific_permissions: vec![SpecificPermission::Logs],
        };
        assert!(validate_resource_accesses(vec![access.clone()]).is_ok());
        assert!(validate_resource_accesses(vec![access.clone(), access]).is_err());
    }
}
