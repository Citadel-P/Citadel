use crate::InstanceError;
use chrono::{DateTime, Utc};
use citadel_primitives::schedule::CronSchedule;
use futures_util::future::BoxFuture;
use uuid::Uuid;

/// Names also identify the corresponding branch of the SQL schedule projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SchedulePurpose {
    Script,
    Backup,
    Install,
    Release,
    Git,
    DeploymentImages,
    ServiceImages,
    StackImages,
    StackGit,
    Image,
}

impl SchedulePurpose {
    pub const ALL: [Self; 10] = [
        Self::Script,
        Self::Backup,
        Self::Install,
        Self::Release,
        Self::Git,
        Self::DeploymentImages,
        Self::ServiceImages,
        Self::StackImages,
        Self::StackGit,
        Self::Image,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Script => "Script",
            Self::Backup => "Backup",
            Self::Install => "Install",
            Self::Release => "Release",
            Self::Git => "Git",
            Self::DeploymentImages => "DeploymentImages",
            Self::ServiceImages => "ServiceImages",
            Self::StackImages => "StackImages",
            Self::StackGit => "StackGit",
            Self::Image => "Image",
        }
    }
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.name() == name)
    }
    pub fn fixed_interval(self) -> Option<i64> {
        match self {
            Self::DeploymentImages | Self::ServiceImages | Self::StackImages => Some(7200),
            Self::StackGit => Some(300),
            _ => None,
        }
    }
}

pub struct ScheduleDefinition {
    pub purpose: SchedulePurpose,
    pub owner: Uuid,
    pub name: String,
    pub platform: Option<Uuid>,
    pub docker_image_id: Option<String>,
    pub enabled: bool,
    pub blocker: Option<String>,
    pub script_timeout_seconds: Option<i32>,
    pub cron: Option<String>,
    pub zone: Option<String>,
    pub seconds: Option<i64>,
    pub anchor: Option<DateTime<Utc>>,
}

impl ScheduleDefinition {
    pub fn next_time(&self, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        if !self.enabled || self.blocker.is_some() {
            return None;
        }
        if let Some(cron) = &self.cron {
            let schedule = CronSchedule::parse(cron, self.zone.as_deref()?).ok()?;
            return calendar_deadline(&schedule, self.anchor, now);
        }
        match self.purpose {
            SchedulePurpose::Release => Some(self.anchor.map_or(now, |at| {
                at + chrono::Duration::seconds(self.seconds.unwrap_or(21600))
            })),
            _ => self.anchor,
        }
    }
}

pub fn calendar_deadline(
    schedule: &CronSchedule,
    consumed: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    let minute = DateTime::from_timestamp(now.timestamp().div_euclid(60) * 60, 0)?;
    if consumed.is_none_or(|last| last < minute) && schedule.is_due(minute) {
        Some(minute)
    } else {
        schedule.next_after(consumed.map_or(now, |last| last.max(now)))
    }
}

pub trait ScheduleReader: Send + Sync {
    fn page(
        &self,
        purpose: Option<SchedulePurpose>,
        after: Option<(SchedulePurpose, Uuid)>,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<ScheduleDefinition>, InstanceError>>;
    fn one(
        &self,
        purpose: SchedulePurpose,
        owner: Uuid,
    ) -> BoxFuture<'_, Result<Option<ScheduleDefinition>, InstanceError>>;
    fn batch_deadline(
        &self,
        purpose: SchedulePurpose,
    ) -> BoxFuture<'_, Result<Option<DateTime<Utc>>, InstanceError>>;
}
