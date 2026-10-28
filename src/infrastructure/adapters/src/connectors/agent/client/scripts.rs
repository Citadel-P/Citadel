use super::*;
use citadel_contracts::citadel::scripts::v1::{
    ScriptAttempt, ScriptCleanup, ScriptEvent, ScriptHost, ScriptRequest,
    script_service_client::ScriptServiceClient,
};
impl AgentClient {
    fn script_client(&self) -> ScriptServiceClient<Channel> {
        ScriptServiceClient::new(self.channel.clone())
            .max_decoding_message_size(MAX_GRPC_MESSAGE_BYTES)
            .max_encoding_message_size(MAX_GRPC_MESSAGE_BYTES)
    }
    pub(crate) async fn script_host(
        &self,
        cancel: &CancellationToken,
    ) -> Result<ScriptHost, RuntimeCapabilityError> {
        let request = self.signer.sign(
            (),
            "/citadel.scripts.v1.ScriptService/CheckHost",
            Some(self.operation_timeout),
        )?;
        let mut client = self.script_client();
        tokio::select! {
            () = cancel.cancelled() => Err(cancelled_error()),
            result = client.check_host(request) => result.map(|v|v.into_inner()).map_err(normalize_status)
        }
    }
    pub(crate) async fn script_cleanup(
        &self,
        attempt: ScriptAttempt,
        cancel: &CancellationToken,
    ) -> Result<ScriptCleanup, RuntimeCapabilityError> {
        let request = self.signer.sign(
            attempt,
            "/citadel.scripts.v1.ScriptService/Reconcile",
            Some(self.operation_timeout),
        )?;
        let mut client = self.script_client();
        tokio::select! {
            () = cancel.cancelled() => Err(cancelled_error()),
            result = client.reconcile(request) => result.map(|v|v.into_inner()).map_err(normalize_status)
        }
    }
    pub(crate) async fn script_run(
        &self,
        script: ScriptRequest,
        cancel: &CancellationToken,
    ) -> Result<
        futures_util::stream::BoxStream<'static, Result<ScriptEvent, tonic::Status>>,
        RuntimeCapabilityError,
    > {
        let timeout = Duration::from_secs(u64::from(script.timeout_seconds) + 30);
        let request = self.signer.sign(
            script,
            "/citadel.scripts.v1.ScriptService/Run",
            Some(timeout),
        )?;
        // No retry: a lost response does not prove that execution was refused.
        let mut client = self.script_client();
        tokio::select! {
            () = cancel.cancelled() => Err(cancelled_error()),
            result = client.run(request) => result.map(|v| Box::pin(v.into_inner()) as _).map_err(normalize_status)
        }
    }
}
