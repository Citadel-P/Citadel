use chrono::{DateTime, Utc};
use citadel_primitives::{ActorId, schedule::CronSchedule};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{UpdateError, release_version};

/// An installation, not an individual resource alias or Swarm service task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentUpdateTarget {
    pub id: String,
    pub resource_id: Uuid,
    pub kind: AgentUpdateKind,
    pub name: String,
    pub installed_version: Option<String>,
    pub connected: bool,
    pub supported: bool,
    pub reason: Option<String>,
    pub nodes: u32,
    /// Reviewed Swarm nodes; cleanup must not lose them when live inventory changes.
    #[serde(default)]
    pub drain_nodes: Vec<String>,
    /// Transport/installation identity captured during discovery and reviewed again before execution.
    pub revision: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentUpdateKind {
    Platform,
    BuildPool,
    SwarmService,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentUpdateRelease {
    pub version: String,
    pub image: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentUpdateEligibility {
    Available,
    Current,
    Offline,
    Unknown,
    ManualRequired,
    Incompatible,
}

impl AgentUpdateTarget {
    pub fn eligibility(&self, release: Option<&AgentUpdateRelease>) -> AgentUpdateEligibility {
        use AgentUpdateEligibility::*;
        if !self.connected {
            return Offline;
        }
        let Some(installed) = self
            .installed_version
            .as_deref()
            .and_then(|v| release_version(v).ok())
        else {
            return Unknown;
        };
        let Some(target) = release.and_then(|r| release_version(&r.version).ok()) else {
            return Unknown;
        };
        if installed == target {
            return Current;
        }
        if installed > target || installed.pre.is_empty() != target.pre.is_empty() {
            return Incompatible;
        }
        if !self.supported {
            return ManualRequired;
        }
        Available
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSchedule {
    #[serde(default)]
    pub include_core: bool,
    pub targets: Vec<String>,
    pub enabled: bool,
    pub cron: String,
    pub time_zone: String,
}
impl Default for UpdateSchedule {
    fn default() -> Self {
        Self {
            targets: vec![],
            include_core: false,
            enabled: false,
            cron: "0 3 * * *".into(),
            time_zone: "UTC".into(),
        }
    }
}
impl UpdateSchedule {
    pub fn validate(&self) -> Result<(), UpdateError> {
        CronSchedule::parse(&self.cron, &self.time_zone)
            .map_err(|e| UpdateError::Validation(e.into()))?;
        if self.targets.len() > 100
            || self
                .targets
                .iter()
                .any(|id| id.is_empty() || id.len() > 256)
            || self.enabled && !self.include_core && self.targets.is_empty()
        {
            return Err(UpdateError::Validation(
                "Select Core or at least one Agent, with at most 100 Agent installations.".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentUpdateReview {
    pub id: Uuid,
    pub actor_id: ActorId,
    pub release: AgentUpdateRelease,
    pub targets: Vec<AgentUpdateTarget>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentUpdatePhase {
    Pending,
    Updating,
    Succeeded,
    Skipped,
    Failed,
    NotStarted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentUpdateResult {
    pub target: AgentUpdateTarget,
    pub phase: AgentUpdatePhase,
    pub message: Option<String>,
    /// Durable acknowledgement; only meaningful for a succeeded target.
    #[serde(default)]
    pub cleanup_completed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentUpdateOperation {
    pub id: Uuid,
    pub request_id: Uuid,
    pub review_id: Uuid,
    pub actor_id: ActorId,
    pub scheduled: bool,
    pub release: AgentUpdateRelease,
    pub targets: Vec<AgentUpdateResult>,
    pub revision: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub finished: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentUpdateState {
    pub release: Option<AgentUpdateRelease>,
    pub check_error: Option<String>,
    pub schedule: UpdateSchedule,
    pub run: Option<crate::UpdateScheduleRun>,
    pub operation: Option<AgentUpdateOperation>,
}
