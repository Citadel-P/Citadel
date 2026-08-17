use std::sync::Arc;

use chrono::{DateTime, Utc};
use citadel_domain::{ActivityEventType, ActivityResourceType, ActivityStatus, ActorType};
use citadel_identity::{ActorPrincipal, IdentityError};
use futures_util::future::BoxFuture;
use uuid::Uuid;

pub const DEFAULT_ACTIVITY_PAGE_SIZE: i32 = 50;
pub const MAXIMUM_ACTIVITY_PAGE_SIZE: i32 = 500;

#[derive(Debug, Clone, Copy, Default)]
pub struct ActivityFilter {
    pub resource_id: Option<Uuid>,
    pub resource_type: Option<ActivityResourceType>,
    pub event_type: Option<ActivityEventType>,
    pub page: Option<i32>,
    pub page_size: Option<i32>,
}

impl ActivityFilter {
    pub fn validated(self) -> Result<ValidatedActivityFilter, IdentityError> {
        let page = self.page.unwrap_or(1);
        let page_size = self.page_size.unwrap_or(DEFAULT_ACTIVITY_PAGE_SIZE);
        if page <= 0 {
            return Err(IdentityError::Validation(
                "Page must be greater than zero.".to_owned(),
            ));
        }
        if !(1..=MAXIMUM_ACTIVITY_PAGE_SIZE).contains(&page_size) {
            return Err(IdentityError::Validation(format!(
                "PageSize must be between 1 and {MAXIMUM_ACTIVITY_PAGE_SIZE}."
            )));
        }
        Ok(ValidatedActivityFilter {
            resource_id: self.resource_id,
            resource_type: self.resource_type,
            event_type: self.event_type,
            page,
            page_size,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidatedActivityFilter {
    pub resource_id: Option<Uuid>,
    pub resource_type: Option<ActivityResourceType>,
    pub event_type: Option<ActivityEventType>,
    pub page: i32,
    pub page_size: i32,
}

#[derive(Debug, Clone)]
pub struct ActivityRecord {
    pub id: Uuid,
    pub platform_id: Option<Uuid>,
    pub resource_id: Option<Uuid>,
    pub platform_name: String,
    pub resource_name: String,
    pub platform_status: String,
    pub resource_type: ActivityResourceType,
    pub event_type: ActivityEventType,
    pub status: ActivityStatus,
    pub created_at: DateTime<Utc>,
    pub info_json: String,
    pub actor_id: Uuid,
    pub actor_name: String,
    pub actor_type: ActorType,
}

#[derive(Debug, Clone)]
pub struct PagedActivityRecords {
    pub items: Vec<ActivityRecord>,
    pub total_count: i64,
    pub page: i32,
    pub page_size: i32,
}

pub trait ActivityQueryStore: Send + Sync {
    fn get_authorized<'a>(
        &'a self,
        principal: &'a ActorPrincipal,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Option<ActivityRecord>, IdentityError>>;

    fn list_authorized<'a>(
        &'a self,
        principal: &'a ActorPrincipal,
        filter: ValidatedActivityFilter,
    ) -> BoxFuture<'a, Result<PagedActivityRecords, IdentityError>>;
}

pub struct ActivityService {
    store: Arc<dyn ActivityQueryStore>,
}

impl ActivityService {
    #[must_use]
    pub fn new(store: Arc<dyn ActivityQueryStore>) -> Self {
        Self { store }
    }

    pub async fn get(
        &self,
        principal: &ActorPrincipal,
        id: Uuid,
    ) -> Result<ActivityRecord, IdentityError> {
        self.store
            .get_authorized(principal, id)
            .await?
            .ok_or(IdentityError::NotFound)
    }

    pub async fn list(
        &self,
        principal: &ActorPrincipal,
        filter: ActivityFilter,
    ) -> Result<PagedActivityRecords, IdentityError> {
        self.store
            .list_authorized(principal, filter.validated()?)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_paging_is_bounded() {
        assert!(
            ActivityFilter {
                page_size: Some(MAXIMUM_ACTIVITY_PAGE_SIZE),
                ..ActivityFilter::default()
            }
            .validated()
            .is_ok()
        );
        assert!(
            ActivityFilter {
                page_size: Some(MAXIMUM_ACTIVITY_PAGE_SIZE + 1),
                ..ActivityFilter::default()
            }
            .validated()
            .is_err()
        );
    }
}
