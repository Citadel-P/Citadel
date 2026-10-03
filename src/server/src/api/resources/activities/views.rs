use crate::api::error::ApiError;
use citadel_activities::{ActivityEventType, ActivityRecord, ActivityResourceType, ActivityStatus};
use citadel_identity::ActorType;
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ActivityView {
    pub(crate) id: Uuid,
    #[schema(required = true)]
    pub(crate) platform_id: Option<Uuid>,
    #[schema(required = true)]
    pub(crate) resource_id: Option<Uuid>,
    pub(crate) platform_name: String,
    pub(crate) resource_name: String,
    #[schema(value_type = crate::api::resources::schema_models::primitives::PlatformStatusSchema)]
    pub(crate) platform_status: citadel_primitives::PlatformStatus,
    #[schema(value_type = crate::api::resources::vocabulary::ActivityResourceTypeSchema)]
    pub(crate) resource_type: ActivityResourceType,
    #[schema(value_type = crate::api::resources::vocabulary::ActivityEventTypeSchema)]
    pub(crate) event_type: ActivityEventType,
    #[schema(value_type = crate::api::resources::vocabulary::ActivityStatusSchema)]
    pub(crate) status: ActivityStatus,
    pub(crate) created_at: chrono::DateTime<chrono::Utc>,
    #[schema(value_type = crate::api::resources::activities::schema::PublicActivityEventInfo)]
    pub(crate) info: Value,
    pub(crate) actor_id: Uuid,
    pub(crate) actor_name: String,
    #[schema(value_type = crate::api::resources::vocabulary::ActorTypeSchema)]
    pub(crate) actor_type: ActorType,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PagedActivityView {
    pub(crate) items: Vec<ActivityView>,
    pub(crate) total_count: i64,
    pub(crate) page: i32,
    pub(crate) page_size: i32,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ActivitiesView {
    pub(crate) paged_result: PagedActivityView,
}

pub(crate) fn map_activity(record: ActivityRecord) -> Result<ActivityView, ApiError> {
    let info = serde_json::from_str::<Value>(&record.info_json).map_err(ApiError::internal)?;
    Ok(ActivityView {
        id: record.id,
        platform_id: record.platform_id,
        resource_id: record.resource_id,
        platform_name: record.platform_name,
        resource_name: record.resource_name,
        platform_status: record.platform_status,
        resource_type: record.resource_type,
        event_type: record.event_type,
        status: record.status,
        created_at: record.created_at,
        info: crate::api::resources::activities::presentation::public_activity_info(info),
        actor_id: record.actor_id,
        actor_name: record.actor_name,
        actor_type: ActorType::from_database_str(&record.actor_type).ok_or_else(|| {
            ApiError::Storage(format!(
                "Unknown persisted ActorType '{}'.",
                record.actor_type
            ))
        })?,
    })
}

/// Activity summary embedded in resource HTTP and realtime responses.
#[derive(Debug, Clone, PartialEq, serde::Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LatestActivityView {
    pub id: Uuid,
    #[schema(value_type = crate::api::resources::vocabulary::ActivityResourceTypeSchema)]
    pub resource_type: ActivityResourceType,
    #[schema(value_type = crate::api::resources::vocabulary::ActivityEventTypeSchema)]
    pub event_type: ActivityEventType,
    #[schema(value_type = crate::api::resources::vocabulary::ActivityStatusSchema)]
    pub status: ActivityStatus,
    #[schema(value_type = crate::api::resources::activities::schema::PublicActivityEventInfo)]
    pub info: Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<citadel_activities::ActivitySummary> for LatestActivityView {
    fn from(value: citadel_activities::ActivitySummary) -> Self {
        Self {
            id: value.id,
            resource_type: value.resource_type,
            event_type: value.event_type,
            status: value.status,
            info: crate::api::resources::activities::presentation::public_activity_info(value.info),
            created_at: value.created_at,
        }
    }
}
