use super::runtime::{AgentUpdateRuntimeRouter, error};
use crate::{
    connectors::{
        agent::execution::unary,
        edge::{EdgeRegistry, EdgeTarget},
    },
    persistence::postgres::updates::agent_inventory,
};
use citadel_contracts::citadel::{
    edge::v1::EdgeCommandKind,
    platforms::v1::{AgentUpdateDrainRequest, AgentUpdateInfo},
};
use citadel_platforms::node_agents::setup::{NodeAgentSetupService, SetupKind, SetupOptions};
use citadel_updates::{AgentUpdateRelease, AgentUpdateTarget, UpdateError};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

impl AgentUpdateRuntimeRouter {
    pub(super) async fn install_swarm(
        &self,
        target: &AgentUpdateTarget,
        release: &AgentUpdateRelease,
        operation: Uuid,
        cancel: &CancellationToken,
    ) -> Result<(), UpdateError> {
        let nodes = &target.drain_nodes;
        if *nodes != agent_inventory::swarm_nodes(&self.pool, target.resource_id).await? {
            return Err(error("Swarm node membership changed after review"));
        }
        let bounded = cancel.child_token();
        let cancel = &bounded;
        let work = async {
            for node in nodes {
                let session = self
                    .edge
                    .get(&EdgeTarget::node(target.resource_id, node.clone()))
                    .map_err(error)?;
                unary::<_, ()>(
                    &session,
                    EdgeCommandKind::PlatformAgentUpdateDrain,
                    AgentUpdateDrainRequest {
                        operation_id: operation.to_string(),
                        resume: false,
                        ..Default::default()
                    },
                    cancel,
                )
                .await
                .map_err(error)?;
            }
            let options = SetupOptions {
                policy: self.swarm_options.policy.clone(),
                core_url: self.swarm_options.core_url.clone(),
                image: release.image.clone(),
                ca_bundle: self.swarm_options.ca_bundle.clone(),
            };
            let (sender, mut receiver) = tokio::sync::mpsc::channel(32);
            let run = NodeAgentSetupService {
                store: self.swarm_store.clone(),
                runtime: self.swarm_runtime.clone(),
                changed: self.changed.clone(),
            };
            let ((), result) = tokio::join!(
                run.run(
                    citadel_primitives::ActorId::new(citadel_identity::SYSTEM_ACTOR_ID),
                    target.resource_id,
                    SetupKind::Upgrade,
                    options,
                    sender,
                    cancel.clone()
                ),
                async {
                    let mut outcome = None;
                    while let Some(item) = receiver.recv().await {
                        if item.is_completed {
                            outcome = Some(item.error_message);
                        }
                    }
                    match outcome {
                        Some(None) => Ok(()),
                        Some(Some(message)) => Err(error(message)),
                        None => Err(error("Node-agent rollout did not complete")),
                    }
                }
            );
            result
        };
        tokio::pin!(work);
        tokio::select! {
            result = &mut work => result,
            () = tokio::time::sleep(std::time::Duration::from_secs(600)) => {
                bounded.cancel();
                // Let the existing operation owner persist cancellation and finish cleanup.
                let _ = work.await;
                Err(error("Node-agent drain or rollout timed out"))
            }
        }
    }
    pub(super) async fn resume_swarm(
        &self,
        target: &AgentUpdateTarget,
        operation: Uuid,
        cancel: &CancellationToken,
    ) -> Result<(), UpdateError> {
        // Keep cleanup scoped to the saved selection even when a node leaves live inventory.
        let nodes = if target.drain_nodes.is_empty() {
            agent_inventory::swarm_nodes(&self.pool, target.resource_id).await?
        } else {
            target.drain_nodes.clone()
        };
        resume_nodes(&self.edge, target.resource_id, &nodes, operation, cancel).await
    }
    pub(super) async fn observe_swarm(
        &self,
        registration: &agent_inventory::Registration,
        cancel: &CancellationToken,
    ) -> Result<AgentUpdateTarget, UpdateError> {
        let mut target = agent_inventory::swarm(&self.pool, registration).await?;
        if !target.connected {
            return Ok(target);
        }
        // This additive, read-only RPC also proves the node supports the bounded drain command.
        // Older node Agents remain visible with explicit manual recovery guidance.
        for node in agent_inventory::swarm_nodes(&self.pool, registration.id).await? {
            let session = self
                .edge
                .get(&EdgeTarget::node(registration.id, node))
                .map_err(error)?;
            match unary::<_, AgentUpdateInfo>(
                &session,
                EdgeCommandKind::PlatformAgentUpdateInfo,
                (),
                cancel,
            )
            .await
            {
                Ok(info) if Some(&info.version) == target.installed_version.as_ref() => {}
                Ok(_) => {
                    target.installed_version = None;
                    target.reason = Some("Node Agents report different versions; reconcile the service before updating.".into());
                }
                Err(_) => {
                    target.supported = false;
                    target.reason = Some("Manual update required: node Agents do not support central update draining.".into());
                    break;
                }
            }
        }
        Ok(target)
    }
}

/// Missing sessions and lost acknowledgements are unresolved cleanup, never success.
pub(super) async fn resume_nodes(
    edge: &EdgeRegistry,
    platform: Uuid,
    nodes: &[String],
    operation: Uuid,
    cancel: &CancellationToken,
) -> Result<(), UpdateError> {
    use futures_util::StreamExt;
    let results = futures_util::stream::iter(nodes.iter().cloned())
        .map(|node| async move {
            let session = edge
                .get(&EdgeTarget::node(platform, node.clone()))
                .map_err(error)?;
            tokio::time::timeout(
                std::time::Duration::from_secs(5),
                unary::<_, ()>(
                    &session,
                    EdgeCommandKind::PlatformAgentUpdateDrain,
                    AgentUpdateDrainRequest {
                        operation_id: operation.to_string(),
                        resume: true,
                        ..Default::default()
                    },
                    cancel,
                ),
            )
            .await
            .map_err(|_| error("Agent admission resume was not acknowledged"))?
            .map_err(error)
        })
        .buffer_unordered(8)
        .collect::<Vec<_>>()
        .await;
    if cancel.is_cancelled() {
        return Err(UpdateError::Cancelled);
    }
    results.into_iter().collect::<Result<Vec<_>, _>>()?;
    Ok(())
}
