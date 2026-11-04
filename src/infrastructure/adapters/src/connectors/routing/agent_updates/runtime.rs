use crate::{
    connectors::{
        agent::{client::AgentClient, execution::AgentExecutionClient},
        edge::{EdgeRegistry, EdgeTarget},
    },
    external::updates::GithubReleaseArtifacts,
    persistence::postgres::updates::agent_inventory::{self, Registration},
};
use citadel_platforms::node_agents::setup::*;
use citadel_updates::*;
use futures_util::{StreamExt, future::BoxFuture};
use std::{sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub struct AgentUpdateRuntimeRouter {
    pub pool: sqlx::PgPool,
    pub platforms: super::super::platforms::runtime::PlatformRuntimeRouter,
    pub agent: Option<AgentClient>,
    pub edge: EdgeRegistry,
    pub artifacts: GithubReleaseArtifacts,
    pub swarm_store: Arc<dyn NodeAgentSetupStore>,
    pub swarm_runtime: Arc<dyn NodeAgentSetupRuntime>,
    pub swarm_options: SetupOptions,
    pub changed: Arc<dyn Fn(Uuid) + Send + Sync>,
}
pub(super) fn error(error: impl std::fmt::Display) -> UpdateError {
    UpdateError::Unavailable(error.to_string())
}
impl AgentUpdateRuntimeRouter {
    async fn client(
        &self,
        r: &Registration,
        cancel: &CancellationToken,
    ) -> Result<AgentExecutionClient, UpdateError> {
        if r.kind == AgentUpdateKind::BuildPool {
            let provider = serde_json::from_value(
                r.provider
                    .clone()
                    .ok_or_else(|| error("Build Pool configuration is missing"))?,
            )
            .map_err(error)?;
            return match citadel_builds::pool_target(&provider).map_err(error)? {
                citadel_builds::BuildPoolTarget::Edge => Ok(AgentExecutionClient::Edge(
                    self.edge
                        .get(&EdgeTarget::build_pool(r.id))
                        .map_err(error)?,
                )),
                citadel_builds::BuildPoolTarget::Inbound(endpoint) => {
                    Ok(AgentExecutionClient::Direct(Arc::new(
                        self.agent
                            .as_ref()
                            .ok_or_else(|| error("Signed Agent transport is not configured"))?
                            .for_address(&endpoint, r.id, cancel)
                            .await
                            .map_err(error)?,
                    )))
                }
            };
        }
        use super::super::platforms::runtime::Runtime;
        match self
            .platforms
            .resolve(r.id, None, false, cancel)
            .await
            .map_err(error)?
        {
            Runtime::Agent(client) => Ok(AgentExecutionClient::Direct(Arc::new(client))),
            Runtime::Edge(edge) => Ok(AgentExecutionClient::Edge(edge.session)),
            Runtime::Local(_) => Err(error(
                "Local Docker does not identify an Agent installation",
            )),
        }
    }
    async fn registration(&self, target: &AgentUpdateTarget) -> Result<Registration, UpdateError> {
        agent_inventory::registrations(&self.pool)
            .await?
            .into_iter()
            .find(|r| r.id == target.resource_id && r.kind == target.kind)
            .ok_or(UpdateError::NotFound)
    }
    async fn observe(&self, r: Registration, cancel: &CancellationToken) -> AgentUpdateTarget {
        let fallback = r.previous.clone().unwrap_or_else(|| AgentUpdateTarget {
            id: format!("{:?}:{}", r.kind, r.id),
            resource_id: r.id,
            kind: r.kind,
            name: r.name.clone(),
            installed_version: None,
            connected: false,
            supported: false,
            reason: None,
            nodes: 1,
            drain_nodes: vec![],
            revision: String::new(),
        });
        let observation = async {
            if r.kind == AgentUpdateKind::SwarmService {
                return self.observe_swarm(&r, cancel).await;
            }
            let client = self.client(&r, cancel).await?;
            let info = match client.update_info(cancel).await {
                Ok(info) => info,
                Err(_) => {
                    let version = client.agent_version(cancel).await.map_err(error)?;
                    return Ok(AgentUpdateTarget {connected:true,supported:false,installed_version:Some(version),reason:Some("Manual update required: this Agent does not support central replacement.".into()),..fallback.clone()});
                }
            };
            Ok(AgentUpdateTarget {
                id: info.installation_id,
                resource_id: r.id,
                kind: r.kind,
                name: r.name.clone(),
                installed_version: Some(info.version),
                connected: true,
                supported: info.supported,
                reason: (!info.reason.is_empty()).then_some(info.reason),
                nodes: 1,
                drain_nodes: vec![],
                revision: format!("{}:{}", info.container_id, info.source_image),
            })
        };
        match tokio::time::timeout(Duration::from_secs(5), observation).await {
            Ok(Ok(target)) if !target.id.is_empty() => target,
            _ => AgentUpdateTarget {
                connected: false,
                reason: Some(
                    "Agent is offline or requires a manual update to support central updates."
                        .into(),
                ),
                ..fallback
            },
        }
    }
}
impl AgentUpdateRuntime for AgentUpdateRuntimeRouter {
    fn resume<'a>(
        &'a self,
        target: &'a AgentUpdateTarget,
        operation: Uuid,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(async move {
            if cancel.is_cancelled() {
                return Err(UpdateError::Cancelled);
            }
            if target.kind == AgentUpdateKind::SwarmService {
                self.resume_swarm(target, operation, cancel).await?;
            }
            Ok(())
        })
    }
    fn targets<'a>(
        &'a self,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<AgentUpdateTarget>, UpdateError>> {
        Box::pin(async move {
            let registrations = agent_inventory::registrations(&self.pool).await?;
            let targets = futures_util::stream::iter(registrations)
                .map(|r| self.observe(r, cancel))
                .buffer_unordered(4)
                .collect::<Vec<_>>()
                .await;
            agent_inventory::remember(&self.pool, &targets).await?;
            Ok(targets)
        })
    }
    fn release<'a>(
        &'a self,
        version: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<AgentUpdateRelease, UpdateError>> {
        Box::pin(self.artifacts.agent(version, cancel))
    }
    fn verify<'a>(
        &'a self,
        target: &'a AgentUpdateTarget,
        release: &'a AgentUpdateRelease,
        _operation: Uuid,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<bool, UpdateError>> {
        Box::pin(async move {
            if cancel.is_cancelled() {
                return Err(UpdateError::Cancelled);
            }
            let wait = async {
                loop {
                    if self
                        .verify_once(target, release, cancel)
                        .await
                        .unwrap_or(false)
                    {
                        return Ok(true);
                    }
                    tokio::select! { ()=cancel.cancelled()=>return Err(UpdateError::Cancelled), ()=tokio::time::sleep(Duration::from_secs(2))=>{} }
                }
            };
            tokio::time::timeout(Duration::from_secs(180), wait)
                .await
                .unwrap_or(Ok(false))
        })
    }
    fn install<'a>(
        &'a self,
        target: &'a AgentUpdateTarget,
        release: &'a AgentUpdateRelease,
        operation: Uuid,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(async move {
            if target.kind == AgentUpdateKind::SwarmService {
                self.install_swarm(target, release, operation, cancel)
                    .await?;
            } else {
                let r = self.registration(target).await?;
                let client = self.client(&r, cancel).await?;
                let info = client.update_info(cancel).await.map_err(error)?;
                if info.installation_id != target.id
                    || format!("{}:{}", info.container_id, info.source_image) != target.revision
                    || !info.supported
                {
                    return Err(error("Agent installation changed after review"));
                }
                // Dispatch once. Connection loss can be the source exiting; reconcile, never resend.
                let result = client
                    .update_agent(
                        citadel_contracts::citadel::platforms::v1::UpdateAgentRequest {
                            operation_id: operation.to_string(),
                            installation_id: info.installation_id,
                            source_image: info.source_image,
                            target_image: release.image.clone(),
                        },
                        cancel,
                    )
                    .await;
                if let Err(error) = result {
                    tracing::warn!(%operation,%error,"Agent update dispatch requires reconnect verification");
                }
            }
            if self.verify(target, release, operation, cancel).await? {
                return Ok(());
            }
            Err(error(
                "Agent update could not be verified. Inspect the retained updater helper and previous Agent container before retrying.",
            ))
        })
    }
}

impl AgentUpdateRuntimeRouter {
    async fn verify_once(
        &self,
        target: &AgentUpdateTarget,
        release: &AgentUpdateRelease,
        cancel: &CancellationToken,
    ) -> Result<bool, UpdateError> {
        let r = self.registration(target).await?;
        if target.kind != AgentUpdateKind::SwarmService {
            let info = self
                .client(&r, cancel)
                .await?
                .update_info(cancel)
                .await
                .map_err(error)?;
            return Ok(info.supported
                && info.installation_id == target.id
                && info.version == release.version
                && info.runtime_image == release.image
                && !target
                    .revision
                    .starts_with(&format!("{}:", info.container_id)));
        }
        let observed = self.observe(r, cancel).await;
        let digest = release
            .image
            .split_once('@')
            .map(|(_, digest)| digest)
            .unwrap_or_default();
        Ok(observed.connected
            && observed.id == target.id
            && observed.installed_version.as_deref() == Some(&release.version)
            && observed.revision.ends_with(digest))
    }
}
