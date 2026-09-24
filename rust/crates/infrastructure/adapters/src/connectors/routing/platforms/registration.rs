use citadel_platforms::{
    PlatformConnectorType, PlatformInventoryPort, PlatformRegistrationError,
    PlatformRegistrationRuntime, RuntimeCapabilityError, RuntimeErrorKind,
};

use crate::connectors::agent::client::AgentClient;

use crate::connectors::docker::DockerClient;
use std::sync::Arc;

#[derive(Clone)]
pub struct PlatformRegistrationRuntimeRouter {
    local: DockerClient,
    agent: Option<AgentClient>,
}

impl PlatformRegistrationRuntimeRouter {
    #[must_use]
    pub fn new(local: DockerClient, agent: Option<AgentClient>) -> Self {
        Self { local, agent }
    }
}

impl PlatformRegistrationRuntime for PlatformRegistrationRuntimeRouter {
    fn inventory_for(
        &self,
        connector_type: PlatformConnectorType,
        address: &str,
    ) -> Result<Arc<dyn PlatformInventoryPort>, PlatformRegistrationError> {
        match connector_type {
            PlatformConnectorType::Unknown => Err(PlatformRegistrationError::Runtime(
                RuntimeCapabilityError::new(
                    RuntimeErrorKind::InvalidRequest,
                    "A supported Platform connector type is required.",
                    false,
                ),
            )),
            PlatformConnectorType::Local => Ok(Arc::new(self.local.clone())),
            PlatformConnectorType::Agent => self
                .agent
                .as_ref()
                .ok_or_else(|| {
                    PlatformRegistrationError::Runtime(RuntimeCapabilityError::new(
                        RuntimeErrorKind::Unavailable,
                        "The requested Agent transport is not configured in the Rust Core.",
                        false,
                    ))
                })
                .and_then(|agent| {
                    agent
                        .at_address(address)
                        .map(|agent| Arc::new(agent) as Arc<dyn PlatformInventoryPort>)
                        .map_err(PlatformRegistrationError::from)
                }),
            PlatformConnectorType::EdgeAgent => Err(PlatformRegistrationError::Runtime(
                RuntimeCapabilityError::new(
                    RuntimeErrorKind::Unavailable,
                    "The Edge Agent is disconnected or unavailable.",
                    true,
                ),
            )),
        }
    }
}
