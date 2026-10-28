use super::*;
use citadel_contracts::citadel::automation::v1::{
    AutomationAttempt, AutomationCleanup, AutomationEvent, AutomationHost, AutomationRequest,
    automation_service_client::AutomationServiceClient,
};
impl AgentClient {
    fn automation_client(&self) -> AutomationServiceClient<Channel> {
        AutomationServiceClient::new(self.channel.clone())
            .max_decoding_message_size(MAX_GRPC_MESSAGE_BYTES)
            .max_encoding_message_size(MAX_GRPC_MESSAGE_BYTES)
    }
    pub(crate) async fn automation_host(
        &self,
        cancel: &CancellationToken,
    ) -> Result<AutomationHost, RuntimeCapabilityError> {
        let request = self.signer.sign(
            (),
            "/citadel.automation.v1.AutomationService/CheckHost",
            Some(self.operation_timeout),
        )?;
        let mut client = self.automation_client();
        tokio::select! {
            () = cancel.cancelled() => Err(cancelled_error()),
            result = client.check_host(request) => result.map(|v|v.into_inner()).map_err(normalize_status)
        }
    }
    pub(crate) async fn automation_cleanup(
        &self,
        attempt: AutomationAttempt,
        cancel: &CancellationToken,
    ) -> Result<AutomationCleanup, RuntimeCapabilityError> {
        let request = self.signer.sign(
            attempt,
            "/citadel.automation.v1.AutomationService/Reconcile",
            Some(self.operation_timeout),
        )?;
        let mut client = self.automation_client();
        tokio::select! {
            () = cancel.cancelled() => Err(cancelled_error()),
            result = client.reconcile(request) => result.map(|v|v.into_inner()).map_err(normalize_status)
        }
    }
    pub(crate) async fn automation_run(
        &self,
        script: AutomationRequest,
        cancel: &CancellationToken,
    ) -> Result<
        futures_util::stream::BoxStream<'static, Result<AutomationEvent, tonic::Status>>,
        RuntimeCapabilityError,
    > {
        let timeout = Duration::from_secs(u64::from(script.timeout_seconds) + 30);
        let request = self.signer.sign(
            script,
            "/citadel.automation.v1.AutomationService/Run",
            Some(timeout),
        )?;
        // No retry: a lost response does not prove that execution was refused.
        let mut client = self.automation_client();
        tokio::select! {
            () = cancel.cancelled() => Err(cancelled_error()),
            result = client.run(request) => result.map(|v| Box::pin(v.into_inner()) as _).map_err(normalize_status)
        }
    }
}
