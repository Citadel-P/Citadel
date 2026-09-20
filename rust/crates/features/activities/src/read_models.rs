use crate::ActivityError;
use crate::ActivityEventType;
use crate::ActivityResourceType;
use crate::ActivityStatus;
use chrono::{DateTime, Utc};
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
    pub fn validated(self) -> Result<ValidatedActivityFilter, ActivityError> {
        let page = self.page.unwrap_or(1);
        let page_size = self.page_size.unwrap_or(DEFAULT_ACTIVITY_PAGE_SIZE);
        if page <= 0 {
            return Err(ActivityError::Validation(
                "Page must be greater than zero.".to_owned(),
            ));
        }
        if !(1..=MAXIMUM_ACTIVITY_PAGE_SIZE).contains(&page_size) {
            return Err(ActivityError::Validation(format!(
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
    pub actor_type: String,
}

#[derive(Debug, Clone)]
pub struct PagedActivityRecords {
    pub items: Vec<ActivityRecord>,
    pub total_count: i64,
    pub page: i32,
    pub page_size: i32,
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

/// Scope derived from the authenticated principal, never from request fields.
#[derive(Debug, Clone, Copy)]
pub struct ActivityAccess {
    pub actor_id: citadel_primitives::ActorId,
    pub administrator: bool,
}
