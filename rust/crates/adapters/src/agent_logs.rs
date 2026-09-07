use super::*;
use citadel_contracts::citadel::containers::v1::ContainerLogRequest;
use citadel_platforms::logs::*;

impl LogReadPort for AgentClient {
    fn read_logs<'a>(
        &'a self,
        resource: LogResource<'a>,
        tail: u16,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<LogSnapshot, RuntimeCapabilityError>> {
        Box::pin(async move {
            crate::container_logs::validate_tail(tail)?;
            let operation = async {
                match resource {
                    LogResource::Service(id) => {
                        let request = self.signer.sign(
                            citadel_contracts::citadel::swarm::v1::SwarmLogsRequest {
                                resource_id: id.into(),
                                tail: tail.into(),
                            },
                            "/citadel.swarm.v1.SwarmService/GetServiceLogs",
                            Some(self.operation_timeout),
                        )?;
                        let response = self
                            .swarm_client()
                            .get_service_logs(request)
                            .await
                            .map_err(normalize_status)?
                            .into_inner();
                        crate::container_logs::checked_snapshot(response.lines, response.truncated)
                    }
                    LogResource::Container(id) => {
                        let request = self.signer.sign(
                            ContainerLogRequest {
                                container_id: id.into(),
                                follow: Some(false),
                                tail: tail.into(),
                            },
                            "/citadel.containers.v1.ContainerService/StreamContainerLogs",
                            Some(self.operation_timeout),
                        )?;
                        let mut stream = self
                            .container_client()
                            .stream_container_logs(request)
                            .await
                            .map_err(normalize_status)?
                            .into_inner();
                        let output = Box::pin(async_stream::try_stream! {
                            while let Some(frame) = stream.next().await {
                                let frame = frame.map_err(normalize_status)?;
                                if frame.log.len()>MAX_LOG_FRAME {Err(crate::container_logs::failure("Agent log frame exceeds the limit."))?;}
                                yield frame.log;
                            }
                        });
                        crate::container_logs::snapshot(output).await
                    }
                }
            };
            tokio::select! {
                biased;
                () = cancel.cancelled() => Err(cancelled_error()),
                result = tokio::time::timeout(self.operation_timeout, operation) => result.map_err(|_|timeout_error("reading logs"))?,
            }
        })
    }
}

impl ContainerLogPort for AgentClient {
    fn container_logs<'a>(
        &'a self,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeLogStream, RuntimeCapabilityError>> {
        Box::pin(async move {
            let request = self.signer.sign(
                ContainerLogRequest {
                    container_id: id.into(),
                    follow: Some(true),
                    tail: 100,
                },
                "/citadel.containers.v1.ContainerService/StreamContainerLogs",
                None,
            )?;
            let mut client = self.container_client();
            let response = tokio::select! {
                biased;
                ()=cancel.cancelled()=>return Err(cancelled_error()),
                result=tokio::time::timeout(self.operation_timeout,client.stream_container_logs(request))=>result.map_err(|_|timeout_error("opening container logs"))?.map_err(normalize_status)?,
            };
            let mut input = response.into_inner();
            let cancel = cancel.clone();
            Ok(Box::pin(async_stream::try_stream! {
                loop {
                    let item=tokio::select!{biased;()=cancel.cancelled()=>break,item=input.next()=>item};
                    let Some(item)=item else {break}; let item=item.map_err(normalize_status)?;
                    if item.log.len()>MAX_LOG_FRAME {Err(crate::container_logs::failure("Agent log frame exceeds the limit."))?;}
                    yield item.log;
                }
            }) as RuntimeLogStream)
        })
    }
}
