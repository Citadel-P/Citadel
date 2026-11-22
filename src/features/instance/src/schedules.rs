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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleMode {
    Calendar,
    PersistedDeadline,
    Fixed {
        seconds: i64,
        startup_label: &'static str,
    },
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
    pub fn resource(self) -> &'static str {
        match self {
            Self::Script => "script",
            Self::Backup => "backup-policy",
            Self::Install | Self::Release => "instance",
            Self::Git => "git-repository",
            Self::DeploymentImages => "deployment",
            Self::ServiceImages => "swarm-service",
            Self::StackImages | Self::StackGit => "stack",
            Self::Image => "image",
        }
    }
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.name() == name)
    }
    pub const fn mode(self) -> ScheduleMode {
        use ScheduleMode::*;
        match self {
            Self::Script | Self::Backup | Self::Install => Calendar,
            Self::Release | Self::Git | Self::Image => PersistedDeadline,
            Self::DeploymentImages => Fixed {
                seconds: 7200,
                startup_label: "deployment-image-updates",
            },
            Self::ServiceImages => Fixed {
                seconds: 7200,
                startup_label: "service-image-updates",
            },
            Self::StackImages => Fixed {
                seconds: 7200,
                startup_label: "stack-updates",
            },
            Self::StackGit => Fixed {
                seconds: 300,
                startup_label: "stack-updates",
            },
        }
    }

    pub fn fixed_interval(self) -> Option<i64> {
        match self.mode() {
            ScheduleMode::Fixed { seconds, .. } => Some(seconds),
            _ => None,
        }
    }
}

#[derive(Default)]
pub struct ScheduleActivity {
    pub catalog_revision: u64,
    pub active_batches: std::collections::BTreeSet<SchedulePurpose>,
}

pub struct ScheduleBatchGuard<'a> {
    pub(crate) purpose: SchedulePurpose,
    pub(crate) changes: &'a tokio::sync::watch::Sender<ScheduleActivity>,
}
impl Drop for ScheduleBatchGuard<'_> {
    fn drop(&mut self) {
        self.changes.send_modify(|activity| {
            activity.active_batches.remove(&self.purpose);
        });
    }
}

pub const SCHEDULE_PAGE_SIZE: u32 = 25;

#[derive(Clone, Copy)]
pub enum SchedulePolicyBlocker {
    EligibilityUnknown,
    Blocked(&'static str),
}

impl SchedulePolicyBlocker {
    pub fn reason(self) -> &'static str {
        match self {
            Self::EligibilityUnknown => "Schedule eligibility is unavailable.",
            Self::Blocked(reason) => reason,
        }
    }
}

#[derive(Default)]
pub struct ScheduleCatalogPolicy {
    pub blockers: std::collections::BTreeMap<SchedulePurpose, SchedulePolicyBlocker>,
    pub max_script_timeout: u32,
    pub attention_only: bool,
}

pub struct ScheduleCount {
    pub purpose: SchedulePurpose,
    pub total: i64,
    pub attention: i64,
}

pub struct ScheduledRun {
    pub queued_at: DateTime<Utc>,
    pub id: Uuid,
    pub status: String,
}

pub struct ScheduleDefinition {
    pub resource_status: Option<String>,
    pub needs_attention: bool,
    pub latest_run: Option<ScheduledRun>,
    pub active_run_status: Option<String>,
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
        if self.purpose.mode() == ScheduleMode::Calendar {
            let schedule =
                CronSchedule::parse(self.cron.as_deref()?, self.zone.as_deref()?).ok()?;
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
    fn counts<'a>(
        &'a self,
        search: &'a str,
        policy: &'a ScheduleCatalogPolicy,
    ) -> BoxFuture<'a, Result<Vec<ScheduleCount>, InstanceError>>;
    fn page<'a>(
        &'a self,
        purposes: &'a [SchedulePurpose],
        search: &'a str,
        offset: i64,
        limit: i64,
        policy: &'a ScheduleCatalogPolicy,
    ) -> BoxFuture<'a, Result<Vec<ScheduleDefinition>, InstanceError>>;
    fn discover(
        &self,
        purpose: SchedulePurpose,
        after: Option<Uuid>,
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
