use citadel_primitives::JobCategory;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobsSnapshotView {
    pub boot_id: Uuid,
    pub started_at: u64,
    pub observed_at: u64,
    pub revision: u64,
    pub history_revision: u64,
    pub jobs: Vec<JobStatusView>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobStatusView {
    pub id: String,
    pub name: String,
    #[schema(value_type = crate::api::resources::schema_models::primitives::JobCategorySchema)]
    pub resource_type: JobCategory,
    pub ownership: JobOwnershipView,
    pub state: JobStateView,
    pub trigger: Option<String>,
    pub waiting_for: Option<String>,
    pub budget_ms: Option<u64>,
    pub next_check: Option<u64>,
    pub active_iterations: u64,
    pub active_operations: Option<u64>,
    pub current_started_at: Option<u64>,
    pub last_started_at: Option<u64>,
    pub last_completed_at: Option<u64>,
    pub last_success_at: Option<u64>,
    pub last_progress_at: Option<u64>,
    pub last_failure_at: Option<u64>,
    pub last_error: Option<String>,
    pub last_outcome: Option<JobOutcomeView>,
    pub last_duration_ms: Option<u64>,
    /// Work from the last measured check, not the lifetime counter.
    pub last_completed_units: Option<u64>,
    pub last_unit: Option<String>,
    pub last_failed_units: Option<u64>,
    /// Historical report; subsequent scheduler success does not prove this problem resolved.
    pub last_problem: Option<JobProblemView>,
    pub iterations: u64,
    pub productive: u64,
    pub no_work: u64,
    pub failures: u64,
    pub budget_exhaustions: u64,
    pub total_duration_ms: u64,
    pub productive_duration_ms: u64,
    pub no_work_duration_ms: u64,
    pub max_duration_ms: u64,
    pub completed_units: Option<u64>,
    pub unit: Option<String>,
    pub backlog: Option<u64>,
    pub backlog_is_lower_bound: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum JobOwnershipView {
    ProcessLocal,
    Coordinator,
    TargetSession,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum JobStateView {
    Disabled,
    NotStarted,
    Waiting,
    Running,
    BackingOff,
    Stopped,
    Unavailable,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum JobOutcomeView {
    Completed,
    Productive,
    NoWork,
    BudgetExhausted,
    Failed,
    Cancelled,
    Interrupted,
}

impl From<citadel_instance::JobsSnapshot> for JobsSnapshotView {
    fn from(s: citadel_instance::JobsSnapshot) -> Self {
        Self {
            boot_id: s.boot_id,
            started_at: s.started_at,
            observed_at: s.observed_at,
            revision: s.revision,
            history_revision: s.history_revision,
            jobs: s.jobs.into_iter().map(Into::into).collect(),
        }
    }
}
impl From<citadel_instance::JobStatus> for JobStatusView {
    fn from(s: citadel_instance::JobStatus) -> Self {
        Self {
            id: s.id,
            name: s.name,
            resource_type: s.resource_type,
            ownership: match s.ownership {
                citadel_instance::JobOwnership::ProcessLocal => JobOwnershipView::ProcessLocal,
                citadel_instance::JobOwnership::Coordinator => JobOwnershipView::Coordinator,
                citadel_instance::JobOwnership::TargetSession => JobOwnershipView::TargetSession,
            },
            state: match s.state {
                citadel_instance::JobState::Disabled => JobStateView::Disabled,
                citadel_instance::JobState::NotStarted => JobStateView::NotStarted,
                citadel_instance::JobState::Waiting => JobStateView::Waiting,
                citadel_instance::JobState::Running => JobStateView::Running,
                citadel_instance::JobState::BackingOff => JobStateView::BackingOff,
                citadel_instance::JobState::Stopped => JobStateView::Stopped,
                citadel_instance::JobState::Unavailable => JobStateView::Unavailable,
            },
            trigger: s.trigger,
            waiting_for: s.waiting_for,
            budget_ms: s.budget_ms,
            next_check: s.next_check,
            active_iterations: s.active_iterations,
            active_operations: s.active_operations,
            current_started_at: s.current_started_at,
            last_started_at: s.last_started_at,
            last_completed_at: s.last_completed_at,
            last_success_at: s.last_success_at,
            last_progress_at: s.last_progress_at,
            last_failure_at: s.last_failure_at,
            last_error: s.last_error,
            last_outcome: s.last_outcome.map(|v| match v {
                citadel_instance::JobOutcome::Completed => JobOutcomeView::Completed,
                citadel_instance::JobOutcome::Productive => JobOutcomeView::Productive,
                citadel_instance::JobOutcome::NoWork => JobOutcomeView::NoWork,
                citadel_instance::JobOutcome::BudgetExhausted => JobOutcomeView::BudgetExhausted,
                citadel_instance::JobOutcome::Failed => JobOutcomeView::Failed,
                citadel_instance::JobOutcome::Cancelled => JobOutcomeView::Cancelled,
                citadel_instance::JobOutcome::Interrupted => JobOutcomeView::Interrupted,
            }),
            last_duration_ms: s.last_duration_ms,
            last_completed_units: s.last_completed_units,
            last_unit: s.last_unit,
            last_failed_units: s.last_failed_units,
            last_problem: s.last_problem.map(Into::into),
            iterations: s.iterations,
            productive: s.productive,
            no_work: s.no_work,
            failures: s.failures,
            budget_exhaustions: s.budget_exhaustions,
            total_duration_ms: s.total_duration_ms,
            productive_duration_ms: s.productive_duration_ms,
            no_work_duration_ms: s.no_work_duration_ms,
            max_duration_ms: s.max_duration_ms,
            completed_units: s.completed_units,
            unit: s.unit,
            backlog: s.backlog,
            backlog_is_lower_bound: s.backlog_is_lower_bound,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobProblemView {
    pub at: u64,
    pub summary: String,
    pub target: Option<JobTargetView>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobTargetView {
    #[schema(value_type = crate::api::resources::schema_models::primitives::ResourceTypeSchema)]
    pub resource_type: citadel_primitives::ResourceType,
    pub resource_id: Uuid,
    pub operation_id: Option<Uuid>,
    /// Persisted container identity within the Platform target, when known.
    #[serde(default)]
    pub container_id: Option<Uuid>,
}
impl From<citadel_instance::JobProblem> for JobProblemView {
    fn from(p: citadel_instance::JobProblem) -> Self {
        Self {
            at: p.at,
            summary: p.summary,
            target: p.target.map(|t| JobTargetView {
                resource_type: t.resource_type,
                resource_id: t.resource_id,
                operation_id: t.operation_id,
                container_id: t.container_id,
            }),
        }
    }
}
