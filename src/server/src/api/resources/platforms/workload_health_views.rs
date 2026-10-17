//! OpenAPI mirrors of the shared, transport-neutral health read models.
use uuid::Uuid;
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub enum WorkloadHealthStatus {
    Healthy,
    Unhealthy,
    Starting,
    NoCheck,
    Disabled,
    NotRunning,
    Partial,
    Unknown,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub enum HealthAttention {
    Unhealthy,
    RuntimeMissing,
    UnexpectedExit,
    RuntimeUnavailable,
    ObservationStale,
    OperationFailed,
    OwnershipConflict,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContainerHealthObservation {
    pub configuration: CheckConfiguration,
    pub result: Option<HealthStatus>,
    /// Time of an actual inspection, never a heartbeat or statistics timestamp.
    pub observed_at: i64,
    pub last_check_at: Option<String>,
    pub failing_streak: Option<i64>,
    pub exit_code: Option<i64>,
    pub oom_killed: Option<bool>,
    pub restart_count: Option<i64>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub enum CheckConfiguration {
    Enabled,
    Disabled,
    NoCheck,
    Unknown,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Starting,
    NoCheck,
    Disabled,
    NotRunning,
    Unknown,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HealthMember {
    pub status: HealthStatus,
    pub container_id: Option<Uuid>,
    pub docker_container_id: Option<String>,
    pub task_id: Option<String>,
    pub platform_id: Uuid,
    pub node_id: Option<String>,
    pub name: String,
    pub service: String,
    pub state: String,
    pub available: bool,
    pub observation: Option<ContainerHealthObservation>,
    #[serde(default)]
    pub completed_job: bool,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HealthCounts {
    pub healthy: usize,
    pub unhealthy: usize,
    pub starting: usize,
    pub no_check: usize,
    pub disabled: usize,
    pub not_running: usize,
    pub unknown: usize,
    pub completed: usize,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkloadHealth {
    pub status: WorkloadHealthStatus,
    pub counts: HealthCounts,
    pub observed: usize,
    pub members_truncated: bool,
    pub expected: Option<usize>,
    pub missing: Option<usize>,
    pub coverage_complete: bool,
    pub observed_at: Option<i64>,
    pub evaluated_at: i64,
    pub valid_until: Option<i64>,
    pub unhealthy_until: Option<i64>,
    pub starting_until: Option<i64>,
    pub reason: String,
    pub attention: Vec<HealthAttention>,
    pub members: Vec<HealthMember>,
}
