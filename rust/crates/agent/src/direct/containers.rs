use super::{Runtime, container_mapping as mapping, docker_error, runtime_error};
use citadel_adapters::connectors::docker::{DockerExecError, DockerExecEvent, container_summary};
use citadel_contracts::citadel::{
    containers::v1::*,
    shared_models::v1::{ContainerMessage, ContainerStatMessage, InspectContainerResponse},
};
use citadel_platforms::{
    containers::ContainerAction,
    terminal::{TerminalInput, TerminalOutput},
};
use futures_util::{StreamExt, stream::BoxStream};
use tonic::{Request, Response, Status};

impl Runtime {
    async fn change(
        &self,
        ids: ContainerIds,
        action: ContainerAction,
    ) -> Result<Response<()>, Status> {
        if ids.ids.iter().any(|id| id.trim().is_empty()) {
            return Err(Status::invalid_argument("Container ids must not be empty"));
        }
        let count = ids.ids.len();
        let results = futures_util::stream::iter(ids.ids.into_iter().map(|id| {
            let docker = self.docker.clone();
            async move {
                let _permit = tokio::select! {
                    () = self.shutdown.cancelled() => return Err((id, Status::cancelled("Agent shutting down"))),
                    permit = self.mutations.acquire() => permit.map_err(|_| (id.clone(), Status::cancelled("Agent shutting down")))?,
                };
                docker
                    .change_container_state(&id, action)
                    .await
                    .map_err(|e| (id, docker_error(e)))
            }
        }))
        .buffer_unordered(8)
        .collect::<Vec<_>>()
        .await;
        self.samples.invalidate();
        let failures: Vec<_> = results.into_iter().filter_map(Result::err).collect();
        if !failures.is_empty() {
            let details = failures
                .iter()
                .take(8)
                .map(|(id, e)| format!("{}: {}", id, e.message()))
                .collect::<Vec<_>>()
                .join(" | ");
            return Err(Status::failed_precondition(format!(
                "Failed ({}/{}): {details}",
                failures.len(),
                count
            )));
        }
        Ok(Response::new(()))
    }
}

#[tonic::async_trait]
impl container_service_server::ContainerService for Runtime {
    async fn list(
        &self,
        request: Request<ListContainersRequest>,
    ) -> Result<Response<ListContainersResponse>, Status> {
        let r = request.into_inner();
        let metadata_only = r.metadata_only;
        let filter = serde_json::to_string(
            &r.filters
                .into_iter()
                .map(|(k, v)| (k, v.options))
                .collect::<std::collections::HashMap<_, _>>(),
        )
        .expect("string filters");
        let models = self
            .docker
            .list_container_models(r.all, r.limit, r.size, Some(&filter))
            .await
            .map_err(docker_error)?;
        let cached = if metadata_only {
            Default::default()
        } else {
            self.samples.cached_stats().await
        };
        let values = futures_util::stream::iter(models.into_iter().map(|model| async {
            let mut c = summary(model)?;
            if metadata_only {
                c.container_stat_message = None;
            }
            if c.state == 2 && !metadata_only {
                c.container_stat_message = Some(match cached.get(&c.id).cloned() {
                    Some(value) => stat(value),
                    None => self
                        .docker
                        .sample_container_stats(&c.id, &self.shutdown)
                        .await
                        .map(stat)
                        .unwrap_or_default(),
                });
            }
            Ok::<_, Status>((c.id.clone(), c))
        }))
        .buffer_unordered(8)
        .collect::<Vec<_>>()
        .await;
        let containers = values.into_iter().collect::<Result<_, _>>()?;
        Ok(Response::new(ListContainersResponse { containers }))
    }
    async fn start(&self, r: Request<ContainerIds>) -> Result<Response<()>, Status> {
        self.change(r.into_inner(), ContainerAction::Start).await
    }
    async fn stop(&self, r: Request<ContainerIds>) -> Result<Response<()>, Status> {
        self.change(r.into_inner(), ContainerAction::Stop).await
    }
    async fn pause(&self, r: Request<ContainerIds>) -> Result<Response<()>, Status> {
        self.change(r.into_inner(), ContainerAction::Pause).await
    }
    async fn unpause(&self, r: Request<ContainerIds>) -> Result<Response<()>, Status> {
        self.change(r.into_inner(), ContainerAction::Unpause).await
    }
    async fn restart(&self, r: Request<ContainerIds>) -> Result<Response<()>, Status> {
        self.change(r.into_inner(), ContainerAction::Restart).await
    }
    async fn delete(&self, r: Request<DeleteContainerRequest>) -> Result<Response<()>, Status> {
        let r = r.into_inner();
        if r.ids.iter().any(|id| id.trim().is_empty()) {
            return Err(Status::invalid_argument("Container ids must not be empty"));
        }
        self.samples.invalidate();
        for id in r.ids {
            let result = self
                .docker
                .delete_container_with_options(
                    &id,
                    r.v.unwrap_or(false),
                    r.force.unwrap_or(false),
                    r.link.unwrap_or(false),
                )
                .await;
            self.samples.invalidate();
            result.map_err(docker_error)?;
        }
        Ok(Response::new(()))
    }
    async fn inspect(
        &self,
        r: Request<InspectContainerRequest>,
    ) -> Result<Response<InspectContainerResponse>, Status> {
        Ok(Response::new(mapping::inspect_container_response(
            &self
                .docker
                .inspect_container_document(&r.into_inner().container_id)
                .await
                .map_err(docker_error)?,
        )))
    }
    async fn create(
        &self,
        r: Request<CreateContainerRequest>,
    ) -> Result<Response<CreateContainerResponse>, Status> {
        let r = r.into_inner();
        let body = super::container_config::create(&r)?;
        Ok(Response::new(CreateContainerResponse {
            container_id: self
                .docker
                .create_container(&r.name, &body)
                .await
                .map_err(docker_error)?,
        }))
    }

    type ExecBinaryStream = BoxStream<'static, Result<ExecServerMessage, Status>>;
    async fn exec_binary(
        &self,
        r: Request<ExecBinaryRequest>,
    ) -> Result<Response<Self::ExecBinaryStream>, Status> {
        let r = r.into_inner();
        if r.container_id.trim().is_empty() || r.cmd.is_empty() {
            return Err(Status::invalid_argument(
                "Container ID and command are required",
            ));
        }
        let cancel = self.shutdown.child_token();
        let guard = cancel.clone().drop_guard();
        let mut output = self
            .docker
            .execute_binary(
                &r.container_id,
                citadel_docker_api::models::ExecConfig {
                    cmd: Some(r.cmd),
                    env: Some(
                        r.env
                            .into_iter()
                            .map(|(key, value)| format!("{key}={value}"))
                            .collect(),
                    ),
                    attach_stdout: Some(r.attach_stdout.unwrap_or(true)),
                    attach_stderr: Some(r.attach_stderr.unwrap_or(true)),
                    tty: Some(r.tty),
                    ..Default::default()
                },
                &cancel,
            )
            .await
            .map_err(runtime_error)?;
        Ok(Response::new(Box::pin(async_stream::try_stream! {
            let _guard=guard;
            while let Some(event)=output.next().await {
                let event=event.map_err(|e|match e {
                    DockerExecError::Cancelled=>Status::cancelled(e.to_string()),DockerExecError::InvalidFrame(_)=>Status::data_loss(e.to_string()),
                    DockerExecError::FrameTooLarge=>Status::resource_exhausted(e.to_string()),DockerExecError::Transport(_)=>Status::unavailable(e.to_string()),
                })?;
                yield ExecServerMessage {msg:Some(match event {
                    DockerExecEvent::Output{data,stderr}=>exec_server_message::Msg::Output(ExecOutput {data,stream:i32::from(stderr)}),
                    DockerExecEvent::Exit(exit_code)=>exec_server_message::Msg::Exit(ExecExit{exit_code}),
                })};
            }
        })))
    }

    type ExecStream = BoxStream<'static, Result<ExecServerMessage, Status>>;
    async fn exec(
        &self,
        r: Request<tonic::Streaming<ExecClientMessage>>,
    ) -> Result<Response<Self::ExecStream>, Status> {
        self.exec_messages(Box::pin(r.into_inner())).await
    }

    type StreamContainerLogsStream = BoxStream<'static, Result<ContainerLogResponse, Status>>;
    async fn stream_container_logs(
        &self,
        r: Request<ContainerLogRequest>,
    ) -> Result<Response<Self::StreamContainerLogsStream>, Status> {
        let r = r.into_inner();
        let cancel = self.shutdown.child_token();
        let guard = cancel.clone().drop_guard();
        let tail = if r.tail <= 0 {
            100
        } else {
            r.tail.min(u16::MAX as i32) as u16
        };
        let mut logs = self
            .docker
            .open_resource_logs(
                citadel_platforms::logs::LogResource::Container(&r.container_id),
                r.follow.unwrap_or(true),
                tail,
                &cancel,
            )
            .await
            .map_err(docker_error)?;
        Ok(Response::new(Box::pin(async_stream::try_stream! {
            let _guard=guard;
            while let Some(log)=logs.next().await {yield ContainerLogResponse {log:log.map_err(runtime_error)?};}
        })))
    }
    type StreamContainersStatsStream = BoxStream<'static, Result<ContainersStatsResponse, Status>>;
    async fn stream_containers_stats(
        &self,
        r: Request<StreamContainersStatsRequest>,
    ) -> Result<Response<Self::StreamContainersStatsStream>, Status> {
        let interval = super::interval(r.into_inner().fetch_interval_ms);
        let sampler = self.samples.clone();
        let cancel = self.shutdown.child_token();
        let guard = cancel.clone().drop_guard();
        Ok(Response::new(Box::pin(async_stream::try_stream! {
            let _guard=guard;
            loop {
                let sample=tokio::select! { ()=cancel.cancelled()=>break, value=sampler.sample(interval, &cancel)=>value.map_err(runtime_error) }?;
                yield ContainersStatsResponse { captured_at: Some(sample.captured_at), containers:sample.containers.stats.into_iter().map(|s|(s.docker_container_id.clone(),stat(s))).collect()};
                tokio::select! { ()=cancel.cancelled()=>break,()=tokio::time::sleep(interval)=>{} }
            }
        })))
    }
    type StreamContainerStatsStream = BoxStream<'static, Result<ContainerMessage, Status>>;
    async fn stream_container_stats(
        &self,
        r: Request<StreamContainerStatsRequest>,
    ) -> Result<Response<Self::StreamContainerStatsStream>, Status> {
        let r = r.into_inner();
        let interval = super::interval(r.fetch_interval_ms);
        let docker = self.docker.clone();
        let cancel = self.shutdown.child_token();
        let guard = cancel.clone().drop_guard();
        Ok(Response::new(Box::pin(async_stream::try_stream! {
            let _guard=guard;
            let filter=serde_json::json!({"id":[r.container_id]}).to_string();
            loop {
                let mut models=docker.list_container_models(Some(true),None,None,Some(&filter)).await.map_err(docker_error)?;
                let model=models.pop().ok_or_else(||Status::not_found("Container not found"))?;
                let mut c=summary(model)?;
                c.container_stat_message=Some(if c.state==2 {docker.sample_container_stats(&c.id,&cancel).await.map(stat).unwrap_or_default()}else{ContainerStatMessage::default()});
                yield c;
                tokio::select! { ()=cancel.cancelled()=>break,()=tokio::time::sleep(interval)=>{} }
            }
        })))
    }
}

impl Runtime {
    /// Shared duplex boundary for Direct gRPC input and Edge StreamInput messages.
    pub(crate) async fn exec_messages(
        &self,
        mut incoming: BoxStream<'static, Result<ExecClientMessage, Status>>,
    ) -> Result<Response<BoxStream<'static, Result<ExecServerMessage, Status>>>, Status> {
        let opening = incoming
            .next()
            .await
            .transpose()?
            .ok_or_else(|| Status::invalid_argument("Missing Open message"))?;
        let Some(exec_client_message::Msg::Open(open)) = opening.msg else {
            return Err(Status::invalid_argument("The first message must be Open"));
        };
        let cancel = self.shutdown.child_token();
        let guard = cancel.clone().drop_guard();
        if open.container_id.trim().is_empty() || open.cmd.is_empty() {
            return Err(Status::invalid_argument(
                "Container ID and command are required",
            ));
        }
        let session = self
            .docker
            .execute_terminal(&open.container_id, &open.cmd, &cancel)
            .await
            .map_err(docker_error)?;
        let mut input = Some(session.input);
        let mut output = session.output;
        Ok(Response::new(Box::pin(async_stream::try_stream! {
            let _guard=guard;
            loop {
                let event=tokio::select! {
                    ()=cancel.cancelled()=>break,
                    value=incoming.next(), if input.is_some()=>Ok(value),
                    value=output.next()=>Err(value),
                };
                match event {
                    Ok(value)=>match value.transpose()? {
                        None=>{input.take();},
                        Some(message)=>{
                            let message=match message.msg {
                                Some(exec_client_message::Msg::Stdin(value))=>TerminalInput::Stdin(value.data),
                                Some(exec_client_message::Msg::Resize(value))=>TerminalInput::Resize {
                                    cols:value.cols.try_into().map_err(|_|Status::invalid_argument("Invalid terminal columns"))?,
                                    rows:value.rows.try_into().map_err(|_|Status::invalid_argument("Invalid terminal rows"))?,
                                },
                                _=>Err(Status::invalid_argument("Expected terminal input or resize"))?,
                            };
                            input.as_ref().expect("input is open").try_send(message).map_err(runtime_error)?;
                        }
                    },
                    Err(None)=>break,
                    Err(Some(value))=>yield ExecServerMessage {msg:Some(match value.map_err(runtime_error)? {
                        TerminalOutput::Data(data)=>exec_server_message::Msg::Output(ExecOutput {data,stream:0}),
                        TerminalOutput::Exit(exit_code)=>exec_server_message::Msg::Exit(ExecExit {exit_code}),
                    })},
                }
            }
        })))
    }
}

pub(super) fn summary(
    mut model: citadel_docker_api::models::ContainerSummary,
) -> Result<ContainerMessage, Status> {
    let command = model.command.take().unwrap_or_default();
    let size_rw = model.size_rw.flatten().unwrap_or_default();
    let size_root_fs = model.size_root_fs.flatten().unwrap_or_default();
    let c = container_summary(model.try_into().map_err(docker_error)?);
    Ok(ContainerMessage {
        id: c.id,
        image: c.image,
        image_id: c.image_id,
        command,
        created: c.created,
        size_rw,
        size_root_fs,
        status: c.status,
        state: mapping::state(&c.state),
        name: c.name,
        ports: c
            .ports
            .as_object()
            .into_iter()
            .flatten()
            .map(|(key, v)| (key.clone(), mapping::host_port_binding_list(v)))
            .collect(),
        container_stat_message: Some(ContainerStatMessage::default()),
        stack: Some(c.stack.unwrap_or_default()),
        stack_id: c
            .labels
            .get("com.citadel.stack-id")
            .cloned()
            .unwrap_or_default(),
        is_system: c.is_system,
        system_role: Some(c.system_role.unwrap_or_default()),
        has_citadel_ownership_labels: c.has_citadel_ownership_labels,
        is_swarm_task: c.is_swarm_task,
    })
}
fn stat(s: citadel_platforms::RuntimeContainerStat) -> ContainerStatMessage {
    ContainerStatMessage {
        memory_active: s.memory_active,
        memory_cache: s.memory_cache,
        cpu_usage: s.cpu_usage,
        memory_limit: s.memory_limit,
        rx_bytes: s.rx_bytes,
        tx_bytes: s.tx_bytes,
    }
}
