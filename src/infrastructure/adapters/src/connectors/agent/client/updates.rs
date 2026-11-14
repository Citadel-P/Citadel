use super::*;
use citadel_contracts::citadel::platforms::v1::{
    AgentUpdateDrainRequest, AgentUpdateDrainResponse, AgentUpdateInfo, UpdateAgentRequest,
    UpdateAgentResponse,
};
impl AgentClient {
    pub(crate) async fn settle_update(
        &self,
        request: AgentUpdateDrainRequest,
        cancel: &CancellationToken,
    ) -> Result<AgentUpdateDrainResponse, RuntimeCapabilityError> {
        self.retry_unary(cancel, || async {
            let request = self.signer.sign(
                request.clone(),
                "/citadel.platforms.v1.PlatformService/SetAgentUpdateDrain",
                Some(self.operation_timeout),
            )?;
            self.platform_client()
                .set_agent_update_drain(request)
                .await
                .map(|response| response.into_inner())
                .map_err(normalize_status)
        })
        .await
    }

    pub(crate) async fn update_info(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<AgentUpdateInfo, RuntimeCapabilityError> {
        self.retry_unary(cancellation, || async {
            let request = self.signer.sign(
                (),
                "/citadel.platforms.v1.PlatformService/GetAgentUpdateInfo",
                Some(self.operation_timeout),
            )?;
            self.platform_client()
                .get_agent_update_info(request)
                .await
                .map(|r| r.into_inner())
                .map_err(normalize_status)
        })
        .await
    }
    pub(crate) async fn update_agent(
        &self,
        request: UpdateAgentRequest,
    ) -> Result<UpdateAgentResponse, RuntimeCapabilityError> {
        // This mutates the Agent itself: never retry an ambiguous dispatch.
        let request = self.signer.sign(
            request,
            "/citadel.platforms.v1.PlatformService/UpdateAgent",
            Some(Duration::from_secs(450)),
        )?;
        self.platform_client()
            .update_agent(request)
            .await
            .map(|r| r.into_inner())
            .map_err(normalize_status)
    }
}
