use citadel_instance::{ScheduleDefinition, SchedulePurpose};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, utoipa::IntoParams)]
pub struct ScheduleGroupQuery {
    /// Case-insensitive substring of the resource name.
    pub search: Option<String>,
}

#[derive(Deserialize, utoipa::IntoParams)]
pub struct UpcomingQuery {
    pub limit: Option<u8>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct SchedulePurposeCount {
    pub resource: String,
    pub purpose: String,
    pub count: i64,
    pub attention: i64,
}

#[derive(Deserialize, utoipa::IntoParams)]
pub struct ScheduleQuery {
    /// Resource group, for example script, instance or stack. Omit to include all groups.
    pub resource: Option<String>,
    /// Case-insensitive substring of the resource name.
    pub search: Option<String>,
    /// One-based page number.
    pub page: Option<u32>,
    #[serde(rename = "pageSize")]
    pub page_size: Option<u32>,
    #[serde(rename = "attentionOnly")]
    pub attention_only: Option<bool>,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleView {
    /// Status of the owning resource. Instance-wide schedules have no resource indicator.
    pub resource_status: Option<String>,
    pub needs_attention: bool,
    pub latest_run: Option<ScheduleRunView>,
    /// Current scheduled script or backup run, while active.
    pub active_run_status: Option<String>,
    pub purpose: String,
    pub owner_id: Uuid,
    pub name: String,
    pub platform_id: Option<Uuid>,
    pub docker_image_id: Option<String>,
    pub enabled: bool,
    pub cron: Option<String>,
    pub time_zone: Option<String>,
    pub interval_seconds: Option<i64>,
    pub next_at: Option<chrono::DateTime<chrono::Utc>>,
    pub system_managed: bool,
    pub reason: Option<String>,
}
#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleRunView {
    pub queued_at: chrono::DateTime<chrono::Utc>,
    pub id: Uuid,
    pub status: String,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SchedulePage {
    pub items: Vec<ScheduleView>,
}
impl From<Vec<ScheduleDefinition>> for SchedulePage {
    fn from(rows: Vec<ScheduleDefinition>) -> Self {
        Self::at(rows, chrono::Utc::now())
    }
}
impl SchedulePage {
    pub fn at(rows: Vec<ScheduleDefinition>, now: chrono::DateTime<chrono::Utc>) -> Self {
        let items = rows
            .into_iter()
            .map(|d| {
                let next_at = d.next_time(now);
                ScheduleView {
                    resource_status: d.resource_status,
                    active_run_status: d.active_run_status,
                    needs_attention: d.needs_attention,
                    latest_run: d.latest_run.map(|r| ScheduleRunView {
                        queued_at: r.queued_at,
                        id: r.id,
                        status: r.status,
                    }),
                    next_at,
                    reason: if !d.enabled {
                        Some("Scheduling is disabled for this resource.".into())
                    } else {
                        d.blocker
                    },
                    purpose: d.purpose.name().into(),
                    system_managed: d.purpose.fixed_interval().is_some()
                        || d.purpose == SchedulePurpose::Image,
                    owner_id: d.owner,
                    name: d.name,
                    platform_id: d.platform,
                    docker_image_id: d.docker_image_id,
                    enabled: d.enabled,
                    cron: d.cron,
                    time_zone: d.zone,
                    interval_seconds: d.seconds,
                }
            })
            .collect();
        Self { items }
    }
}
