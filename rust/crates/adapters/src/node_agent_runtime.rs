use crate::{
    agent::AgentClient,
    agent_execution::{AgentExecutionClient, unary},
    docker::DockerClient,
    edge::{EdgeRegistry, EdgeRuntime, EdgeTarget},
};
use citadel_contracts::citadel::{edge::v1::EdgeCommandKind, swarm::v1::*};
use citadel_platforms::{node_agents::lifecycle::*, *};
use futures_util::future::BoxFuture;
use sqlx::PgPool;
use std::{collections::BTreeMap, sync::Arc};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub struct NodeAgentRuntimeRouter {
    pub pool: PgPool,
    pub docker: DockerClient,
    pub agent: Option<AgentClient>,
    pub edge: EdgeRegistry,
}
#[path = "node_agent_setup_runtime.rs"]
mod setup;
enum Target {
    Local(DockerClient),
    Agent(Arc<AgentClient>),
    Edge(EdgeRuntime),
}
impl Target {
    fn inventory(&self) -> &dyn PlatformInventoryPort {
        match self {
            Self::Local(r) => r,
            Self::Agent(r) => r.as_ref(),
            Self::Edge(r) => r,
        }
    }
}
impl NodeAgentRuntimeRouter {
    async fn target(&self, id: Uuid) -> Result<Target, RuntimeCapabilityError> {
        let (kind, address): (String, String) =
            sqlx::query_as("SELECT connectortype,address FROM platforms WHERE id=$1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage)?
                .ok_or_else(|| failure("Platform not found."))?;
        match kind.as_str() {
            "Local" => Ok(Target::Local(self.docker.clone())),
            "Agent" => self
                .agent
                .as_ref()
                .filter(|a| a.address().trim_end_matches('/') == address.trim_end_matches('/'))
                .map(|a| Target::Agent(Arc::new(a.clone())))
                .ok_or_else(|| failure("The configured Agent is unavailable.")),
            "EdgeAgent" => self
                .edge
                .get(&EdgeTarget::platform(id))
                .map(|session| Target::Edge(EdgeRuntime { session }))
                .map_err(|_| failure("The bound Edge manager is unavailable.")),
            _ => Err(failure("Unsupported manager connector.")),
        }
    }
}
impl NodeAgentLifecycleRuntime for NodeAgentRuntimeRouter {
    fn info<'a>(
        &'a self,
        id: Uuid,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimePlatformInfo, RuntimeCapabilityError>> {
        Box::pin(async move { self.target(id).await?.inventory().get_info(cancel).await })
    }
    fn delete_owned<'a>(
        &'a self,
        claim: &'a NodeAgentRemovalClaim,
        kind: NodeAgentResource,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move {
            let target = self.target(claim.platform_id).await?;
            let info = target.inventory().get_info(cancel).await?;
            if info.daemon_id != claim.manager_daemon_id
                || !info.swarm.as_ref().is_some_and(|s| {
                    s.control_available
                        && s.local_node_state.eq_ignore_ascii_case("active")
                        && s.node_id == claim.manager_node_id
                        && s.cluster_id.as_deref() == Some(&claim.cluster_id)
                })
            {
                return Err(failure("Pinned manager identity changed during removal."));
            }
            let labels: Result<BTreeMap<String, String>, RuntimeCapabilityError> = match &target {
                Target::Local(d) => match kind {
                    NodeAgentResource::Service => d
                        .inspect_swarm_service(id)
                        .await
                        .map(|s| labels(s.spec))
                        .map_err(docker_error),
                    NodeAgentResource::Secret => d
                        .inspect_swarm_secret(id)
                        .await
                        .map(|s| labels(s.spec))
                        .map_err(docker_error),
                    NodeAgentResource::Config => d
                        .inspect_swarm_config(id)
                        .await
                        .map(|s| labels(s.spec))
                        .map_err(docker_error),
                },
                Target::Agent(a) => a.inspect_node_agent_resource(kind, id, cancel).await,
                Target::Edge(e) => match kind {
                    NodeAgentResource::Service => unary::<_, SwarmServiceMessage>(
                        &e.session,
                        EdgeCommandKind::SwarmServiceInspect,
                        InspectSwarmServiceRequest {
                            service_id: id.into(),
                        },
                        cancel,
                    )
                    .await
                    .map(|r| r.labels.into_iter().collect()),
                    NodeAgentResource::Secret => unary::<_, SwarmSecretMessage>(
                        &e.session,
                        EdgeCommandKind::SwarmSecretInspect,
                        InspectSwarmSecretRequest {
                            secret_id: id.into(),
                        },
                        cancel,
                    )
                    .await
                    .map(|r| r.labels.into_iter().collect()),
                    NodeAgentResource::Config => unary::<_, SwarmConfigMessage>(
                        &e.session,
                        EdgeCommandKind::SwarmConfigInspect,
                        InspectSwarmConfigRequest {
                            config_id: id.into(),
                        },
                        cancel,
                    )
                    .await
                    .map(|r| r.labels.into_iter().collect()),
                },
            };
            let labels = match labels {
                Err(e) if e.kind == RuntimeErrorKind::NotFound => return Ok(()),
                other => other?,
            };
            if !owned(&labels, claim.platform_id, &claim.cluster_id) {
                return Err(failure(
                    "Refusing to delete a resource without matching Citadel node-agent ownership labels.",
                ));
            }
            let result = match target {
                Target::Local(d) => match kind {
                    NodeAgentResource::Service => d.delete_swarm_service(id).await,
                    NodeAgentResource::Secret => d.delete_swarm_secret(id).await,
                    NodeAgentResource::Config => d.delete_swarm_config(id).await,
                }
                .map_err(docker_error),
                Target::Agent(a) => {
                    a.delete_node_agent_resource(kind, id, claim.operation_id, cancel)
                        .await
                }
                Target::Edge(e) => match kind {
                    NodeAgentResource::Service => {
                        AgentExecutionClient::Edge(e.session)
                            .delete_managed_swarm_service(
                                DeleteManagedSwarmServiceRequest {
                                    service_id: id.into(),
                                    operation_id: claim.operation_id.to_string(),
                                },
                                cancel,
                            )
                            .await
                    }
                    NodeAgentResource::Secret => {
                        unary(
                            &e.session,
                            EdgeCommandKind::SwarmSecretDelete,
                            DeleteSwarmSecretRequest {
                                secret_id: id.into(),
                            },
                            cancel,
                        )
                        .await
                    }
                    NodeAgentResource::Config => {
                        unary(
                            &e.session,
                            EdgeCommandKind::SwarmConfigDelete,
                            DeleteSwarmConfigRequest {
                                config_id: id.into(),
                            },
                            cancel,
                        )
                        .await
                    }
                },
            };
            match result {
                Err(e) if e.kind == RuntimeErrorKind::NotFound => Ok(()),
                other => other,
            }
        })
    }
    fn disconnect(&self, id: Uuid, nodes: &[String]) {
        for node in nodes {
            self.edge.disconnect(&EdgeTarget::node(id, node.clone()));
        }
    }
}
fn owned(labels: &BTreeMap<String, String>, id: Uuid, cluster: &str) -> bool {
    [
        ("com.citadel.system", "true"),
        ("com.citadel.system-role", "swarm-node-agent"),
        ("com.citadel.platform-id", &id.to_string()),
        ("com.citadel.swarm-cluster-id", cluster),
    ]
    .iter()
    .all(|(k, v)| labels.get(*k).is_some_and(|label| label == v))
}
fn labels(spec: serde_json::Value) -> BTreeMap<String, String> {
    spec.get("Labels")
        .and_then(serde_json::Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(key, value)| value.as_str().map(|value| (key.clone(), value.to_owned())))
        .collect()
}
fn failure(message: &str) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Conflict, message, false)
}
fn storage(error: impl std::fmt::Display) -> RuntimeCapabilityError {
    tracing::error!(%error, "Node-agent manager lookup failed");
    RuntimeCapabilityError::new(
        RuntimeErrorKind::Remote,
        "Could not resolve the node-agent manager connection.",
        false,
    )
}
fn docker_error(error: crate::docker::DockerError) -> RuntimeCapabilityError {
    crate::docker::runtime::normalize_docker_error(error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ownership_requires_every_exact_label_and_matching_platform_and_cluster() {
        let platform = Uuid::now_v7();
        let expected = BTreeMap::from([
            ("com.citadel.system".into(), "true".into()),
            ("com.citadel.system-role".into(), "swarm-node-agent".into()),
            ("com.citadel.platform-id".into(), platform.to_string()),
            ("com.citadel.swarm-cluster-id".into(), "cluster".into()),
        ]);
        assert!(owned(&expected, platform, "cluster"));
        assert!(!owned(&expected, Uuid::now_v7(), "cluster"));
        assert!(!owned(&expected, platform, "other-cluster"));
        for key in expected.keys() {
            let mut incomplete = expected.clone();
            incomplete.remove(key);
            assert!(!owned(&incomplete, platform, "cluster"), "{key}");
        }
        let invalid = labels(serde_json::json!({"Labels":{"com.citadel.system":true}}));
        assert!(!owned(&invalid, platform, "cluster"));
    }
}
