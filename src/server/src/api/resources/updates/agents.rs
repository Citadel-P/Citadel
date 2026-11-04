use citadel_updates::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentTargetView {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub installed_version: Option<String>,
    pub available_version: Option<String>,
    pub status: AgentEligibilityView,
    pub supported: bool,
    pub reason: Option<String>,
    pub nodes: u32,
}
#[derive(Serialize, ToSchema)]
pub enum AgentEligibilityView {
    Available,
    Current,
    Offline,
    Unknown,
    ManualRequired,
    Incompatible,
}
impl AgentTargetView {
    pub fn new(target: AgentUpdateTarget, release: Option<&AgentUpdateRelease>) -> Self {
        let eligibility = target.eligibility(release);
        let status = match eligibility {
            AgentUpdateEligibility::Available => AgentEligibilityView::Available,
            AgentUpdateEligibility::Current => AgentEligibilityView::Current,
            AgentUpdateEligibility::Offline => AgentEligibilityView::Offline,
            AgentUpdateEligibility::Unknown => AgentEligibilityView::Unknown,
            AgentUpdateEligibility::ManualRequired => AgentEligibilityView::ManualRequired,
            AgentUpdateEligibility::Incompatible => AgentEligibilityView::Incompatible,
        };
        Self {
            id: target.id,
            name: target.name,
            kind: format!("{:?}", target.kind),
            installed_version: target.installed_version,
            available_version: release.map(|r| r.version.clone()),
            status,
            supported: target.supported,
            reason: target.reason,
            nodes: target.nodes,
        }
    }
}
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateScheduleView {
    #[serde(default)]
    pub include_core: bool,
    pub targets: Vec<String>,
    pub enabled: bool,
    pub cron: String,
    pub time_zone: String,
}
impl From<UpdateSchedule> for UpdateScheduleView {
    fn from(s: UpdateSchedule) -> Self {
        Self {
            include_core: s.include_core,
            targets: s.targets,
            enabled: s.enabled,
            cron: s.cron,
            time_zone: s.time_zone,
        }
    }
}
impl From<UpdateScheduleView> for UpdateSchedule {
    fn from(s: UpdateScheduleView) -> Self {
        Self {
            include_core: s.include_core,
            targets: s.targets,
            enabled: s.enabled,
            cron: s.cron,
            time_zone: s.time_zone,
        }
    }
}
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewAgentsInput {
    pub targets: Vec<String>,
}
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentReviewView {
    pub id: Uuid,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub targets: Vec<AgentTargetView>,
}
impl From<AgentUpdateReview> for AgentReviewView {
    fn from(r: AgentUpdateReview) -> Self {
        Self {
            id: r.id,
            expires_at: r.expires_at,
            targets: r
                .targets
                .into_iter()
                .map(|t| AgentTargetView::new(t, Some(&r.release)))
                .collect(),
        }
    }
}
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentResultView {
    pub target: AgentTargetView,
    pub phase: AgentPhaseView,
    pub message: Option<String>,
}
#[derive(Serialize, ToSchema)]
pub enum AgentPhaseView {
    Pending,
    Updating,
    Succeeded,
    Skipped,
    Failed,
    NotStarted,
}
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentOperationView {
    pub id: Uuid,
    pub finished: bool,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub targets: Vec<AgentResultView>,
}
impl From<AgentUpdateOperation> for AgentOperationView {
    fn from(op: AgentUpdateOperation) -> Self {
        Self {
            id: op.id,
            finished: op.finished,
            updated_at: op.updated_at,
            targets: op
                .targets
                .into_iter()
                .map(|r| AgentResultView {
                    target: AgentTargetView::new(r.target, Some(&op.release)),
                    message: r.message,
                    phase: match r.phase {
                        AgentUpdatePhase::Pending => AgentPhaseView::Pending,
                        AgentUpdatePhase::Updating => AgentPhaseView::Updating,
                        AgentUpdatePhase::Succeeded => AgentPhaseView::Succeeded,
                        AgentUpdatePhase::Skipped => AgentPhaseView::Skipped,
                        AgentUpdatePhase::Failed => AgentPhaseView::Failed,
                        AgentUpdatePhase::NotStarted => AgentPhaseView::NotStarted,
                    },
                })
                .collect(),
        }
    }
}
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentUpdatesView {
    pub targets: Vec<AgentTargetView>,
    pub schedule: UpdateScheduleView,
    pub operation: Option<AgentOperationView>,
    pub check_error: Option<String>,
    pub next_run: Option<chrono::DateTime<chrono::Utc>>,
    pub schedule_run: Option<UpdateScheduleRunView>,
}
impl AgentUpdatesView {
    pub fn new(state: AgentUpdateState, targets: Vec<AgentUpdateTarget>) -> Self {
        let next_run = state
            .schedule
            .enabled
            .then(|| {
                citadel_primitives::schedule::CronSchedule::parse(
                    &state.schedule.cron,
                    &state.schedule.time_zone,
                )
                .ok()
                .and_then(|schedule| schedule.next_after(chrono::Utc::now()))
            })
            .flatten();
        Self {
            next_run,
            schedule_run: state.run.map(|run| UpdateScheduleRunView {
                id: run.id,
                phase: format!("{:?}", run.phase),
                message: run.message,
                core_operation: run.core_operation,
                agent_operation: run.agent_operation,
            }),
            targets: targets
                .into_iter()
                .map(|t| AgentTargetView::new(t, state.release.as_ref()))
                .collect(),
            schedule: state.schedule.into(),
            operation: state.operation.map(Into::into),
            check_error: state.check_error,
        }
    }
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateScheduleRunView {
    pub id: Uuid,
    pub phase: String,
    pub message: Option<String>,
    pub core_operation: Option<Uuid>,
    pub agent_operation: Option<Uuid>,
}
