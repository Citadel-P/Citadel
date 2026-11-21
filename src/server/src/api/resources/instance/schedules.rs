use citadel_instance::{ScheduleDefinition, SchedulePurpose};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, utoipa::IntoParams)]
pub struct ScheduleQuery {
    pub purpose: Option<String>,
    pub after: Option<String>,
}
impl ScheduleQuery {
    pub fn purpose(&self) -> Result<Option<SchedulePurpose>, citadel_instance::InstanceError> {
        self.purpose.as_deref().map(parse).transpose()
    }
    pub fn cursor(
        &self,
    ) -> Result<Option<(SchedulePurpose, Uuid)>, citadel_instance::InstanceError> {
        self.after
            .as_deref()
            .map(|s| {
                let (p, id) = s.split_once(':').ok_or_else(invalid)?;
                let purpose = parse(p)?;
                if self
                    .purpose
                    .as_deref()
                    .is_some_and(|filter| filter != purpose.name())
                {
                    return Err(invalid());
                }
                Ok((purpose, id.parse().map_err(|_| invalid())?))
            })
            .transpose()
    }
}
fn parse(p: &str) -> Result<SchedulePurpose, citadel_instance::InstanceError> {
    SchedulePurpose::parse(p).ok_or_else(invalid)
}
fn invalid() -> citadel_instance::InstanceError {
    citadel_instance::InstanceError::Validation("Invalid schedule purpose or page cursor.".into())
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleView {
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
pub struct SchedulePage {
    pub items: Vec<ScheduleView>,
    pub next_cursor: Option<String>,
}
impl From<Vec<ScheduleDefinition>> for SchedulePage {
    fn from(mut rows: Vec<ScheduleDefinition>) -> Self {
        let next_cursor =
            (rows.len() > 50).then(|| format!("{}:{}", rows[49].purpose.name(), rows[49].owner));
        rows.truncate(50);
        let now = chrono::Utc::now();
        let items = rows
            .into_iter()
            .map(|d| {
                let next_at = d.next_time(now);
                ScheduleView {
                    next_at,
                    reason: if !d.enabled {
                        Some("Scheduling is disabled for this resource.".into())
                    } else if let Some(reason) = d.blocker {
                        Some(reason)
                    } else if next_at.is_none() {
                        Some("No eligible deadline is available.".into())
                    } else {
                        None
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
        Self { items, next_cursor }
    }
}
