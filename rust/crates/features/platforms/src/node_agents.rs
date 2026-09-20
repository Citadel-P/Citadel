//! Coverage is a read model, not proof that a mutation can be routed to a node.
//! Runtime operations still perform exact-node identity and freshness checks.
pub mod lifecycle;
pub mod setup;

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeAgentOperation {
    pub operation_id: Uuid,
    pub kind: String,
    pub state: String,
    pub started_at_utc: DateTime<Utc>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeAgentCoverage {
    pub docker_node_id: String,
    pub hostname: String,
    pub role: String,
    pub availability: String,
    pub node_status: String,
    pub architecture: String,
    pub data_source: &'static str,
    pub eligible: bool,
    pub supported: bool,
    pub schedulable: bool,
    pub service_task_state: Option<String>,
    pub agent_connection_state: &'static str,
    pub docker_reachable: bool,
    pub compatible: bool,
    pub projection_stale: bool,
    pub last_heartbeat_at_utc: Option<DateTime<Utc>>,
    pub last_successful_reconciliation_at: Option<DateTime<Utc>>,
    pub stale_since: Option<DateTime<Utc>>,
    pub stale_reason: Option<String>,
    pub reasons: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmNodeAgentCoverage {
    pub state: &'static str,
    pub is_installed: bool,
    pub covered_nodes: usize,
    pub eligible_nodes: usize,
    pub total_nodes: usize,
    pub connected_nodes: usize,
    pub offline_nodes: usize,
    pub enrolling_nodes: usize,
    pub missing_nodes: usize,
    pub incompatible_nodes: usize,
    pub unsupported_nodes: usize,
    pub unschedulable_nodes: usize,
    pub stale_nodes: usize,
    pub last_membership_reconciliation_at_utc: Option<DateTime<Utc>>,
    pub agent_image_reference: Option<String>,
    pub agent_image_digest: Option<String>,
    pub enrollment_expires_at_utc: Option<DateTime<Utc>>,
    pub operation: Option<NodeAgentOperation>,
    pub reasons: Vec<&'static str>,
    pub nodes: Vec<NodeAgentCoverage>,
    pub can_manage_node_agents: bool,
}

#[derive(Debug, Default)]
pub struct NodeCoverageInput {
    pub docker_node_id: String,
    pub hostname: String,
    pub role: String,
    pub availability: String,
    pub status: String,
    pub architecture: String,
    pub operating_system: String,
    pub manager_source: bool,
    pub membership_stale: bool,
    pub runtime_stale: bool,
    pub binding_present: bool,
    pub connected: bool,
    pub protocol_version: Option<i32>,
    pub task_state: Option<String>,
    pub heartbeat: Option<DateTime<Utc>>,
    pub reconciled_at: Option<DateTime<Utc>>,
    pub stale_since: Option<DateTime<Utc>>,
    pub stale_reason: Option<String>,
}

impl NodeCoverageInput {
    pub fn evaluate(self, now: DateTime<Utc>) -> NodeAgentCoverage {
        let linux = self.operating_system.eq_ignore_ascii_case("linux");
        let supported = linux
            && ["amd64", "x86_64", "arm64", "aarch64"]
                .iter()
                .any(|arch| self.architecture.trim().eq_ignore_ascii_case(arch));
        let active = self.availability.eq_ignore_ascii_case("active");
        let schedulable = active && self.status.eq_ignore_ascii_case("ready");
        let eligible = self.manager_source || (supported && active);
        let compatible = self.protocol_version.is_none_or(|version| version == 1);
        let stale = self.membership_stale
            || (!self.manager_source
                && (self.runtime_stale
                    || self
                        .heartbeat
                        .is_some_and(|heartbeat| heartbeat < now - chrono::Duration::minutes(2))));
        let reachable = self.connected
            && (self.manager_source || (!self.runtime_stale && self.reconciled_at.is_some()));
        let state = if self.manager_source {
            "ManagerConnector"
        } else if !supported {
            "Unsupported"
        } else if !active {
            "Unschedulable"
        } else if !compatible {
            "Incompatible"
        } else if self.connected && stale {
            "Stale"
        } else if self.connected {
            "Connected"
        } else if self.binding_present {
            "Offline"
        } else if self.task_state.is_some() {
            "Enrolling"
        } else {
            "Missing"
        };
        let mut reasons = Vec::new();
        if !supported {
            reasons.push(if linux {
                "UnsupportedArchitecture"
            } else {
                "UnsupportedOperatingSystem"
            });
        }
        if !schedulable {
            reasons.push("Unschedulable");
        }
        if eligible && !self.connected {
            reasons.push(if self.binding_present {
                "AgentOffline"
            } else {
                "AgentMissing"
            });
        }
        if !compatible {
            reasons.push("AgentIncompatible");
        }
        if stale {
            reasons.push("ProjectionStale");
        }
        if self.connected && !reachable {
            reasons.push("DockerRuntimeUnavailable");
        }
        NodeAgentCoverage {
            docker_node_id: self.docker_node_id,
            hostname: self.hostname,
            role: self.role,
            availability: self.availability,
            node_status: self.status,
            architecture: self.architecture,
            data_source: if self.manager_source {
                "ManagerConnector"
            } else {
                "Satellite"
            },
            eligible,
            supported,
            schedulable,
            service_task_state: self.task_state,
            agent_connection_state: state,
            docker_reachable: reachable,
            compatible,
            projection_stale: stale,
            last_heartbeat_at_utc: self.heartbeat,
            last_successful_reconciliation_at: self.reconciled_at,
            stale_since: self.stale_since,
            stale_reason: self.stale_reason,
            reasons,
        }
    }
}

impl SwarmNodeAgentCoverage {
    pub fn aggregate(
        nodes: Vec<NodeAgentCoverage>,
        installed: bool,
        drifted: bool,
        operation: Option<NodeAgentOperation>,
    ) -> Self {
        let count = |predicate: fn(&NodeAgentCoverage) -> bool| {
            nodes.iter().filter(|node| predicate(node)).count()
        };
        let eligible = count(|node| node.eligible);
        let covered = count(|node| {
            node.eligible && node.docker_reachable && !node.projection_stale && node.compatible
        });
        let requires_satellites = nodes
            .iter()
            .any(|node| node.eligible && node.data_source == "Satellite");
        let mut reasons = Vec::new();
        if nodes.is_empty() {
            reasons.push("SwarmInventoryUnavailable");
        }
        if drifted {
            reasons.push("NodeAgentServiceDrifted");
        }
        let state = if nodes.is_empty() {
            "Unavailable"
        } else if let Some(operation) = operation.as_ref().filter(|value| value.state == "Running")
        {
            if operation.kind == "Remove" {
                "Removing"
            } else {
                "Installing"
            }
        } else if drifted {
            if covered > 0 { "Partial" } else { "Failed" }
        } else if !requires_satellites && covered == eligible {
            "Complete"
        } else if !installed {
            "NotInstalled"
        } else if covered == eligible {
            "Complete"
        } else if covered > 0 {
            "Partial"
        } else {
            "Failed"
        };
        Self {
            state,
            is_installed: installed,
            covered_nodes: covered,
            eligible_nodes: eligible,
            total_nodes: nodes.len(),
            connected_nodes: count(|node| {
                matches!(
                    node.agent_connection_state,
                    "ManagerConnector" | "Connected" | "Stale"
                )
            }),
            offline_nodes: count(|node| node.agent_connection_state == "Offline"),
            enrolling_nodes: count(|node| node.agent_connection_state == "Enrolling"),
            missing_nodes: count(|node| node.agent_connection_state == "Missing"),
            incompatible_nodes: count(|node| !node.compatible),
            unsupported_nodes: count(|node| !node.supported),
            unschedulable_nodes: count(|node| !node.schedulable),
            stale_nodes: count(|node| node.projection_stale),
            last_membership_reconciliation_at_utc: None,
            agent_image_reference: None,
            agent_image_digest: None,
            enrollment_expires_at_utc: None,
            operation,
            reasons,
            nodes,
            can_manage_node_agents: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn node() -> NodeCoverageInput {
        NodeCoverageInput {
            operating_system: "linux".into(),
            architecture: "x86_64".into(),
            availability: "Active".into(),
            status: "Ready".into(),
            ..Default::default()
        }
    }
    #[test]
    fn pinned_manager_needs_no_satellite_installation() {
        let manager = NodeCoverageInput {
            manager_source: true,
            connected: true,
            runtime_stale: true,
            ..node()
        }
        .evaluate(Utc::now());
        let result = SwarmNodeAgentCoverage::aggregate(vec![manager], false, false, None);
        assert_eq!(result.state, "Complete");
        assert_eq!(result.covered_nodes, 1);
    }
    #[test]
    fn down_active_worker_stays_eligible_and_architecture_aliases_are_supported() {
        let worker = NodeCoverageInput {
            status: "Down".into(),
            runtime_stale: true,
            ..node()
        }
        .evaluate(Utc::now());
        assert!(worker.eligible && worker.supported && !worker.schedulable);
        assert_eq!(
            SwarmNodeAgentCoverage::aggregate(vec![worker], true, false, None).state,
            "Failed"
        );
        assert!(
            NodeCoverageInput {
                architecture: "aarch64".into(),
                ..node()
            }
            .evaluate(Utc::now())
            .supported
        );
    }
    #[test]
    fn connected_socket_without_fresh_runtime_or_protocol_is_not_coverage() {
        for input in [
            NodeCoverageInput {
                connected: true,
                runtime_stale: true,
                ..node()
            },
            NodeCoverageInput {
                connected: true,
                reconciled_at: Some(Utc::now()),
                protocol_version: Some(2),
                ..node()
            },
            NodeCoverageInput {
                connected: true,
                reconciled_at: Some(Utc::now()),
                heartbeat: Some(Utc::now() - chrono::Duration::minutes(3)),
                ..node()
            },
        ] {
            assert_eq!(
                SwarmNodeAgentCoverage::aggregate(
                    vec![input.evaluate(Utc::now())],
                    true,
                    false,
                    None
                )
                .covered_nodes,
                0
            );
        }
    }
}
