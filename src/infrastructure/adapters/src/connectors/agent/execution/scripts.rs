use super::*;
use citadel_contracts::citadel::scripts::v1::{
    ScriptAttempt, ScriptCleanup, ScriptEvent, ScriptHost, ScriptRequest,
};
impl AgentExecutionClient {
    pub(crate) async fn script_host(
        &self,
        cancel: &CancellationToken,
    ) -> Result<ScriptHost, RuntimeCapabilityError> {
        match self {
            Self::Direct(client) => client.script_host(cancel).await,
            Self::Edge(session) => {
                unary(session, EdgeCommandKind::ScriptCheckHost, (), cancel).await
            }
        }
    }
    pub(crate) async fn script_cleanup(
        &self,
        attempt: ScriptAttempt,
        cancel: &CancellationToken,
    ) -> Result<ScriptCleanup, RuntimeCapabilityError> {
        match self {
            Self::Direct(client) => client.script_cleanup(attempt, cancel).await,
            Self::Edge(session) => {
                unary(session, EdgeCommandKind::ScriptReconcile, attempt, cancel).await
            }
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
        match self {
            Self::Direct(client) => client.script_run(script, cancel).await,
            Self::Edge(session) => {
                let mut pending = session
                    .command(
                        EdgeCommandKind::ScriptRun,
                        script.encode_to_vec(),
                        Duration::from_secs(u64::from(script.timeout_seconds) + 30),
                        true,
                    )
                    .map_err(edge_error)?;
                let cancel = cancel.clone();
                Ok(Box::pin(async_stream::stream! {
                    loop { match pending.next(&cancel).await {
                        Ok(Some(bytes)) => yield ScriptEvent::decode(bytes.as_slice()).map_err(|_| tonic::Status::data_loss("Invalid scripts response")),
                        Ok(None) => break,
                        Err(error) => { yield Err(error.status()); break; }
                    }}
                }))
            }
        }
    }
}
