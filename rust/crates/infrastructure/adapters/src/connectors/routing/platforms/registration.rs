use citadel_platforms::{
    PlatformConnectorType, PlatformInventoryPort, PlatformRegistrationError,
    PlatformRegistrationRuntime, RuntimeCapabilityError, RuntimeErrorKind,
};

use crate::connectors::agent::client::AgentClient;

use crate::connectors::docker::DockerClient;

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
    fn inventory_for<'a>(
        &'a self,
        connector_type: PlatformConnectorType,
        address: &'a str,
    ) -> Result<&'a dyn PlatformInventoryPort, PlatformRegistrationError> {
        match connector_type {
            PlatformConnectorType::Unknown => Err(PlatformRegistrationError::Runtime(
                RuntimeCapabilityError::new(
                    RuntimeErrorKind::InvalidRequest,
                    "A supported Platform connector type is required.",
                    false,
                ),
            )),
            PlatformConnectorType::Local => Ok(&self.local),
            PlatformConnectorType::Agent => self
                .agent
                .as_ref()
                .filter(|agent| same_address(agent.address(), address))
                .map(|agent| agent as &dyn PlatformInventoryPort)
                .ok_or_else(|| {
                    PlatformRegistrationError::Runtime(RuntimeCapabilityError::new(
                        RuntimeErrorKind::Unavailable,
                        "The requested Agent transport is not configured in the Rust Core.",
                        false,
                    ))
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

fn same_address(left: &str, right: &str) -> bool {
    left.trim_end_matches('/')
        .eq_ignore_ascii_case(right.trim_end_matches('/'))
}
