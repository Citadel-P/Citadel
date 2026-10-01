//! Candidate connection validation intentionally accepts an uncommitted address.
use crate::connectors::{
    agent::client::AgentClient,
    docker::DockerClient,
    edge::{EdgeRegistry, EdgeRuntime, EdgeTarget},
};
use citadel_platforms::{
    CreatePlatformInput, PlatformConnectorType, PlatformInfoPort, RuntimeCapabilityError,
    RuntimeErrorKind, RuntimePlatformInfo, management::PlatformManagementRuntime,
};
use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
#[derive(Clone)]
pub struct PlatformManagementRuntimeAdapter {
    pub local: DockerClient,
    pub agent: Option<AgentClient>,
    pub edge: EdgeRegistry,
}
impl PlatformManagementRuntime for PlatformManagementRuntimeAdapter {
    fn inspect_candidate<'a>(
        &'a self,
        id: Uuid,
        input: &'a CreatePlatformInput,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimePlatformInfo, RuntimeCapabilityError>> {
        Box::pin(async move {
            match input.connector_type {
                PlatformConnectorType::Local => self.local.get_info(cancel).await,
                PlatformConnectorType::Agent => {
                    let agent = self.agent.as_ref().ok_or_else(|| {
                        RuntimeCapabilityError::new(
                            RuntimeErrorKind::Unavailable,
                            "Agent transport is not configured.",
                            false,
                        )
                    })?;
                    agent
                        .for_address(input.address.as_deref().unwrap_or_default(), cancel)
                        .await?
                        .get_info(cancel)
                        .await
                }
                PlatformConnectorType::EdgeAgent => {
                    let session = self.edge.get(&EdgeTarget::platform(id)).map_err(|_| {
                        RuntimeCapabilityError::new(
                            RuntimeErrorKind::Unavailable,
                            "Edge Agent is disconnected.",
                            false,
                        )
                    })?;
                    EdgeRuntime { session }.get_info(cancel).await
                }
                _ => Err(RuntimeCapabilityError::new(
                    RuntimeErrorKind::InvalidRequest,
                    "A supported connector is required.",
                    false,
                )),
            }
        })
    }
}

impl citadel_platforms::management::AgentSigningPort
    for crate::connectors::agent::client::AgentRequestSigner
{
    fn public_key_base64(&self) -> String {
        self.public_key_base64()
    }
    fn rotate(&self) -> BoxFuture<'_, Result<(), RuntimeCapabilityError>> {
        let signer = self.clone();
        Box::pin(async move {
            tokio::task::spawn_blocking(move || signer.rotate())
                .await
                .map_err(|_| {
                    RuntimeCapabilityError::new(
                        RuntimeErrorKind::Unavailable,
                        "Agent key rotation failed.",
                        false,
                    )
                })?
                .map_err(|_| {
                    RuntimeCapabilityError::new(
                        RuntimeErrorKind::Unavailable,
                        "Agent key rotation could not be persisted.",
                        false,
                    )
                })?;
            Ok(())
        })
    }
}
