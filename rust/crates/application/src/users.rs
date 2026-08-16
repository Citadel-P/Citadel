use std::sync::Arc;

use citadel_domain::{ActorId, PermissionLevel, ResourceType, SpecificPermission};
use futures_util::future::BoxFuture;
use serde::Serialize;
use uuid::Uuid;

use crate::{IdentityError, PagedResult, ResourceInfo, StoredPage};

const MAXIMUM_USER_NAME_CHARACTERS: usize = 140;
const MINIMUM_SEARCH_CHARACTERS: usize = 2;
const MAXIMUM_PAGE_SIZE: i64 = 500;
const MAXIMUM_SEARCH_RESULTS: i64 = 50;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserView {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub actor_id: ActorId,
    pub is_enabled: bool,
    pub teams: Option<Vec<ResourceInfo>>,
    pub roles: Option<Vec<ResourceInfo>>,
    pub resource_accesses: Option<Vec<UserResourceAccessView>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSearchItemView {
    pub id: Uuid,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserResourceAccessView {
    pub resource_type: ResourceType,
    pub resource_id: Uuid,
    pub resource_name: Option<String>,
    pub permission_level: PermissionLevel,
    pub specific_permissions: Option<Vec<SpecificPermission>>,
    pub id: Option<Uuid>,
}

pub trait UserReadStore: Send + Sync {
    fn list<'a>(
        &'a self,
        name: Option<&'a str>,
        limit: i64,
        offset: i64,
    ) -> BoxFuture<'a, Result<StoredPage<UserView>, IdentityError>>;

    fn get(&self, id: Uuid) -> BoxFuture<'_, Result<Option<UserView>, IdentityError>>;

    fn search<'a>(
        &'a self,
        query: &'a str,
        limit: i64,
    ) -> BoxFuture<'a, Result<Vec<UserSearchItemView>, IdentityError>>;
}

#[derive(Clone)]
pub struct UserReadService {
    store: Arc<dyn UserReadStore>,
}

impl UserReadService {
    #[must_use]
    pub fn new(store: Arc<dyn UserReadStore>) -> Self {
        Self { store }
    }

    pub async fn list(
        &self,
        page: i64,
        page_size: i64,
        name: Option<&str>,
    ) -> Result<PagedResult<UserView>, IdentityError> {
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

    pub async fn get(&self, id: Uuid) -> Result<UserView, IdentityError> {
        self.store.get(id).await?.ok_or(IdentityError::NotFound)
    }

    pub async fn search(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<UserSearchItemView>, IdentityError> {
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
