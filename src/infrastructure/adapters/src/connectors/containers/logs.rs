use citadel_contracts::citadel::{
    containers::v1::{ContainerLogRequest, ContainerLogResponse},
    edge::v1::EdgeCommandKind,
};
use citadel_docker::containers::logs::*;
use citadel_platforms::{RuntimeCapabilityError, logs::*};
use futures_util::future::BoxFuture;
use prost::Message;
use tokio_util::sync::CancellationToken;

impl ContainerLogPort for crate::connectors::edge::EdgeRuntime {
    fn container_logs<'a>(
        &'a self,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeLogStream, RuntimeCapabilityError>> {
        Box::pin(async move {
            if cancel.is_cancelled() {
                return Err(cancelled());
            }
            let mut command = self
                .session
                .command(
                    EdgeCommandKind::ContainerLogsStream,
                    ContainerLogRequest {
                        container_id: id.into(),
                        follow: Some(true),
                        tail: 100,
                    }
                    .encode_to_vec(),
                    std::time::Duration::from_secs(24 * 60 * 60),
                    true,
                )
                .map_err(crate::connectors::edge::EdgeError::runtime)?;
            let cancel = cancel.clone();
            Ok(Box::pin(async_stream::try_stream! {
                loop {
                    let next=tokio::select! {biased;()=cancel.cancelled()=>break,next=command.next(&cancel)=>next};
                    let Some(bytes)=next.map_err(crate::connectors::edge::EdgeError::runtime)? else {break};
                    let frame=ContainerLogResponse::decode(bytes.as_slice()).map_err(|_|failure("Invalid Agent log frame."))?;
                    if frame.log.len()>MAX_LOG_FRAME {Err(failure("Agent log frame exceeds the limit."))?;}
                    yield frame.log;
                }
            }) as RuntimeLogStream)
        })
    }
}

impl LogReadPort for crate::connectors::edge::EdgeRuntime {
    fn read_logs<'a>(
        &'a self,
        resource: LogResource<'a>,
        tail: u16,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<LogSnapshot, RuntimeCapabilityError>> {
        Box::pin(async move {
            validate_tail(tail)?;
            match resource {
                LogResource::Service(id) => {
                    let response: citadel_contracts::citadel::swarm::v1::SwarmLogsResponse =
                        crate::connectors::agent::execution::unary(
                            &self.session,
                            EdgeCommandKind::SwarmServiceLogs,
                            citadel_contracts::citadel::swarm::v1::SwarmLogsRequest {
                                resource_id: id.into(),
                                tail: tail.into(),
                            },
                            cancel,
                        )
                        .await?;
                    checked_snapshot(response.lines, response.truncated)
                }
                LogResource::Container(id) => {
                    let mut command = self
                        .session
                        .command(
                            EdgeCommandKind::ContainerLogsStream,
                            ContainerLogRequest {
                                container_id: id.into(),
                                follow: Some(false),
                                tail: tail.into(),
                            }
                            .encode_to_vec(),
                            std::time::Duration::from_secs(30),
                            true,
                        )
                        .map_err(crate::connectors::edge::EdgeError::runtime)?;
                    let cancel = cancel.clone();
                    let stream = Box::pin(async_stream::try_stream! {
                        while let Some(bytes) = command.next(&cancel).await.map_err(crate::connectors::edge::EdgeError::runtime)? {
                            let response = ContainerLogResponse::decode(bytes.as_slice()).map_err(|_|failure("Invalid Agent log frame."))?;
                            if response.log.len()>MAX_LOG_FRAME {Err(failure("Agent log frame exceeds the limit."))?;}
                            yield response.log;
                        }
                    });
                    snapshot(stream).await
                }
            }
        })
    }
}
