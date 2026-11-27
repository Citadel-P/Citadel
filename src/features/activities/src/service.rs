use crate::ActivityAccess;
use crate::ActivityError;
use crate::read_models::*;
use crate::repository::*;
use std::sync::Arc;
use uuid::Uuid;
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
        principal: &ActivityAccess,
        id: Uuid,
    ) -> Result<ActivityRecord, ActivityError> {
        self.store
            .get_authorized(principal, id)
            .await?
            .ok_or(ActivityError::NotFound)
    }

    pub async fn recent(
        &self,
        principal: &ActivityAccess,
        event_type: Option<crate::ActivityEventType>,
        limit: i32,
        tags: &[String],
    ) -> Result<Vec<RecentActivity>, ActivityError> {
        if !(1..=10).contains(&limit) {
            return Err(ActivityError::Validation(
                "Limit must be between 1 and 10.".into(),
            ));
        }
        self.store
            .recent_authorized(principal, event_type, limit, tags)
            .await
    }

    pub async fn list(
        &self,
        principal: &ActivityAccess,
        filter: ActivityFilter,
    ) -> Result<PagedActivityRecords, ActivityError> {
        self.store
            .list_authorized(principal, filter.validated()?)
            .await
    }
}
