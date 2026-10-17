//! Shared observation vocabulary and pure health classification. No runtime or ownership policy.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckConfiguration {
    Enabled,
    Disabled,
    NoCheck,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Starting,
    NoCheck,
    Disabled,
    NotRunning,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

impl ContainerHealthObservation {
    pub fn status(&self, state: &str, available: bool) -> HealthStatus {
        if !available {
            return HealthStatus::Unknown;
        }
        match state.to_ascii_lowercase().as_str() {
            "created" | "exited" | "dead" | "paused" | "restarting" | "removing" => {
                return HealthStatus::NotRunning;
            }
            "running" => {}
            _ => return HealthStatus::Unknown,
        }
        match self.configuration {
            CheckConfiguration::NoCheck => HealthStatus::NoCheck,
            CheckConfiguration::Disabled => HealthStatus::Disabled,
            CheckConfiguration::Unknown => HealthStatus::Unknown,
            CheckConfiguration::Enabled => match self.result {
                Some(HealthStatus::Healthy | HealthStatus::Unhealthy | HealthStatus::Starting) => {
                    self.result.unwrap()
                }
                _ => HealthStatus::Unknown,
            },
        }
    }
}

fn unknown_health_status() -> HealthStatus {
    HealthStatus::Unknown
}

pub const HEALTH_MAX_AGE_MS: i64 = 180_000;

pub fn observed_status(
    observation: Option<&ContainerHealthObservation>,
    state: &str,
    available: bool,
    now: i64,
) -> HealthStatus {
    if !available
        || observation
            .is_some_and(|o| !(0..=HEALTH_MAX_AGE_MS).contains(&now.saturating_sub(o.observed_at)))
    {
        return HealthStatus::Unknown;
    }
    if matches!(
        state.to_ascii_lowercase().as_str(),
        "created" | "exited" | "dead" | "paused" | "restarting" | "removing"
    ) {
        return HealthStatus::NotRunning;
    }
    observation.map_or(HealthStatus::Unknown, |o| o.status(state, available))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WorkloadHealthStatus {
    Healthy,
    Unhealthy,
    Starting,
    NoCheck,
    Disabled,
    NotRunning,
    Partial,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthAttention {
    Unhealthy,
    RuntimeMissing,
    UnexpectedExit,
    RuntimeUnavailable,
    ObservationStale,
    OperationFailed,
    OwnershipConflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthMember {
    /// Internal persisted intent; never inferred from an observed Exited state.
    #[serde(default, skip_serializing)]
    pub stop_requested_at: Option<i64>,
    #[serde(default = "unknown_health_status")]
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

impl HealthMember {
    pub fn intentionally_inactive(&self, now: i64) -> bool {
        if !self.available {
            return false;
        }
        if self.state.eq_ignore_ascii_case("paused") {
            return true;
        }
        self.state.eq_ignore_ascii_case("exited")
            && self.stop_requested_at.is_some_and(|requested| {
                requested <= now
                    && self.observation.as_ref().is_some_and(|observation| {
                        (0..=HEALTH_MAX_AGE_MS)
                            .contains(&now.saturating_sub(observation.observed_at))
                            && observation
                                .started_at
                                .as_deref()
                                .and_then(|started| {
                                    chrono::DateTime::parse_from_rfc3339(started).ok()
                                })
                                .is_some_and(|started| started.timestamp_millis() <= requested)
                    })
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
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
    /// Earliest expiry of the evidence supporting counts and complete coverage.
    pub valid_until: Option<i64>,
    /// Latest expiry of a failing/starting member, including omitted members.
    pub unhealthy_until: Option<i64>,
    pub starting_until: Option<i64>,
    pub reason: String,
    pub attention: Vec<HealthAttention>,
    pub members: Vec<HealthMember>,
}

/// Supplied by the owner using applied topology, never an editable desired spec.
#[derive(Debug, Clone)]
pub struct HealthRequirement {
    pub service: String,
    pub replicas: usize,
    pub completed_job: bool,
}

pub struct HealthAssessmentInput {
    pub requirements: Option<Vec<HealthRequirement>>,
    pub members: Vec<HealthMember>,
    pub available: bool,
    pub inactive: bool,
    pub progressing: bool,
    pub operation_failed: bool,
    pub ownership_conflict: bool,
}

pub fn assess_health(mut input: HealthAssessmentInput, now: i64) -> WorkloadHealth {
    let mut result = WorkloadHealth {
        evaluated_at: now,
        ..Default::default()
    };
    result.expected = input
        .requirements
        .as_ref()
        .map(|rs| rs.iter().map(|r| r.replicas).sum());
    // Older inventory can identify the owner without identifying its Compose service.
    // Such a member may satisfy a requirement; absence is not yet established.
    let unidentified_members = input.requirements.as_ref().is_some_and(|rs| {
        !rs.iter().any(|r| r.service.is_empty())
            && input.members.iter().any(|m| m.service.is_empty())
    });
    result.missing = input.requirements.as_ref().map(|rs| {
        rs.iter()
            .map(|r| {
                r.replicas.saturating_sub(
                    input
                        .members
                        .iter()
                        .filter(|m| m.service == r.service)
                        .count(),
                )
            })
            .sum()
    });
    if unidentified_members {
        result.missing = None;
    }
    let mut latest_valid_until = None;
    for member in &mut input.members {
        let requirement = input
            .requirements
            .as_ref()
            .and_then(|rs| rs.iter().find(|r| r.service == member.service));
        member.completed_job = input.available
            && member.available
            && member.observation.as_ref().is_some_and(|o| {
                (0..=HEALTH_MAX_AGE_MS).contains(&now.saturating_sub(o.observed_at))
            })
            && requirement.is_some_and(|r| r.completed_job)
            && member.state.eq_ignore_ascii_case("exited")
            && member
                .observation
                .as_ref()
                .is_some_and(|o| o.exit_code == Some(0));
        if member.completed_job {
            if let Some(observation) = &member.observation {
                let expiry = observation.observed_at.saturating_add(HEALTH_MAX_AGE_MS);
                result.valid_until = Some(result.valid_until.map_or(expiry, |old| old.min(expiry)));
            }
            member.status = HealthStatus::NotRunning;
            result.counts.completed += 1;
            continue;
        }
        let status = observed_status(
            member.observation.as_ref(),
            &member.state,
            input.available && member.available,
            now,
        );
        member.status = status;
        if status != HealthStatus::Unknown
            && let Some(observation) = &member.observation
        {
            let expiry = observation.observed_at.saturating_add(HEALTH_MAX_AGE_MS);
            result.valid_until = Some(result.valid_until.map_or(expiry, |old| old.min(expiry)));
            let priority_expiry = match status {
                HealthStatus::Unhealthy => Some(&mut result.unhealthy_until),
                HealthStatus::Starting => Some(&mut result.starting_until),
                _ => None,
            };
            if let Some(until) = priority_expiry {
                *until = Some(until.map_or(expiry, |old| old.max(expiry)));
            }
        }
        match status {
            HealthStatus::Healthy => result.counts.healthy += 1,
            HealthStatus::Unhealthy => result.counts.unhealthy += 1,
            HealthStatus::Starting => result.counts.starting += 1,
            HealthStatus::NoCheck => result.counts.no_check += 1,
            HealthStatus::Disabled => result.counts.disabled += 1,
            HealthStatus::NotRunning => result.counts.not_running += 1,
            HealthStatus::Unknown => result.counts.unknown += 1,
        }
        if status != HealthStatus::Unknown
            && let Some(o) = &member.observation
        {
            latest_valid_until =
                Some(latest_valid_until.map_or(o.observed_at, |t: i64| t.min(o.observed_at)));
        }
    }
    result.observed_at = latest_valid_until;
    let c = &result.counts;
    let total = input.members.len();
    result.coverage_complete = input.available
        && input.requirements.is_some()
        && result.missing == Some(0)
        && c.unknown == 0
        && result.expected == Some(total);
    let inactive = input.inactive
        || (result.coverage_complete
            && total > 0
            && input
                .members
                .iter()
                .all(|member| member.intentionally_inactive(now) || member.completed_job));
    result.status = if !input.available {
        WorkloadHealthStatus::Unknown
    } else if c.unhealthy > 0 {
        WorkloadHealthStatus::Unhealthy
    } else if c.starting > 0 {
        WorkloadHealthStatus::Starting
    } else if inactive && c.healthy + c.no_check + c.disabled == 0 {
        WorkloadHealthStatus::NotRunning
    } else if total == 0 || c.unknown == total {
        WorkloadHealthStatus::Unknown
    } else if !result.coverage_complete {
        WorkloadHealthStatus::Partial
    } else if c.completed == total {
        WorkloadHealthStatus::NotRunning
    } else if c.healthy + c.completed == total {
        WorkloadHealthStatus::Healthy
    } else if c.no_check + c.completed == total {
        WorkloadHealthStatus::NoCheck
    } else if c.disabled + c.completed == total {
        WorkloadHealthStatus::Disabled
    } else if c.not_running + c.completed == total {
        WorkloadHealthStatus::NotRunning
    } else {
        WorkloadHealthStatus::Partial
    };
    if !input.available {
        result.attention.push(HealthAttention::RuntimeUnavailable);
    }
    if c.unhealthy > 0 {
        result.attention.push(HealthAttention::Unhealthy);
    }
    if c.unknown > 0 && input.available {
        result.attention.push(HealthAttention::ObservationStale);
    }
    if !inactive && !input.progressing && result.missing.is_some_and(|n| n > 0) {
        result.attention.push(HealthAttention::RuntimeMissing);
    }
    if !inactive
        && !input.progressing
        && input.members.iter().any(|member| {
            member.status == HealthStatus::NotRunning
                && !member.completed_job
                && !member.intentionally_inactive(now)
        })
    {
        result.attention.push(HealthAttention::UnexpectedExit);
    }
    if input.operation_failed {
        result.attention.push(HealthAttention::OperationFailed);
    }
    if input.ownership_conflict {
        result.attention.push(HealthAttention::OwnershipConflict);
    }
    result.reason = if !input.available {
        "Runtime observations are unavailable."
    } else if input.requirements.is_none() {
        "Applied membership is not fully known; coverage is partial."
    } else if unidentified_members {
        "Some containers have no observed service identity; coverage is partial until inventory is reconciled."
    } else if inactive && (total == 0 || c.completed != total) {
        "The applied workload is inactive."
    } else if result.missing.is_some_and(|n| n > 0) {
        "Some required instances have not been observed."
    } else if c.unknown > 0 {
        "Some observations are missing, stale or unsupported."
    } else if c.completed == total && total > 0 {
        "The applied one-shot jobs completed successfully."
    } else if c.unhealthy > 0 {
        "One or more current containers have failing health checks."
    } else if c.healthy + c.completed == total && total > 0 {
        "All required active containers have fresh passing checks."
    } else {
        "Health reflects the checks configured on the current containers."
    }
    .into();
    result.observed = input.members.len();
    // Bound list/detail payloads, retaining actionable members first. Counts cover all members.
    input.members.sort_by_key(|m| match m.status {
        HealthStatus::Unhealthy => 0,
        HealthStatus::Unknown => 1,
        HealthStatus::Starting => 2,
        HealthStatus::NotRunning => 3,
        HealthStatus::Healthy => 4,
        HealthStatus::NoCheck => 5,
        HealthStatus::Disabled => 6,
    });
    result.members_truncated = input.members.len() > 200;
    input.members.truncate(200);
    result.members = input.members;
    result
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppliedServiceHealth {
    pub service: String,
    pub mode: String,
    pub desired: usize,
    pub running: usize,
    pub stale: bool,
    pub update_state: String,
    pub ownership: String,
    #[serde(default)]
    pub stack_service_count: Option<String>,
    #[serde(default)]
    pub release_id: Option<String>,
}

#[cfg(test)]
#[path = "health_tests.rs"]
mod tests;
