//! Semantic state observations, without inventory metadata or command ownership changes.
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerLifecycleState {
    Running,
    Paused,
    Exited,
}
impl ContainerLifecycleState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Exited => "exited",
        }
    }
}

/// Platform/node scope belongs to the write batch, not each incoming row.
#[derive(Debug, Clone)]
pub struct ContainerStateDelta {
    pub docker_id: String,
    pub state: ContainerLifecycleState,
    /// Daemon time (or Agent receive time), in the projection's Unix-second units.
    pub observed_at: i64,
    /// Original observation time for live command causality checks.
    pub observed_at_millis: i64,
}

#[derive(Debug, Clone, Default)]
pub struct ContainerDeltaResult {
    pub docker_id: String,
    /// A known but older observation is handled, without changing its projection.
    pub accepted: bool,
    /// Fresh committed observation; stale accepted inputs cannot confirm commands.
    pub observed: bool,
    pub changed: bool,
    pub container_id: Option<Uuid>,
    pub operation_id: Option<Uuid>,
    pub defer_parent_effects: bool,
    pub deployment_id: Option<Uuid>,
    pub stack_id: Option<Uuid>,
    pub patch: Option<crate::containers::ContainerStatePatch>,
}
