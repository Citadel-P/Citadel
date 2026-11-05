use crate::{AgentUpdateRelease, AgentUpdateTarget, UpdatePlan, UpdateSchedule};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One frozen occurrence owns both executors until their cleanup is verified.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateScheduleRun {
    pub id: Uuid,
    pub minute: DateTime<Utc>,
    pub schedule: UpdateSchedule,
    pub revision: u64,
    pub phase: UpdateSchedulePhase,
    pub core_plan: Option<UpdatePlan>,
    pub core_operation: Option<Uuid>,
    pub agent_operation: Option<Uuid>,
    pub release: Option<AgentUpdateRelease>,
    pub targets: Vec<AgentUpdateTarget>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateSchedulePhase {
    Preparing,
    Core,
    Agents,
    Finished,
}

impl UpdateScheduleRun {
    pub fn new(schedule: UpdateSchedule, minute: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::now_v7(),
            minute,
            schedule,
            revision: 0,
            phase: UpdateSchedulePhase::Preparing,
            core_plan: None,
            core_operation: None,
            agent_operation: None,
            release: None,
            targets: vec![],
            message: None,
        }
    }
}
