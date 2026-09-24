use super::identity::invalid;
use crate::config::{EdgeConfig, EdgeProfile};
use citadel_adapters::connectors::docker::{DockerClient, projection::DockerInfo};
use citadel_contracts::citadel::edge::v1::AgentHeartbeat;
use std::{io, time::Duration};

#[derive(Clone)]
pub(super) struct Observation {
    pub daemon_id: String,
    pub hostname: String,
    pub version: String,
    pub cluster_id: String,
    pub node_id: String,
    pub swarm_role: String,
}
impl Observation {
    pub async fn read(docker: &DockerClient, config: &EdgeConfig) -> io::Result<Self> {
        let info = tokio::time::timeout(Duration::from_secs(10), docker.info())
            .await
            .map_err(|_| io::Error::other("Reading Edge Docker identity timed out"))?
            .map_err(|_| io::Error::other("Could not read Edge Docker identity"))?;
        Self::from_info(info, config)
    }
    fn from_info(info: DockerInfo, config: &EdgeConfig) -> io::Result<Self> {
        if info.id.trim().is_empty() || info.hostname.trim().is_empty() {
            return Err(invalid("Docker returned an incomplete Edge identity"));
        }
        let mut result = Self {
            daemon_id: info.id.trim().into(),
            hostname: info.hostname.trim().into(),
            version: info.server_version,
            cluster_id: String::new(),
            node_id: String::new(),
            swarm_role: String::new(),
        };
        if let EdgeProfile::SwarmNode(node) = &config.profile {
            let swarm = info.swarm.ok_or_else(|| {
                invalid("Swarm-node profile requires active Docker Swarm membership")
            })?;
            if !swarm.local_node_state.eq_ignore_ascii_case("active")
                || swarm.node_id != node.node_id
                || result.hostname != node.node_hostname
                || swarm
                    .cluster
                    .as_ref()
                    .is_some_and(|v| !v.id.is_empty() && v.id != node.cluster_id)
            {
                return Err(invalid(
                    "Docker Swarm identity does not match the injected node identity",
                ));
            }
            result.cluster_id = node.cluster_id.clone();
            result.node_id = swarm.node_id;
            result.swarm_role = if swarm.control_available {
                "manager"
            } else {
                "worker"
            }
            .into();
        }
        Ok(result)
    }
    pub fn heartbeat(&self, config: &EdgeConfig, reachable: bool) -> AgentHeartbeat {
        let (service_id, task_id) = task_identity(config);
        AgentHeartbeat {
            docker_reachable: reachable,
            docker_version: if reachable {
                self.version.clone()
            } else {
                String::new()
            },
            hostname: self.hostname.clone(),
            agent_version: super::version(),
            capabilities_json: capabilities(config),
            daemon_id: self.daemon_id.clone(),
            cluster_id: self.cluster_id.clone(),
            node_id: self.node_id.clone(),
            docker_hostname: self.hostname.clone(),
            service_id,
            task_id,
            swarm_role: self.swarm_role.clone(),
        }
    }
}
pub(super) fn task_identity(config: &EdgeConfig) -> (String, String) {
    match &config.profile {
        EdgeProfile::SwarmNode(node) => (node.service_id.clone(), node.task_id.clone()),
        _ => Default::default(),
    }
}
pub(super) fn capabilities(config: &EdgeConfig) -> String {
    let commands: &[&str] = if matches!(config.profile, EdgeProfile::SwarmNode(_)) {
        &[
            "platform.checkHealth",
            "platform.getInfo",
            "platform.stats",
            "platform.events",
            "containers.list",
            "containers.logs",
            "containers.inspect",
            "containers.patch",
            "containers.delete",
            "containers.stats",
            "containers.exec",
            "containers.createHelper",
            "containers.execBinaryHelper",
            "images.list",
            "images.inspect",
            "volumes.list",
            "volumes.inspect",
            "networks.list",
            "networks.inspect",
        ]
    } else {
        &[
            "platform.checkHealth",
            "platform.getInfo",
            "platform.stats",
            "platform.events",
            "platform.prune",
            "containers.list",
            "containers.logs",
            "containers.inspect",
            "containers.create",
            "containers.patch",
            "containers.delete",
            "containers.stats",
            "containers.exec",
            "containers.execBinary",
            "images.get",
            "images.list",
            "images.inspect",
            "images.delete",
            "images.history",
            "images.exposedPorts",
            "images.distributionInspect",
            "images.pull",
            "images.build",
            "images.push",
            "images.checkBuildHost",
            "volumes.list",
            "volumes.inspect",
            "volumes.create",
            "volumes.delete",
            "networks.list",
            "networks.inspect",
            "networks.create",
            "networks.delete",
            "stacks.apply",
            "deployments.apply",
            "swarm.nodes.list",
            "swarm.nodes.inspect",
            "swarm.services.list",
            "swarm.services.inspect",
            "swarm.services.logs",
            "swarm.tasks.list",
            "swarm.tasks.inspect",
            "swarm.tasks.logs",
            "swarm.networks.list",
            "swarm.networks.inspect",
            "swarm.secrets.list",
            "swarm.secrets.inspect",
            "swarm.configs.list",
            "swarm.configs.inspect",
        ]
    };
    serde_json::json!({"commands": commands}).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn swarm_observation_checks_injected_identity_and_accepts_worker_without_cluster_document() {
        let config =
            super::super::tests::config(EdgeProfile::SwarmNode(crate::config::SwarmIdentity {
                bootstrap_file: "bootstrap".into(),
                platform_id: uuid::Uuid::now_v7().to_string(),
                service_id: "service".into(),
                task_id: "task".into(),
                node_id: "node".into(),
                node_hostname: "worker".into(),
                cluster_id: "cluster".into(),
            }));
        let mut info = DockerInfo {
            id: "daemon".into(),
            hostname: "worker".into(),
            swarm: Some(
                citadel_adapters::connectors::docker::projection::DockerSwarmInfo {
                    node_id: "node".into(),
                    local_node_state: "active".into(),
                    ..Default::default()
                },
            ),
            ..Default::default()
        };
        let observed = Observation::from_info(info.clone(), &config).unwrap();
        assert_eq!(observed.cluster_id, "cluster");
        assert_eq!(observed.swarm_role, "worker");
        info.swarm.as_mut().unwrap().node_id = "wrong".into();
        assert!(Observation::from_info(info, &config).is_err());
        let capabilities = capabilities(&config);
        assert!(capabilities.contains("containers.createHelper"));
        assert!(!capabilities.contains("images.build"));
    }
}
