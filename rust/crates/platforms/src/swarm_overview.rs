use crate::{PlatformCapabilitiesView, WorkloadStatusCounts};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Default)]
pub struct SwarmSummary {
    pub is_stale: bool,
    pub node_count: i64,
    pub manager_count: i64,
    pub reachable_managers: i64,
    pub has_leader: bool,
    pub managers_stale: bool,
    pub service_counts: WorkloadStatusCounts,
    pub running_tasks: i64,
    pub desired_tasks: i64,
    pub network_count: i64,
    pub local_network_count: i64,
    pub volume_count: i64,
    pub image_count: i64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmQuorumView {
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

impl SwarmSummary {
    pub fn into_view(
        self,
        platform: Uuid,
        online: bool,
        control: bool,
        error: Option<&str>,
        capabilities: PlatformCapabilitiesView,
    ) -> SwarmOverviewView {
        let required = if self.manager_count == 0 {
            0
        } else {
            self.manager_count / 2 + 1
        };
        let quorum = if !online || self.manager_count == 0 || self.managers_stale {
            "Unknown"
        } else if !self.has_leader || self.reachable_managers < required {
            "Lost"
        } else if self.reachable_managers < self.manager_count {
            "Degraded"
        } else {
            "Healthy"
        };
        let health = if !online {
            "Offline"
        } else if self.is_stale {
            "Stale"
        } else if quorum != "Healthy" || !control || error.is_some_and(|e| !e.trim().is_empty()) {
            "Degraded"
        } else {
            "Healthy"
        };
        let message = match (health, quorum) {
            ("Offline", _) => Some("The Swarm manager is offline. Last-known inventory remains available.".into()),
            ("Stale", _) => Some("The latest inventory refresh failed. Last-known inventory may be out of date.".into()),
            ("Degraded", "Lost") => Some(format!("Swarm manager quorum is unavailable: {} of {} managers are reachable and {} are required.", self.reachable_managers, self.manager_count, required)),
            ("Degraded", "Degraded") => Some(format!("Swarm manager quorum is available, but only {} of {} managers are reachable.", self.reachable_managers, self.manager_count)),
            ("Degraded", "Unknown") => Some("Swarm manager quorum cannot be confirmed from the current Node inventory.".into()),
            ("Degraded", _) => Some(error.unwrap_or("The connected Docker node does not currently expose Swarm manager control.").into()),
            _ => None,
        };
        SwarmOverviewView {
            platform_id: platform,
            health,
            message,
            is_stale: self.is_stale,
            node_count: self.node_count,
            manager_count: self.manager_count,
            quorum: SwarmQuorumView {
                state: quorum,
                reachable_managers: self.reachable_managers,
                required_managers: required,
                has_leader: self.has_leader,
            },
            service_count: self.service_counts.total,
            service_status_counts: self.service_counts,
            running_task_count: self.running_tasks,
            desired_task_count: self.desired_tasks,
            network_count: self.network_count,
            local_network_count: self.local_network_count,
            volume_count: self.volume_count,
            image_count: self.image_count,
            capabilities,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
            let result = SwarmSummary {
                manager_count: managers,
                reachable_managers: reachable,
                has_leader: leader,
                is_stale: stale,
                managers_stale: stale,
                ..Default::default()
            }
            .into_view(Uuid::nil(), online, true, None, Default::default());
            assert_eq!((result.quorum.state, result.health), (quorum, health));
            assert_eq!(
                result.quorum.required_managers,
                if managers == 0 { 0 } else { managers / 2 + 1 }
            );
        }
    }
}
