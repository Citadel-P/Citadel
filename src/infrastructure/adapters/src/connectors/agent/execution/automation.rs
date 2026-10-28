use super::*;
use citadel_contracts::citadel::automation::v1::{
    AutomationAttempt, AutomationCleanup, AutomationEvent, AutomationHost, AutomationRequest,
};
impl AgentExecutionClient {
    pub(crate) async fn automation_host(
        &self,
        cancel: &CancellationToken,
    ) -> Result<AutomationHost, RuntimeCapabilityError> {
        match self {
            Self::Direct(client) => client.automation_host(cancel).await,
            Self::Edge(session) => {
                unary(session, EdgeCommandKind::AutomationCheckHost, (), cancel).await
            }
        }
    }
    pub(crate) async fn automation_cleanup(
        &self,
        attempt: AutomationAttempt,
        cancel: &CancellationToken,
    ) -> Result<AutomationCleanup, RuntimeCapabilityError> {
        match self {
            Self::Direct(client) => client.automation_cleanup(attempt, cancel).await,
            Self::Edge(session) => {
                unary(
                    session,
                    EdgeCommandKind::AutomationReconcile,
                    attempt,
                    cancel,
                )
                .await
            }
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
        match self {
            Self::Direct(client) => client.automation_run(script, cancel).await,
            Self::Edge(session) => {
                let mut pending = session
                    .command(
                        EdgeCommandKind::AutomationRun,
                        script.encode_to_vec(),
                        Duration::from_secs(u64::from(script.timeout_seconds) + 30),
                        true,
                    )
                    .map_err(edge_error)?;
                let cancel = cancel.clone();
                Ok(Box::pin(async_stream::stream! {
                    loop { match pending.next(&cancel).await {
                        Ok(Some(bytes)) => yield AutomationEvent::decode(bytes.as_slice()).map_err(|_| tonic::Status::data_loss("Invalid automation response")),
                        Ok(None) => break,
                        Err(error) => { yield Err(error.status()); break; }
                    }}
                }))
            }
        }
    }
}
