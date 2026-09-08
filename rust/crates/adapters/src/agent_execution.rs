//! Execution operations shared by direct and inbound Agents. Both use the
//! canonical protobuf messages; Edge changes delivery, not the payload model.
use crate::{
    agent::{AgentBinaryExecOutput, AgentClient, AgentContainerAction},
    edge::{EdgeError, EdgeSession},
};
use citadel_contracts::citadel::{
    containers::v1::{
        ContainerIds, CreateContainerRequest, CreateContainerResponse, DeleteContainerRequest,
        ExecBinaryRequest, ExecServerMessage, exec_server_message,
    },
    edge::v1::EdgeCommandKind,
};
use citadel_platforms::{RuntimeCapabilityError, RuntimeErrorKind};
use prost::Message;
use std::{sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub(crate) enum AgentExecutionClient {
    Direct(Arc<AgentClient>),
    Edge(Arc<EdgeSession>),
}
impl AgentExecutionClient {
    pub async fn apply_deployment(
        &self,
        command: &citadel_deployments::RuntimeDeploymentCommand,
        cancellation: &CancellationToken,
    ) -> Result<citadel_deployments::RuntimeDeploymentResult, RuntimeCapabilityError> {
        match self {
            Self::Direct(client) => client.apply_deployment(command, cancellation).await,
            Self::Edge(session) => {
                let response = unary(
                    session,
                    EdgeCommandKind::DeploymentApply,
                    crate::agent::workloads::deployment_request(command),
                    cancellation,
                )
                .await?;
                Ok(crate::agent::workloads::deployment_result(
                    command, response,
                ))
            }
        }
    }

    pub async fn apply_stack(
        &self,
        claim: &citadel_stacks::StackOperationClaim,
        source: &citadel_stacks::StackApplySource,
        environment: &[String],
        registry: Option<&crate::agent::AgentStackRegistry>,
        cancellation: &CancellationToken,
    ) -> Result<citadel_stacks::StackRuntimeResult, RuntimeCapabilityError> {
        match self {
            Self::Direct(client) => {
                client
                    .apply_stack(claim, source, environment, registry, cancellation)
                    .await
            }
            Self::Edge(session) => {
                let frames = stream::<_, citadel_contracts::citadel::stacks::v1::StackApplyResponse>(
                    session,
                    EdgeCommandKind::StackApplyStream,
                    crate::agent::workloads::stack_request(claim, source, environment, registry),
                    cancellation,
                )?;
                crate::agent::workloads::consume_stack_stream(frames, cancellation).await
            }
        }
    }

    pub async fn pull_deployment_image(
        &self,
        reference: &str,
        auth: Option<String>,
        cancellation: &CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        match self {
            Self::Direct(client) => {
                client
                    .pull_deployment_image(reference, auth, cancellation)
                    .await
            }
            Self::Edge(session) => {
                use citadel_contracts::citadel::images::v1::{
                    ImageBuildResponse, PullImageRequest,
                };
                let frames = stream::<_, ImageBuildResponse>(
                    session,
                    EdgeCommandKind::ImagePullStream,
                    PullImageRequest {
                        from_image: reference.into(),
                        auth,
                        ..Default::default()
                    },
                    cancellation,
                )?;
                crate::agent::consume_image_build_stream(frames, 0, None, cancellation).await?;
                Ok(())
            }
        }
    }

    pub async fn list_images(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<Vec<citadel_platforms::RuntimeImageSummary>, RuntimeCapabilityError> {
        use citadel_platforms::PlatformInventoryPort;
        match self {
            Self::Direct(client) => client.list_images(cancellation).await,
            Self::Edge(session) => {
                crate::edge::EdgeRuntime {
                    session: session.clone(),
                }
                .list_images(cancellation)
                .await
            }
        }
    }

    pub async fn list_containers(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<Vec<citadel_platforms::RuntimeContainerSummary>, RuntimeCapabilityError> {
        use citadel_platforms::PlatformRuntimePort;
        match self {
            Self::Direct(client) => client.list_containers(cancellation).await,
            Self::Edge(session) => {
                crate::edge::EdgeRuntime {
                    session: session.clone(),
                }
                .list_containers(cancellation)
                .await
            }
        }
    }

    pub async fn list_networks(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<Vec<citadel_platforms::RuntimeNetworkSummary>, RuntimeCapabilityError> {
        use citadel_platforms::PlatformInventoryPort;
        match self {
            Self::Direct(client) => client.list_networks(cancellation).await,
            Self::Edge(session) => {
                crate::edge::EdgeRuntime {
                    session: session.clone(),
                }
                .list_networks(cancellation)
                .await
            }
        }
    }

    pub async fn delete_network(
        &self,
        id: &str,
        cancellation: &CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        use citadel_platforms::PlatformResourceMutationPort;
        match self {
            Self::Direct(client) => client.delete_network(id, cancellation).await,
            Self::Edge(session) => {
                unary(
                    session,
                    EdgeCommandKind::NetworkDelete,
                    citadel_contracts::citadel::networks::v1::DeleteNetworkRequest {
                        ids: vec![id.into()],
                    },
                    cancellation,
                )
                .await
            }
        }
    }

    pub async fn inspect_managed_swarm_service(
        &self,
        id: &str,
        cancellation: &CancellationToken,
    ) -> Result<citadel_contracts::citadel::swarm::v1::SwarmServiceMessage, RuntimeCapabilityError>
    {
        match self {
            Self::Direct(client) => client.inspect_managed_swarm_service(id, cancellation).await,
            Self::Edge(session) => {
                unary(
                    session,
                    EdgeCommandKind::SwarmServiceInspect,
                    citadel_contracts::citadel::swarm::v1::InspectSwarmServiceRequest {
                        service_id: id.into(),
                    },
                    cancellation,
                )
                .await
            }
        }
    }

    pub async fn create_managed_swarm_service(
        &self,
        request: citadel_contracts::citadel::swarm::v1::CreateManagedSwarmServiceRequest,
        cancellation: &CancellationToken,
    ) -> Result<
        citadel_contracts::citadel::swarm::v1::SwarmServiceMutationResponse,
        RuntimeCapabilityError,
    > {
        match self {
            Self::Direct(client) => {
                client
                    .create_managed_swarm_service(request, cancellation)
                    .await
            }
            Self::Edge(session) => {
                unary(
                    session,
                    EdgeCommandKind::SwarmServiceCreate,
                    request,
                    cancellation,
                )
                .await
            }
        }
    }

    pub async fn update_managed_swarm_service(
        &self,
        request: citadel_contracts::citadel::swarm::v1::UpdateManagedSwarmServiceRequest,
        cancellation: &CancellationToken,
    ) -> Result<
        citadel_contracts::citadel::swarm::v1::SwarmServiceMutationResponse,
        RuntimeCapabilityError,
    > {
        match self {
            Self::Direct(client) => {
                client
                    .update_managed_swarm_service(request, cancellation)
                    .await
            }
            Self::Edge(session) => {
                unary(
                    session,
                    EdgeCommandKind::SwarmServiceUpdate,
                    request,
                    cancellation,
                )
                .await
            }
        }
    }

    pub async fn delete_managed_swarm_service(
        &self,
        request: citadel_contracts::citadel::swarm::v1::DeleteManagedSwarmServiceRequest,
        cancellation: &CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        match self {
            Self::Direct(client) => {
                client
                    .delete_managed_swarm_service(request, cancellation)
                    .await
            }
            Self::Edge(session) => {
                unary(
                    session,
                    EdgeCommandKind::SwarmServiceDelete,
                    request,
                    cancellation,
                )
                .await
            }
        }
    }

    pub async fn volume_exists(
        &self,
        name: &str,
        cancellation: &CancellationToken,
    ) -> Result<bool, RuntimeCapabilityError> {
        match self {
            Self::Direct(client) => match citadel_platforms::PlatformInventoryPort::inspect_volume(
                client.as_ref(),
                name,
                cancellation,
            )
            .await
            {
                Ok(_) => Ok(true),
                Err(error) if error.kind == RuntimeErrorKind::NotFound => Ok(false),
                Err(error) => Err(error),
            },
            Self::Edge(session) => {
                use citadel_contracts::citadel::volumes::v1::{
                    ListVolumesRequest, ListVolumesResponse,
                };
                let response: ListVolumesResponse = unary(
                    session,
                    EdgeCommandKind::VolumeList,
                    ListVolumesRequest {
                        name: Some(name.into()),
                        ..Default::default()
                    },
                    cancellation,
                )
                .await?;
                Ok(response.volumes.iter().any(|volume| volume.name == name))
            }
        }
    }
    pub(crate) async fn build_image(
        &self,
        command: crate::agent::AgentBuildCommand,
        cancellation: &CancellationToken,
    ) -> Result<String, RuntimeCapabilityError> {
        use citadel_contracts::citadel::images::v1::{
            BuildImageRequest, BuildImageSecret, ImageBuildResponse, PushImageRequest,
        };
        let session = match self {
            Self::Direct(client) => return client.build_image(command, cancellation).await,
            Self::Edge(session) => session,
        };
        if cancellation.is_cancelled() {
            return Err(remote("Edge Build canceled."));
        }
        let timeout = Duration::from_secs(command.timeout_seconds.max(1) as u64);
        let maximum = command.maximum_log_bytes.max(1024);
        let request = BuildImageRequest {
            context_directory: ".".into(),
            dockerfile_path: command.dockerfile_path.clone(),
            tags: command.tags.clone(),
            build_args: command.build_args,
            target: command.target,
            registry_auth: command.registry_auth.clone(),
            registry_host: command.registry_host,
            timeout_seconds: command.timeout_seconds,
            max_line_bytes: 16 * 1024,
            context_archive: command.context_archive,
            dockerfile_archive_path: Some(command.dockerfile_path),
            build_secrets: command
                .secrets
                .into_iter()
                .map(|(id, value)| BuildImageSecret { id, value })
                .collect(),
        };
        let mut pending = session
            .command(
                EdgeCommandKind::ImageBuildStream,
                request.encode_to_vec(),
                timeout,
                true,
            )
            .map_err(edge_error)?;
        let frames = async_stream::stream! {
            loop {
                match pending.next(cancellation).await {
                    Ok(Some(payload)) => yield ImageBuildResponse::decode(payload.as_slice()).map_err(|_| tonic::Status::data_loss("Invalid Edge Build response.")),
                    Ok(None) => break,
                    Err(error) => { yield Err(tonic::Status::unavailable(error.to_string())); break; }
                }
            }
        };
        let mut output = crate::agent::consume_image_build_stream(
            Box::pin(frames),
            maximum,
            command.output.as_ref(),
            cancellation,
        )
        .await?;
        for reference in command.tags {
            let request = PushImageRequest {
                image_reference: reference,
                registry_auth: command.registry_auth.clone(),
            };
            let mut pending = session
                .command(
                    EdgeCommandKind::ImagePushStream,
                    request.encode_to_vec(),
                    timeout,
                    true,
                )
                .map_err(edge_error)?;
            let frames = async_stream::stream! {
                loop {
                    match pending.next(cancellation).await {
                        Ok(Some(payload)) => yield ImageBuildResponse::decode(payload.as_slice()).map_err(|_| tonic::Status::data_loss("Invalid Edge Push response.")),
                        Ok(None) => break,
                        Err(error) => { yield Err(tonic::Status::unavailable(error.to_string())); break; }
                    }
                }
            };
            let pushed = crate::agent::consume_image_build_stream(
                Box::pin(frames),
                maximum.saturating_sub(output.len()),
                command.output.as_ref(),
                cancellation,
            )
            .await?;
            output.push_str(&pushed);
        }
        Ok(output)
    }
    pub async fn create_container(
        &self,
        request: CreateContainerRequest,
        cancellation: &CancellationToken,
    ) -> Result<String, RuntimeCapabilityError> {
        match self {
            Self::Direct(client) => client.create_container(request, cancellation).await,
            Self::Edge(session) => {
                let response: CreateContainerResponse = unary(
                    session,
                    EdgeCommandKind::ContainerCreate,
                    request,
                    cancellation,
                )
                .await?;
                if response.container_id.is_empty() || response.container_id.len() > 256 {
                    return Err(remote("Edge Agent returned an invalid Container id."));
                }
                Ok(response.container_id)
            }
        }
    }
    pub async fn delete_container(
        &self,
        id: &str,
        cancellation: &CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        match self {
            Self::Direct(client) => client.delete_container(id, cancellation).await,
            Self::Edge(session) => {
                unary(
                    session,
                    EdgeCommandKind::ContainerDelete,
                    DeleteContainerRequest {
                        ids: vec![id.into()],
                        force: Some(true),
                        v: Some(true),
                        link: Some(false),
                    },
                    cancellation,
                )
                .await
            }
        }
    }
    pub async fn change_containers_state(
        &self,
        ids: &[String],
        action: AgentContainerAction,
        cancellation: &CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        match self {
            Self::Direct(client) => {
                client
                    .change_containers_state(ids, action, cancellation)
                    .await
            }
            Self::Edge(session) => {
                let kind = match action {
                    AgentContainerAction::Start => EdgeCommandKind::ContainerStart,
                    AgentContainerAction::Stop => EdgeCommandKind::ContainerStop,
                    AgentContainerAction::Restart => EdgeCommandKind::ContainerRestart,
                    AgentContainerAction::Pause => EdgeCommandKind::ContainerPause,
                    AgentContainerAction::Unpause => EdgeCommandKind::ContainerUnpause,
                };
                unary(
                    session,
                    kind,
                    ContainerIds { ids: ids.to_vec() },
                    cancellation,
                )
                .await
            }
        }
    }
    pub async fn exec_binary(
        &self,
        request: ExecBinaryRequest,
        timeout: Duration,
        maximum: usize,
        cancellation: &CancellationToken,
    ) -> Result<AgentBinaryExecOutput, RuntimeCapabilityError> {
        self.exec_binary_observed(request,timeout,maximum,cancellation,None).await
    }
    pub(crate) async fn exec_binary_observed(&self,request:ExecBinaryRequest,timeout:Duration,maximum:usize,cancellation:&CancellationToken,output_sender:Option<tokio::sync::mpsc::Sender<citadel_execution::ProcessChunk>>) -> Result<AgentBinaryExecOutput,RuntimeCapabilityError> {
        let Self::Edge(session) = self else {
            let Self::Direct(client) = self else {
                unreachable!()
            };
            return client
                .exec_binary_observed(request, timeout, maximum, cancellation,output_sender)
                .await;
        };
        if cancellation.is_cancelled() {
            return Err(remote("Edge execution canceled."));
        }
        let mut pending = session
            .command(
                EdgeCommandKind::ContainerExecBinary,
                request.encode_to_vec(),
                timeout,
                true,
            )
            .map_err(edge_error)?;
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut exit_code = None;
        while let Some(payload) = pending.next(cancellation).await.map_err(edge_error)? {
            let message = ExecServerMessage::decode(payload.as_slice())
                .map_err(|_| remote("Invalid Edge execution response."))?;
            match message.msg {
                Some(exec_server_message::Msg::Output(output)) => {
                    if stdout
                        .len()
                        .saturating_add(stderr.len())
                        .saturating_add(output.data.len())
                        > maximum
                    {
                        return Err(remote("Edge execution exceeded the output limit."));
                    }
                    if output.stream == 1 {
                        stderr.extend_from_slice(&output.data);
                    } else {
                        stdout.extend_from_slice(&output.data);
                    }
                    if let Some(sender)=&output_sender {let _=sender.send(citadel_execution::ProcessChunk{stream:if output.stream==1{"stderr"}else{"stdout"},bytes:output.data}).await;}
                }
                Some(exec_server_message::Msg::Exit(exit)) => exit_code = Some(exit.exit_code),
                Some(exec_server_message::Msg::Error(_)) => {
                    return Err(remote("Edge binary execution failed."));
                }
                None => {}
            }
        }
        Ok(AgentBinaryExecOutput {
            stdout,
            stderr,
            exit_code: exit_code
                .ok_or_else(|| remote("Edge execution ended without an exit code."))?,
        })
    }
}

pub(crate) async fn unary<T: Message, R: Message + Default + 'static>(
    session: &Arc<EdgeSession>,
    kind: EdgeCommandKind,
    request: T,
    cancellation: &CancellationToken,
) -> Result<R, RuntimeCapabilityError> {
    if cancellation.is_cancelled() {
        return Err(remote("Edge command canceled."));
    }
    let mut command = session
        .command(
            kind,
            request.encode_to_vec(),
            Duration::from_secs(120),
            false,
        )
        .map_err(edge_error)?;
    let mut response = None;
    while let Some(bytes) = command.next(cancellation).await.map_err(edge_error)? {
        if response.is_some() {
            return Err(remote("Edge unary command returned multiple responses."));
        }
        response = Some(
            R::decode(bytes.as_slice())
                .map_err(|_| remote("Edge command returned invalid protobuf."))?,
        );
    }
    match response {
        Some(response) => Ok(response),
        None if std::any::TypeId::of::<R>() == std::any::TypeId::of::<()>() => Ok(R::default()),
        None => Err(remote(
            "Edge command completed without its required response.",
        )),
    }
}

pub(crate) fn stream<T: Message, R: Message + Default + Send + 'static>(
    session: &Arc<EdgeSession>,
    kind: EdgeCommandKind,
    request: T,
    cancellation: &CancellationToken,
) -> Result<
    futures_util::stream::BoxStream<'static, Result<R, tonic::Status>>,
    RuntimeCapabilityError,
> {
    if cancellation.is_cancelled() {
        return Err(remote("Edge command canceled."));
    }
    let mut pending = session
        .command(
            kind,
            request.encode_to_vec(),
            Duration::from_secs(1800),
            true,
        )
        .map_err(edge_error)?;
    let cancellation = cancellation.clone();
    Ok(Box::pin(async_stream::stream! {
        loop {
            match pending.next(&cancellation).await {
                Ok(Some(payload)) => yield R::decode(payload.as_slice()).map_err(|_| tonic::Status::data_loss("Invalid Edge response.")),
                Ok(None) => break,
                Err(error) => { yield Err(tonic::Status::unavailable(error.to_string())); break; }
            }
        }
    }))
}
fn remote(message: &str) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Remote, message, false)
}
fn edge_error(error: EdgeError) -> RuntimeCapabilityError {
    remote(error.0)
}

#[cfg(test)]
#[path = "agent_execution_tests.rs"]
mod tests;
