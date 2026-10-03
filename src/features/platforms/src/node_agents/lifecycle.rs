//! Node-agent lifecycle orchestration. Runtime and persistence are separate ports;
//! no transport may silently fall back to the Core's Docker daemon.
use crate::RuntimeCapabilityError;
use crate::RuntimeErrorKind;
use crate::RuntimePlatformInfo;
use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct NodeAgentRemovalClaim {
    pub platform_id: Uuid,
    pub platform_name: String,
    pub actor: ActorId,
    pub operation_id: Uuid,
    pub cluster_id: String,
    pub manager_node_id: String,
    pub manager_daemon_id: String,
    pub service_id: Option<String>,
    pub ca_config_id: Option<String>,
    pub secret_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum NodeAgentResource {
    Service,
    Secret,
    Config,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeAgentProgress {
    pub platform_id: Uuid,
    pub operation_id: Uuid,
    pub stage: &'static str,
    pub message: String,
    pub is_completed: bool,
    pub is_warning: bool,
    pub error_message: Option<String>,
}

pub trait NodeAgentLifecycleStore: Send + Sync {
    fn claim_remove(
        &self,
        actor: ActorId,
        platform_id: Uuid,
        info: RuntimePlatformInfo,
    ) -> BoxFuture<'_, Result<NodeAgentRemovalClaim, RuntimeCapabilityError>>;
    fn revoke<'a>(
        &'a self,
        claim: &'a NodeAgentRemovalClaim,
    ) -> BoxFuture<'a, Result<Vec<String>, RuntimeCapabilityError>>;
    fn finish<'a>(
        &'a self,
        claim: &'a NodeAgentRemovalClaim,
        error: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>>;
}

pub trait NodeAgentLifecycleRuntime: Send + Sync {
    fn info<'a>(
        &'a self,
        platform_id: Uuid,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimePlatformInfo, RuntimeCapabilityError>>;
    /// Inspect the exact resource ID and verify all ownership labels before deletion.
    fn delete_owned<'a>(
        &'a self,
        claim: &'a NodeAgentRemovalClaim,
        kind: NodeAgentResource,
        id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>>;
    fn disconnect(&self, platform_id: Uuid, node_ids: &[String]);
}

pub struct NodeAgentRemovalService {
    pub store: Arc<dyn NodeAgentLifecycleStore>,
    pub runtime: Arc<dyn NodeAgentLifecycleRuntime>,
    pub changed: Arc<dyn Fn(Uuid) + Send + Sync>,
}

impl NodeAgentRemovalService {
    pub async fn remove(
        &self,
        actor: ActorId,
        platform_id: Uuid,
        sender: mpsc::Sender<NodeAgentProgress>,
        cancellation: CancellationToken,
    ) {
        let start = async {
            let info = self.runtime.info(platform_id, &cancellation).await?;
            self.store.claim_remove(actor, platform_id, info).await
        };
        let claim = tokio::select! {
            () = cancellation.cancelled() => return,
            result = tokio::time::timeout(Duration::from_secs(30), start) => match result {
                Ok(Ok(claim)) => claim,
                result => {
                    let error = match result {
                        Err(_) => "Node-agent validation timed out.".to_owned(),
                        Ok(Err(error)) => error.message,
                        Ok(Ok(_)) => unreachable!(),
                    };
                    let _ = sender.try_send(progress(platform_id, Uuid::nil(), "failed", &error, true, false, Some(error.clone())));
                    return;
                }
            }
        };
        (self.changed)(platform_id);
        let _ = sender.try_send(progress(
            platform_id,
            claim.operation_id,
            "validation",
            "Validated the pinned Swarm manager and claimed node-agent removal.",
            false,
            false,
            None,
        ));
        let operation = self.execute(&claim, &sender, &cancellation);
        let result = tokio::select! {
            () = cancellation.cancelled() => Err(failure("Node-agent removal was canceled. Retry removal to reconcile partial state.")),
            result = tokio::time::timeout(Duration::from_secs(120), operation) => result.unwrap_or_else(|_| Err(failure("Node-agent removal timed out. Retry removal to reconcile partial state."))),
        };
        let error = result.err().map(|error| error.message);
        // A disconnected caller must not prevent durable failure/recovery state.
        let completed = tokio::time::timeout(
            Duration::from_secs(15),
            self.store.finish(&claim, error.as_deref()),
        )
        .await;
        let error = match completed {
            Ok(Ok(())) => error,
            _ => Some("Could not persist node-agent completion. The operation remains recoverable after its lease expires.".into()),
        };
        (self.changed)(platform_id);
        let message = error.as_deref().unwrap_or(
            "Removed node agents and revoked their credentials. State volumes were preserved.",
        );
        let _ = tokio::time::timeout(
            Duration::from_secs(5),
            sender.send(progress(
                platform_id,
                claim.operation_id,
                if error.is_some() {
                    "failed"
                } else {
                    "completed"
                },
                message,
                true,
                false,
                error.clone(),
            )),
        )
        .await;
    }

    async fn execute(
        &self,
        claim: &NodeAgentRemovalClaim,
        sender: &mpsc::Sender<NodeAgentProgress>,
        cancellation: &CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        if let Some(id) = &claim.service_id {
            self.runtime
                .delete_owned(claim, NodeAgentResource::Service, id, cancellation)
                .await?;
        }
        let nodes = self.store.revoke(claim).await?;
        self.runtime.disconnect(claim.platform_id, &nodes);
        for (kind, id) in claim
            .secret_ids
            .iter()
            .map(|id| (NodeAgentResource::Secret, id))
            .chain(
                claim
                    .ca_config_id
                    .iter()
                    .map(|id| (NodeAgentResource::Config, id)),
            )
        {
            if cancellation.is_cancelled() {
                return Err(failure(
                    "Node-agent removal was canceled. Retry removal to reconcile partial state.",
                ));
            }
            if let Err(error) = self
                .runtime
                .delete_owned(claim, kind, id, cancellation)
                .await
            {
                let _ = sender.try_send(progress(
                    claim.platform_id,
                    claim.operation_id,
                    "cleanup",
                    &format!("Preserved {kind:?} '{id}': {}", error.message),
                    false,
                    true,
                    None,
                ));
            }
        }
        if cancellation.is_cancelled() {
            return Err(failure(
                "Node-agent removal was canceled. Retry removal to reconcile partial state.",
            ));
        }
        Ok(())
    }
}

fn progress(
    platform_id: Uuid,
    operation_id: Uuid,
    stage: &'static str,
    message: &str,
    is_completed: bool,
    is_warning: bool,
    error_message: Option<String>,
) -> NodeAgentProgress {
    NodeAgentProgress {
        platform_id,
        operation_id,
        stage,
        message: message.into(),
        is_completed,
        is_warning,
        error_message,
    }
}
fn failure(message: &str) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Conflict, message, false)
}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;
