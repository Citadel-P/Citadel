use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use citadel_adapters::agent::{AgentClient, AgentContainerAction, AgentRequestSigner};
use citadel_contracts::citadel::containers::v1::container_service_server::{
    ContainerService, ContainerServiceServer,
};
use citadel_contracts::citadel::containers::v1::{
    ContainerIds, ContainerLogRequest, ContainerLogResponse, ContainersStatsResponse,
    CreateContainerRequest, CreateContainerResponse, DeleteContainerRequest, ExecBinaryRequest,
    ExecClientMessage, ExecExit, ExecOutput, ExecServerMessage, InspectContainerRequest,
    ListContainersRequest, ListContainersResponse, StreamContainerStatsRequest,
    StreamContainersStatsRequest, StreamType, exec_server_message,
};
use citadel_contracts::citadel::deployments::v1::deployment_service_server::{
    DeploymentService as AgentDeploymentService, DeploymentServiceServer,
};
use citadel_contracts::citadel::deployments::v1::{
    ApplyDeploymentRequest, ApplyDeploymentResponse, DeployedContainerState,
};
use citadel_contracts::citadel::networks::v1::network_service_server::{
    NetworkService, NetworkServiceServer,
};
use citadel_contracts::citadel::networks::v1::{
    CreateNetworkRequest, CreateNetworkResponse, DeleteNetworkRequest, DeleteNetworkResponse,
    InspectNetworkRequest, InspectNetworkResponse, ListNetworksRequest, ListNetworksResponse,
};
use citadel_contracts::citadel::shared_models::v1::VolumeResponse;
use citadel_contracts::citadel::shared_models::v1::{ContainerMessage, InspectContainerResponse};
use citadel_contracts::citadel::stacks::v1::stack_service_server::{
    StackService as AgentStackService, StackServiceServer,
};
use citadel_contracts::citadel::stacks::v1::{
    StackApplyEventType, StackApplyRequest, StackApplyResponse, StackOrchestrationMode,
};
use citadel_contracts::citadel::volumes::v1::volume_service_server::{
    VolumeService, VolumeServiceServer,
};
use citadel_contracts::citadel::volumes::v1::{
    CreateVolumeRequest, InspectVolumeRequest, ListVolumesRequest, ListVolumesResponse,
    RemoveVolumeRequest, RemoveVolumeResponse,
};
use citadel_deployments::{
    DeploymentImageInfo, DeploymentSpec, RuntimeContainerState, RuntimeDeploymentCommand,
    UpdateBehavior,
};
use citadel_platforms::terminal::*;
use citadel_platforms::{
    CreateRuntimeNetwork, CreateRuntimeVolume, PlatformResourceMutationPort, RuntimeErrorKind,
};
use citadel_stacks::{
    StackApplySource, StackOperationClaim, StackSourceFile, StackSpec, StackSpecCommon,
    StackUpdateBehavior,
};
use futures_util::{Stream, StreamExt};
use prost::Message;
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;
use tonic::{Request, Response, Status};

#[derive(Clone)]
struct MutationFixture {
    fail_network_create: bool,
    network_create_calls: Arc<AtomicUsize>,
    container_delete_calls: Arc<AtomicUsize>,
    deployment_apply_calls: Arc<AtomicUsize>,
    container_action_calls: Arc<AtomicUsize>,
    stack_apply_calls: Arc<AtomicUsize>,
}

#[tonic::async_trait]
impl AgentDeploymentService for MutationFixture {
    async fn apply(
        &self,
        request: Request<ApplyDeploymentRequest>,
    ) -> Result<Response<ApplyDeploymentResponse>, Status> {
        require_signature(&request)?;
        let request = request.get_ref();
        assert_eq!(request.image_id, "sha256:image");
        assert_eq!(request.name, "web");
        let spec = request.spec.as_ref().expect("Deployment spec");
        assert_eq!(spec.env_vars, ["TOKEN=resolved"]);
        assert_eq!(
            spec.labels.get("owner").map(String::as_str),
            Some("citadel")
        );
        self.deployment_apply_calls.fetch_add(1, Ordering::Relaxed);
        Ok(Response::new(ApplyDeploymentResponse {
            container_id: "container-applied".to_owned(),
            deployed_container_state: DeployedContainerState::Running as i32,
        }))
    }
}

type TestStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send>>;

#[tonic::async_trait]
impl ContainerService for MutationFixture {
    async fn list(
        &self,
        _: Request<ListContainersRequest>,
    ) -> Result<Response<ListContainersResponse>, Status> {
        Err(Status::unimplemented("not used"))
    }

    async fn start(&self, request: Request<ContainerIds>) -> Result<Response<()>, Status> {
        require_signature(&request)?;
        assert!(
            request.get_ref().ids == ["container-1", "container-2"]
                || request.get_ref().ids == ["backup-helper"]
        );
        self.container_action_calls.fetch_add(1, Ordering::Relaxed);
        Ok(Response::new(()))
    }

    async fn stop(&self, _: Request<ContainerIds>) -> Result<Response<()>, Status> {
        Err(Status::unimplemented("not used"))
    }

    async fn pause(&self, _: Request<ContainerIds>) -> Result<Response<()>, Status> {
        Err(Status::unimplemented("not used"))
    }

    async fn unpause(&self, _: Request<ContainerIds>) -> Result<Response<()>, Status> {
        Err(Status::unimplemented("not used"))
    }

    async fn restart(&self, _: Request<ContainerIds>) -> Result<Response<()>, Status> {
        Err(Status::unimplemented("not used"))
    }

    async fn delete(
        &self,
        request: Request<DeleteContainerRequest>,
    ) -> Result<Response<()>, Status> {
        require_signature(&request)?;
        let request = request.get_ref();
        assert!(request.ids == ["container-1"] || request.ids == ["backup-helper"]);
        assert_eq!(request.v, Some(true));
        assert_eq!(request.force, Some(true));
        assert_eq!(request.link, Some(false));
        self.container_delete_calls.fetch_add(1, Ordering::Relaxed);
        Ok(Response::new(()))
    }

    async fn inspect(
        &self,
        request: Request<InspectContainerRequest>,
    ) -> Result<Response<InspectContainerResponse>, Status> {
        require_signature(&request)?;
        use citadel_contracts::citadel::shared_models::v1::{
            ContainerConfig, ContainerState, ContainerStateType,
        };
        match request.get_ref().container_id.as_str() {
            "missing" => return Err(Status::not_found("Container not found")),
            "denied" => return Err(Status::permission_denied("Inspection denied")),
            "container-1" => (),
            other => panic!("unexpected inspection target: {other}"),
        }
        Ok(Response::new(InspectContainerResponse {
            id: "container-1".into(),
            config: Some(ContainerConfig {
                image: Some("busybox:latest".into()),
                env: vec![
                    "CITADEL_VAULT_SECRET=agent-private".into(),
                    "APP_MODE=production".into(),
                ],
                ..Default::default()
            }),
            state: Some(ContainerState {
                status: ContainerStateType::Running as i32,
                ..Default::default()
            }),
            ..Default::default()
        }))
    }

    async fn create(
        &self,
        request: Request<CreateContainerRequest>,
    ) -> Result<Response<CreateContainerResponse>, Status> {
        require_signature(&request)?;
        Ok(Response::new(CreateContainerResponse {
            container_id: "backup-helper".to_owned(),
        }))
    }

    type ExecStream = TestStream<ExecServerMessage>;

    async fn exec(
        &self,
        request: Request<tonic::Streaming<ExecClientMessage>>,
    ) -> Result<Response<Self::ExecStream>, Status> {
        let (metadata, _, mut input) = request.into_parts();
        let open = input
            .message()
            .await?
            .ok_or_else(|| Status::unauthenticated("missing open"))?;
        let hash = Sha256::digest(open.encode_to_vec());
        let header = |name| {
            metadata
                .get_bin(name)
                .ok_or_else(|| Status::unauthenticated("missing signature header"))?
                .to_bytes()
                .map_err(|_| Status::unauthenticated("invalid header"))
        };
        assert_eq!(header("x-content-sha256-bin")?.as_ref(), hash.as_slice());
        let mut payload = header("x-timestamp-bin")?.to_vec();
        payload.extend_from_slice(&header("x-nonce-bin")?);
        payload.extend_from_slice(b"/citadel.containers.v1.ContainerService/Exec");
        payload.extend_from_slice(&hash);
        let signature = ed25519_dalek::Signature::from_slice(&header("x-signature-bin")?).unwrap();
        ed25519_dalek::SigningKey::from_bytes(&[31; 32])
            .verifying_key()
            .verify_strict(&payload, &signature)
            .unwrap();
        let Some(citadel_contracts::citadel::containers::v1::exec_client_message::Msg::Open(open)) =
            open.msg
        else {
            panic!("expected signed Open")
        };
        assert_eq!(open.container_id, "container-1");
        assert_eq!(open.cmd, ["/bin/sh"]);
        assert!(open.tty);
        Ok(Response::new(Box::pin(async_stream::try_stream! {
            while let Some(message) = input.message().await? {
                use citadel_contracts::citadel::containers::v1::exec_client_message::Msg;
                let data = match message.msg.unwrap() {
                    Msg::Stdin(stdin)=>stdin.data,
                    Msg::Resize(resize)=>format!("{}x{}",resize.cols,resize.rows).into_bytes(),
                    Msg::Open(_)=>panic!("Open sent twice"),
                };
                yield ExecServerMessage {msg:Some(exec_server_message::Msg::Output(ExecOutput {data,stream:0}))};
            }
        })))
    }

    type ExecBinaryStream = TestStream<ExecServerMessage>;

    async fn exec_binary(
        &self,
        request: Request<ExecBinaryRequest>,
    ) -> Result<Response<Self::ExecBinaryStream>, Status> {
        require_signature(&request)?;
        assert_eq!(request.get_ref().container_id, "backup-helper");
        Ok(Response::new(Box::pin(futures_util::stream::iter([
            Ok(ExecServerMessage {
                msg: Some(exec_server_message::Msg::Output(ExecOutput {
                    data: b"{\"message_type\":\"summary\"}\n".to_vec(),
                    stream: StreamType::Stdout as i32,
                })),
            }),
            Ok(ExecServerMessage {
                msg: Some(exec_server_message::Msg::Exit(ExecExit { exit_code: 0 })),
            }),
        ]))))
    }

    type StreamContainerLogsStream = TestStream<ContainerLogResponse>;

    async fn stream_container_logs(
        &self,
        request: Request<ContainerLogRequest>,
    ) -> Result<Response<Self::StreamContainerLogsStream>, Status> {
        require_signature(&request)?;
        let request = request.into_inner();
        assert_eq!(request.container_id, "container-1");
        if request.follow == Some(false) {
            assert_eq!(request.tail, 12);
            return Ok(Response::new(Box::pin(futures_util::stream::iter([Ok(
                ContainerLogResponse {
                    log: b"bounded logs\n".to_vec(),
                },
            )]))));
        }
        assert_eq!(request.tail, 100);
        assert_eq!(request.follow, Some(true));
        Ok(Response::new(Box::pin(
            futures_util::stream::once(async {
                Ok(ContainerLogResponse {
                    log: b"2026-09-06T12:00:00Z hello\n".to_vec(),
                })
            })
            .chain(futures_util::stream::pending()),
        )))
    }

    type StreamContainersStatsStream = TestStream<ContainersStatsResponse>;

    async fn stream_containers_stats(
        &self,
        _: Request<StreamContainersStatsRequest>,
    ) -> Result<Response<Self::StreamContainersStatsStream>, Status> {
        Err(Status::unimplemented("not used"))
    }

    type StreamContainerStatsStream = TestStream<ContainerMessage>;

    async fn stream_container_stats(
        &self,
        _: Request<StreamContainerStatsRequest>,
    ) -> Result<Response<Self::StreamContainerStatsStream>, Status> {
        Err(Status::unimplemented("not used"))
    }
}

#[tonic::async_trait]
impl AgentStackService for MutationFixture {
    type ApplyStream = TestStream<StackApplyResponse>;

    async fn apply(
        &self,
        request: Request<StackApplyRequest>,
    ) -> Result<Response<Self::ApplyStream>, Status> {
        require_signature(&request)?;
        let request = request.get_ref();
        assert_eq!(request.project_name.as_deref(), Some("agent-stack"));
        assert!(request.compose_file_content.is_none());
        assert_eq!(request.source_compose_file_paths, ["compose.yml"]);
        assert_eq!(request.source_files.len(), 1);
        assert_eq!(
            request.source_files[0].content,
            b"services:\n  web:\n    image: nginx\n"
        );
        assert_eq!(request.environment_variables, ["TOKEN=resolved"]);
        assert_eq!(request.service_names, ["web"]);
        assert!(request.pull_images);
        assert!(
            !request.destroy_before_deploy,
            "a scoped Apply must not stop unrelated Services"
        );
        assert_eq!(
            request.pre_deploy.as_ref().unwrap().commands,
            ["echo preparing"]
        );
        assert_eq!(
            request
                .post_deploy
                .as_ref()
                .map(|command| command.path.as_str()),
            Some("scripts")
        );
        assert_eq!(
            request.orchestration_mode,
            StackOrchestrationMode::DockerCompose as i32
        );
        self.stack_apply_calls.fetch_add(1, Ordering::Relaxed);
        Ok(Response::new(Box::pin(futures_util::stream::iter([Ok(
            StackApplyResponse {
                r#type: StackApplyEventType::CommandCompleted as i32,
                message: Some("done".to_owned()),
                exit_code: Some(0),
                stack_status: Some("Healthy".to_owned()),
            },
        )]))))
    }
}

#[tonic::async_trait]
impl NetworkService for MutationFixture {
    async fn list(
        &self,
        _: Request<ListNetworksRequest>,
    ) -> Result<Response<ListNetworksResponse>, Status> {
        Err(Status::unimplemented("not used"))
    }

    async fn create(
        &self,
        request: Request<CreateNetworkRequest>,
    ) -> Result<Response<CreateNetworkResponse>, Status> {
        require_signature(&request)?;
        self.network_create_calls.fetch_add(1, Ordering::Relaxed);
        if self.fail_network_create {
            return Err(Status::unavailable("temporary"));
        }
        assert_eq!(request.get_ref().name, "frontend");
        assert_eq!(request.get_ref().driver.as_deref(), Some("overlay"));
        Ok(Response::new(CreateNetworkResponse {
            id: "network-1".into(),
        }))
    }

    async fn delete(
        &self,
        request: Request<DeleteNetworkRequest>,
    ) -> Result<Response<DeleteNetworkResponse>, Status> {
        require_signature(&request)?;
        assert_eq!(request.get_ref().ids, ["network-1"]);
        Ok(Response::new(DeleteNetworkResponse {}))
    }

    async fn inspect(
        &self,
        _: Request<InspectNetworkRequest>,
    ) -> Result<Response<InspectNetworkResponse>, Status> {
        Err(Status::unimplemented("not used"))
    }
}

#[tonic::async_trait]
impl VolumeService for MutationFixture {
    async fn list(
        &self,
        _: Request<ListVolumesRequest>,
    ) -> Result<Response<ListVolumesResponse>, Status> {
        Err(Status::unimplemented("not used"))
    }

    async fn inspect(
        &self,
        _: Request<InspectVolumeRequest>,
    ) -> Result<Response<VolumeResponse>, Status> {
        Err(Status::unimplemented("not used"))
    }

    async fn create(
        &self,
        request: Request<CreateVolumeRequest>,
    ) -> Result<Response<VolumeResponse>, Status> {
        require_signature(&request)?;
        assert_eq!(request.get_ref().name, "data");
        Ok(Response::new(VolumeResponse {
            name: "data".into(),
            driver: "local".into(),
            scope: "local".into(),
            ..VolumeResponse::default()
        }))
    }

    async fn remove(
        &self,
        request: Request<RemoveVolumeRequest>,
    ) -> Result<Response<RemoveVolumeResponse>, Status> {
        require_signature(&request)?;
        assert_eq!(request.get_ref().names, ["data"]);
        assert!(request.get_ref().force);
        Ok(Response::new(RemoveVolumeResponse {}))
    }
}

#[tokio::test]
async fn agent_network_volume_and_deployment_mutations_are_signed_and_transport_equivalent() {
    let calls = Arc::new(AtomicUsize::new(0));
    let container_calls = Arc::new(AtomicUsize::new(0));
    let deployment_calls = Arc::new(AtomicUsize::new(0));
    let action_calls = Arc::new(AtomicUsize::new(0));
    let stack_calls = Arc::new(AtomicUsize::new(0));
    let (address, shutdown) = start_fixture(MutationFixture {
        fail_network_create: false,
        network_create_calls: calls.clone(),
        container_delete_calls: container_calls.clone(),
        deployment_apply_calls: deployment_calls.clone(),
        container_action_calls: action_calls.clone(),
        stack_apply_calls: stack_calls.clone(),
    })
    .await;
    let client = connect(&address).await;
    let cancellation = CancellationToken::new();
    assert_eq!(
        PlatformResourceMutationPort::create_network(&client, &network_input(), &cancellation,)
            .await
            .unwrap()
            .id,
        "network-1"
    );
    PlatformResourceMutationPort::delete_network(&client, "network-1", &cancellation)
        .await
        .unwrap();
    assert_eq!(
        PlatformResourceMutationPort::create_volume(
            &client,
            &CreateRuntimeVolume {
                name: "data".into(),
                driver: "local".into(),
                labels: Default::default(),
                options: Default::default(),
            },
            &cancellation,
        )
        .await
        .unwrap()
        .name,
        "data"
    );
    PlatformResourceMutationPort::delete_volume(&client, "data", true, &cancellation)
        .await
        .unwrap();
    client
        .delete_container("container-1", &cancellation)
        .await
        .unwrap();
    client
        .change_containers_state(
            &["container-1".to_owned(), "container-2".to_owned()],
            AgentContainerAction::Start,
            &cancellation,
        )
        .await
        .unwrap();
    let applied = client
        .apply_deployment(&deployment_command(), &cancellation)
        .await
        .unwrap();
    assert_eq!(applied.docker_container_id, "container-applied");
    assert_eq!(applied.state, RuntimeContainerState::Running);
    let stack = client
        .apply_stack(
            &stack_claim(),
            &StackApplySource {
                files: vec![StackSourceFile {
                    relative_path: "compose.yml".to_owned(),
                    content: b"services:\n  web:\n    image: nginx\n".to_vec(),
                }],
                compose_paths: vec!["compose.yml".to_owned()],
                env_file_paths: Vec::new(),
                working_directory: ".".to_owned(),
                labels_override_path: None,
                resolved_commit_sha: None,
            },
            &["TOKEN=resolved".to_owned()],
            None,
            &cancellation,
        )
        .await
        .unwrap();
    assert_eq!(stack.status, citadel_stacks::StackReleaseStatus::Healthy);
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    assert_eq!(container_calls.load(Ordering::Relaxed), 1);
    assert_eq!(deployment_calls.load(Ordering::Relaxed), 1);
    assert_eq!(action_calls.load(Ordering::Relaxed), 1);
    assert_eq!(stack_calls.load(Ordering::Relaxed), 1);
    use citadel_platforms::logs::ContainerLogPort;
    use citadel_platforms::logs::{LogReadPort, LogResource};
    let snapshot = client
        .read_logs(LogResource::Container("container-1"), 12, &cancellation)
        .await
        .unwrap();
    assert_eq!(snapshot.lines, ["bounded logs\n"]);
    assert!(!snapshot.truncated);
    let log_cancel = CancellationToken::new();
    let mut logs = client
        .container_logs("container-1", &log_cancel)
        .await
        .unwrap();
    assert_eq!(
        logs.next().await.unwrap().unwrap(),
        b"2026-09-06T12:00:00Z hello\n"
    );
    log_cancel.cancel();
    assert!(
        tokio::time::timeout(Duration::from_secs(1), logs.next())
            .await
            .unwrap()
            .is_none()
    );
    shutdown.cancel();
}

#[tokio::test]
async fn terminal_signs_exact_open_and_streams_input_resize_and_cancellation() {
    let (address, shutdown) = start_fixture(MutationFixture {
        fail_network_create: false,
        network_create_calls: Arc::default(),
        container_delete_calls: Arc::default(),
        deployment_apply_calls: Arc::default(),
        container_action_calls: Arc::default(),
        stack_apply_calls: Arc::default(),
    })
    .await;
    let client = connect(&address).await;
    let cancel = CancellationToken::new();
    let mut session = client
        .container_terminal("container-1", TerminalShell::Sh, &cancel)
        .await
        .unwrap();
    for (input, expected) in [
        (
            TerminalInput::Stdin(b"echo hello\n".to_vec()),
            b"echo hello\n".as_slice(),
        ),
        (
            TerminalInput::Resize {
                cols: 100,
                rows: 30,
            },
            b"100x30".as_slice(),
        ),
    ] {
        session.input.try_send(input).unwrap();
        let output = tokio::time::timeout(Duration::from_secs(2), session.output.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(matches!(output,TerminalOutput::Data(bytes) if bytes==expected));
    }
    cancel.cancel();
    assert!(
        tokio::time::timeout(Duration::from_secs(2), session.output.next())
            .await
            .unwrap()
            .is_none()
    );
    drop(session.output);
    // The request-body task must also terminate, not keep stdin alive after disconnect.
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if session.input.is_closed() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    shutdown.cancel();
}

#[tokio::test]
async fn agent_mutations_do_not_retry_an_ambiguous_failure() {
    let calls = Arc::new(AtomicUsize::new(0));
    let (address, shutdown) = start_fixture(MutationFixture {
        fail_network_create: true,
        network_create_calls: calls.clone(),
        container_delete_calls: Arc::new(AtomicUsize::new(0)),
        deployment_apply_calls: Arc::new(AtomicUsize::new(0)),
        container_action_calls: Arc::new(AtomicUsize::new(0)),
        stack_apply_calls: Arc::new(AtomicUsize::new(0)),
    })
    .await;
    let error = PlatformResourceMutationPort::create_network(
        &connect(&address).await,
        &network_input(),
        &CancellationToken::new(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.kind, RuntimeErrorKind::Unavailable);
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    shutdown.cancel();
}

#[tokio::test]
async fn signed_agent_container_create_and_binary_exec_are_bounded() {
    let (address, shutdown) = start_fixture(MutationFixture {
        fail_network_create: false,
        network_create_calls: Arc::new(AtomicUsize::new(0)),
        container_delete_calls: Arc::new(AtomicUsize::new(0)),
        deployment_apply_calls: Arc::new(AtomicUsize::new(0)),
        container_action_calls: Arc::new(AtomicUsize::new(0)),
        stack_apply_calls: Arc::new(AtomicUsize::new(0)),
    })
    .await;
    let client = connect(&address).await;
    let cancellation = CancellationToken::new();
    let container_id = client
        .create_container(
            CreateContainerRequest {
                name: "backup-helper".to_owned(),
                image_id: "restic/restic:0.18.1".to_owned(),
                ..Default::default()
            },
            &cancellation,
        )
        .await
        .unwrap();
    client
        .change_containers_state(
            std::slice::from_ref(&container_id),
            AgentContainerAction::Start,
            &cancellation,
        )
        .await
        .unwrap();
    let output = client
        .exec_binary(
            ExecBinaryRequest {
                container_id: container_id.clone(),
                cmd: vec!["restic".to_owned(), "backup".to_owned()],
                attach_stdout: Some(true),
                attach_stderr: Some(true),
                ..Default::default()
            },
            Duration::from_secs(5),
            4096,
            &cancellation,
        )
        .await
        .unwrap();
    assert_eq!(output.exit_code, 0);
    assert!(String::from_utf8_lossy(&output.stdout).contains("summary"));
    let bounded = client
        .exec_binary(
            ExecBinaryRequest {
                container_id: container_id.clone(),
                cmd: vec!["restic".to_owned(), "backup".to_owned()],
                attach_stdout: Some(true),
                attach_stderr: Some(true),
                ..Default::default()
            },
            Duration::from_secs(5),
            4,
            &cancellation,
        )
        .await
        .unwrap_err();
    assert_eq!(bounded.kind, RuntimeErrorKind::Remote);
    client
        .delete_container(&container_id, &cancellation)
        .await
        .unwrap();
    shutdown.cancel();
}

fn require_signature<T>(request: &Request<T>) -> Result<(), Status> {
    if request.metadata().get_bin("x-signature-bin").is_none()
        || request.metadata().get_bin("x-nonce-bin").is_none()
    {
        return Err(Status::unauthenticated("missing request signature"));
    }
    Ok(())
}

fn network_input() -> CreateRuntimeNetwork {
    CreateRuntimeNetwork {
        name: "frontend".into(),
        driver: "overlay".into(),
        scope: "swarm".into(),
        internal: None,
        attachable: Some(true),
        ingress: None,
        enable_ipv6: None,
        enable_ipv4: Some(true),
        config_only: None,
        ipam: None,
        config_from: None,
        labels: Default::default(),
        options: Default::default(),
    }
}

fn deployment_command() -> RuntimeDeploymentCommand {
    RuntimeDeploymentCommand {
        deployment_id: uuid::Uuid::now_v7(),
        name: "web".to_owned(),
        image_id: "sha256:image".to_owned(),
        spec: DeploymentSpec {
            image: DeploymentImageInfo::Local {
                image_id: "image".to_owned(),
            },
            update_behavior: UpdateBehavior::Disabled,
            life_cycle_spec: None,
            resource_spec: None,
            labels: Some(std::collections::BTreeMap::from([(
                "owner".to_owned(),
                "citadel".to_owned(),
            )])),
            ports: None,
            volumes: None,
            networks: None,
            command: None,
            environment_variables: None,
        },
        environment_variables: vec!["TOKEN=resolved".to_owned()],
    }
}

fn stack_claim() -> StackOperationClaim {
    let stack_id = uuid::Uuid::now_v7();
    StackOperationClaim {
        stack_id,
        release_id: uuid::Uuid::now_v7(),
        platform_id: uuid::Uuid::now_v7(),
        name: "Agent Stack".to_owned(),
        project_name: "agent-stack".to_owned(),
        platform_type: "Docker".to_owned(),
        spec: StackSpec::WebEditor {
            compose_file: "services:\n  web:\n    image: nginx\n".to_owned(),
            update_behavior: StackUpdateBehavior::Disabled,
            common: StackSpecCommon {
                destroy_before_deploy: true,
                pre_deploy: Some(citadel_stacks::StackCommand {
                    commands: vec!["echo preparing".to_owned()],
                    path: ".".to_owned(),
                }),
                post_deploy: Some(citadel_stacks::StackCommand {
                    commands: vec!["echo complete".to_owned()],
                    path: "scripts".to_owned(),
                }),
                ..Default::default()
            },
        },
        row_version: 1,
        actor_id: uuid::Uuid::now_v7(),
        operation: "Apply".to_owned(),
        service_names: vec!["web".into()],
    }
}

// Ports the runtime inspection assertion from RegularAgentCompatibilityTests
// across signed gRPC. The peer is a fixture, not a live Agent acceptance test.
#[tokio::test]
async fn signed_agent_inspection_preserves_image_state_and_redacts_secrets() {
    use citadel_platforms::containers::ContainerInspectionPort;
    let (address, shutdown) = start_fixture(MutationFixture {
        fail_network_create: false,
        network_create_calls: Arc::default(),
        container_delete_calls: Arc::default(),
        deployment_apply_calls: Arc::default(),
        container_action_calls: Arc::default(),
        stack_apply_calls: Arc::default(),
    })
    .await;
    let _guard = shutdown.drop_guard();
    let client = connect(&address).await;
    let cancel = CancellationToken::new();
    let result = client.inspection("container-1", &cancel).await.unwrap();
    assert_eq!(result["id"], "container-1");
    assert_eq!(result["state"]["status"], "Running");
    assert_eq!(result["config"]["image"], "busybox:latest");
    assert_eq!(
        result["config"]["env"],
        serde_json::json!(["CITADEL_VAULT_SECRET=********", "APP_MODE=production"])
    );
    assert!(!result.to_string().contains("agent-private"));
    assert_eq!(
        client
            .inspection("missing", &cancel)
            .await
            .unwrap_err()
            .kind,
        RuntimeErrorKind::NotFound
    );
    assert_eq!(
        client.inspection("denied", &cancel).await.unwrap_err().kind,
        RuntimeErrorKind::PermissionDenied
    );
    cancel.cancel();
    assert_eq!(
        client
            .inspection("container-1", &cancel)
            .await
            .unwrap_err()
            .kind,
        RuntimeErrorKind::Cancelled
    );
}

async fn connect(address: &str) -> AgentClient {
    AgentClient::connect(
        address,
        AgentRequestSigner::from_bytes(&[31_u8; 32]),
        Duration::from_secs(1),
        true,
    )
    .await
    .unwrap()
}

async fn start_fixture(fixture: MutationFixture) -> (String, CancellationToken) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    let cancellation = CancellationToken::new();
    let shutdown = cancellation.clone();
    tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(ContainerServiceServer::new(fixture.clone()))
            .add_service(DeploymentServiceServer::new(fixture.clone()))
            .add_service(StackServiceServer::new(fixture.clone()))
            .add_service(NetworkServiceServer::new(fixture.clone()))
            .add_service(VolumeServiceServer::new(fixture))
            .serve_with_shutdown(address, shutdown.cancelled_owned())
            .await
            .unwrap();
    });
    tokio::time::sleep(Duration::from_millis(20)).await;
    (format!("http://{address}"), cancellation)
}
