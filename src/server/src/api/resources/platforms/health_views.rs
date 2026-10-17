use citadel_platforms::container_health::{ContainerHealthObservation, HealthStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum ContainerHealthStatus {
    Healthy,
    Unhealthy,
    Starting,
    NoCheck,
    Disabled,
    NotRunning,
    Unknown,
}

impl From<HealthStatus> for ContainerHealthStatus {
    fn from(value: HealthStatus) -> Self {
        match value {
            HealthStatus::Healthy => Self::Healthy,
            HealthStatus::Unhealthy => Self::Unhealthy,
            HealthStatus::Starting => Self::Starting,
            HealthStatus::NoCheck => Self::NoCheck,
            HealthStatus::Disabled => Self::Disabled,
            HealthStatus::NotRunning => Self::NotRunning,
            HealthStatus::Unknown => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContainerHealthView {
    pub status: ContainerHealthStatus,
    /// Unix milliseconds of inspection, not the last lifecycle event or heartbeat.
    pub observed_at: Option<i64>,
    pub last_check_at: Option<String>,
    pub failing_streak: Option<i64>,
    pub reason: String,
}
impl ContainerHealthView {
    pub fn from_observation(
        value: Option<&ContainerHealthObservation>,
        state: &str,
        available: bool,
    ) -> Self {
        let status = citadel_primitives::health::observed_status(
            value,
            state,
            available,
            chrono::Utc::now().timestamp_millis(),
        );
        let reason = match status {
            HealthStatus::Healthy => "Docker reports a passing health check.",
            HealthStatus::Unhealthy => {
                "Docker reports a failing health check. Open details for available evidence."
            }
            HealthStatus::Starting => "Docker is initializing the health check.",
            HealthStatus::NoCheck => "The effective container configuration has no health check.",
            HealthStatus::Disabled => {
                "The effective container configuration explicitly disables health checks."
            }
            HealthStatus::NotRunning => {
                "The container is not currently running health checks. Earlier results are history."
            }
            HealthStatus::Unknown if !available => {
                "The owning runtime is unavailable or its observation is stale."
            }
            HealthStatus::Unknown => {
                "Health metadata has not been observed or is unsupported by this Agent."
            }
        };
        Self {
            status: status.into(),
            observed_at: value.map(|v| v.observed_at),
            last_check_at: value.and_then(|v| v.last_check_at.clone()),
            failing_streak: value.and_then(|v| v.failing_streak),
            reason: reason.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContainerHealthTransitionView {
    pub observed_at: i64,
    /// First actual observation in this group, in Unix seconds. Not a coverage start.
    pub first_observed_at: i64,
    /// Number of retained observations represented; transitions always have one.
    pub sample_count: i64,
    pub state: String,
    pub health: ContainerHealthView,
    pub origin: String,
    pub exit_code: Option<i64>,
    pub oom_killed: Option<bool>,
}
impl From<citadel_platforms::container_health::ContainerHealthTransition>
    for ContainerHealthTransitionView
{
    fn from(value: citadel_platforms::container_health::ContainerHealthTransition) -> Self {
        Self {
            observed_at: value.observed_at,
            first_observed_at: value.first_observed_at,
            sample_count: value.sample_count,
            health: {
                let mut view = ContainerHealthView::from_observation(
                    value.health.as_ref(),
                    &value.state,
                    true,
                );
                if let Some(health) = &value.health {
                    view.status = health.status(&value.state, true).into();
                    view.reason = "Recorded observation; not the current health result.".into();
                }
                view
            },
            exit_code: value.health.as_ref().and_then(|v| v.exit_code),
            oom_killed: value.health.as_ref().and_then(|v| v.oom_killed),
            state: value.state,
            origin: value.origin,
        }
    }
}
