use crate::api::resources::platforms::views::{PlatformCapabilitiesView, WorkloadStatusCounts};
use citadel_platforms::swarm_overview::SwarmSummary;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmQuorumView {
    #[schema(value_type = crate::openapi::compatibility::SwarmQuorumState)]
    pub state: &'static str,
    pub reachable_managers: i64,
    pub required_managers: i64,
    pub has_leader: bool,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmOverviewView {
    pub platform_id: Uuid,
    pub health: &'static str,
    #[schema(required = true)]
    pub message: Option<String>,
    pub is_stale: bool,
    pub node_count: i64,
    pub manager_count: i64,
    pub quorum: SwarmQuorumView,
    pub service_count: i64,
    pub service_status_counts: WorkloadStatusCounts,
    pub running_task_count: i64,
    pub desired_task_count: i64,
    pub network_count: i64,
    pub local_network_count: i64,
    pub volume_count: i64,
    pub image_count: i64,
    pub capabilities: PlatformCapabilitiesView,
}

pub fn overview(
    summary: SwarmSummary,
    platform: Uuid,
    online: bool,
    control: bool,
    error: Option<&str>,
    capabilities: PlatformCapabilitiesView,
) -> SwarmOverviewView {
    let required = if summary.manager_count == 0 {
        0
    } else {
        summary.manager_count / 2 + 1
    };
    let quorum = if !online || summary.manager_count == 0 || summary.managers_stale {
        "Unknown"
    } else if !summary.has_leader || summary.reachable_managers < required {
        "Lost"
    } else if summary.reachable_managers < summary.manager_count {
        "Degraded"
    } else {
        "Healthy"
    };
    let health = if !online {
        "Offline"
    } else if summary.is_stale {
        "Stale"
    } else if quorum != "Healthy" || !control || error.is_some_and(|e| !e.trim().is_empty()) {
        "Degraded"
    } else {
        "Healthy"
    };
    let message = match (health, quorum) {
        ("Offline", _) => {
            Some("The Swarm manager is offline. Last-known inventory remains available.".into())
        }
        ("Stale", _) => Some(
            "The latest inventory refresh failed. Last-known inventory may be out of date.".into(),
        ),
        ("Degraded", "Lost") => Some(format!(
            "Swarm manager quorum is unavailable: {} of {} managers are reachable and {} are required.",
            summary.reachable_managers, summary.manager_count, required
        )),
        ("Degraded", "Degraded") => Some(format!(
            "Swarm manager quorum is available, but only {} of {} managers are reachable.",
            summary.reachable_managers, summary.manager_count
        )),
        ("Degraded", "Unknown") => {
            Some("Swarm manager quorum cannot be confirmed from the current Node inventory.".into())
        }
        ("Degraded", _) => Some(
            error
                .unwrap_or(
                    "The connected Docker node does not currently expose Swarm manager control.",
                )
                .into(),
        ),
        _ => None,
    };
    SwarmOverviewView {
        platform_id: platform,
        health,
        message,
        is_stale: summary.is_stale,
        node_count: summary.node_count,
        manager_count: summary.manager_count,
        quorum: SwarmQuorumView {
            state: quorum,
            reachable_managers: summary.reachable_managers,
            required_managers: required,
            has_leader: summary.has_leader,
        },
        service_count: summary.service_counts.total,
        service_status_counts: summary.service_counts.into(),
        running_task_count: summary.running_tasks,
        desired_task_count: summary.desired_tasks,
        network_count: summary.network_count,
        local_network_count: summary.local_network_count,
        volume_count: summary.volume_count,
        image_count: summary.image_count,
        capabilities,
    }
}

#[cfg(test)]
mod tests {
    use crate::api::resources::platforms::swarm_views::*;

    #[test]
    fn quorum_and_health_follow_dotnet_majority_and_staleness_rules() {
        for (managers, reachable, leader, stale, online, quorum, health) in [
            (0, 0, false, false, true, "Unknown", "Degraded"),
            (1, 1, true, false, true, "Healthy", "Healthy"),
            (3, 2, true, false, true, "Degraded", "Degraded"),
            (3, 1, true, false, true, "Lost", "Degraded"),
            (3, 3, false, false, true, "Lost", "Degraded"),
            (3, 3, true, true, true, "Unknown", "Stale"),
            (3, 3, true, false, false, "Unknown", "Offline"),
        ] {
            let result = overview(
                SwarmSummary {
                    manager_count: managers,
                    reachable_managers: reachable,
                    has_leader: leader,
                    is_stale: stale,
                    managers_stale: stale,
                    ..Default::default()
                },
                Uuid::nil(),
                online,
                true,
                None,
                Default::default(),
            );
            assert_eq!((result.quorum.state, result.health), (quorum, health));
            assert_eq!(
                result.quorum.required_managers,
                if managers == 0 { 0 } else { managers / 2 + 1 }
            );
        }
    }
}
