use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobsSnapshot {
    pub boot_id: Uuid,
    pub started_at: u64,
    pub observed_at: u64,
    pub revision: u64,
    pub history_revision: u64,
    pub jobs: Vec<JobStatus>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobStatus {
    pub id: String,
    pub name: String,
    pub resource_type: std::borrow::Cow<'static, str>,
    pub ownership: JobOwnership,
    pub state: JobState,
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
    pub last_outcome: Option<JobOutcome>,
    pub last_duration_ms: Option<u64>,
    #[serde(default)]
    pub last_completed_units: Option<u64>,
    #[serde(default)]
    pub last_unit: Option<String>,
    #[serde(default)]
    pub last_failed_units: Option<u64>,
    #[serde(default)]
    pub last_problem: Option<JobProblem>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JobOwnership {
    ProcessLocal,
    Coordinator,
    TargetSession,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JobState {
    Disabled,
    NotStarted,
    Waiting,
    Running,
    BackingOff,
    Stopped,
    Unavailable,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JobOutcome {
    Completed,
    Productive,
    NoWork,
    BudgetExhausted,
    Failed,
    Cancelled,
    Interrupted,
}

/// Bounded diagnostic context, never an execution claim or a resource health verdict.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobProblem {
    pub at: u64,
    pub summary: String,
    pub target: Option<JobTarget>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobTarget {
    pub resource_type: std::borrow::Cow<'static, str>,
    pub resource_id: Uuid,
    pub operation_id: Option<Uuid>,
    /// Persisted container identity within the Platform target, when known.
    #[serde(default)]
    pub container_id: Option<Uuid>,
}
