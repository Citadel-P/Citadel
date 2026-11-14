use super::*;
use citadel_contracts::citadel::platforms::v1::{
    AgentUpdateDrainRequest, AgentUpdateDrainResponse, AgentUpdateInfo, UpdateAgentRequest,
    UpdateAgentResponse,
};
impl AgentExecutionClient {
    pub(crate) async fn settle_update(
        &self,
        request: AgentUpdateDrainRequest,
        cancel: &CancellationToken,
    ) -> Result<AgentUpdateDrainResponse, RuntimeCapabilityError> {
        match self {
            Self::Direct(c) => c.settle_update(request, cancel).await,
            Self::Edge(s) => {
                unary(
                    s,
                    EdgeCommandKind::PlatformAgentUpdateDrain,
                    request,
                    cancel,
                )
                .await
            }
        }
    }

    pub(crate) async fn agent_version(
        &self,
        cancel: &CancellationToken,
    ) -> Result<String, RuntimeCapabilityError> {
        let info = match self {
            Self::Direct(c) => c.platform_info(cancel).await?,
            Self::Edge(s) => {
                unary::<_, citadel_contracts::citadel::shared_models::v1::PlatformInfoResponse>(
                    s,
                    EdgeCommandKind::PlatformGetInfo,
                    (),
                    cancel,
                )
                .await?
            }
        };
        Ok(info.agent_version)
    }
    pub(crate) async fn update_info(
        &self,
        cancel: &CancellationToken,
    ) -> Result<AgentUpdateInfo, RuntimeCapabilityError> {
        match self {
            Self::Direct(c) => c.update_info(cancel).await,
            Self::Edge(s) => unary(s, EdgeCommandKind::PlatformAgentUpdateInfo, (), cancel).await,
        }
    }
    pub(crate) async fn update_agent(
        &self,
        request: UpdateAgentRequest,
        cancel: &CancellationToken,
    ) -> Result<UpdateAgentResponse, RuntimeCapabilityError> {
        match self {
            Self::Direct(c) => tokio::select! {
                biased;
                () = cancel.cancelled() => Err(RuntimeCapabilityError::new(
                    citadel_platforms::RuntimeErrorKind::Cancelled, "Agent update interrupted.", false,
                )),
                result = c.update_agent(request) => result,
            },
            Self::Edge(s) => unary(s, EdgeCommandKind::PlatformUpdateAgent, request, cancel).await,
        }
    }
}
