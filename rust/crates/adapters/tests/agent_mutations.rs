use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use citadel_adapters::agent::{AgentClient, AgentRequestSigner};
use citadel_contracts::citadel::containers::v1::container_service_server::{
    ContainerService, ContainerServiceServer,
};
use citadel_contracts::citadel::containers::v1::{
    ContainerIds, ContainerLogRequest, ContainerLogResponse, ContainersStatsResponse,
    CreateContainerRequest, CreateContainerResponse, DeleteContainerRequest, ExecBinaryRequest,
    ExecClientMessage, ExecServerMessage, InspectContainerRequest, ListContainersRequest,
    ListContainersResponse, StreamContainerStatsRequest, StreamContainersStatsRequest,
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
use citadel_platforms::{
    CreateRuntimeNetwork, CreateRuntimeVolume, PlatformResourceMutationPort, RuntimeErrorKind,
};
use futures_util::Stream;
use tokio_util::sync::CancellationToken;
use tonic::{Request, Response, Status};

#[derive(Clone)]
struct MutationFixture {
    fail_network_create: bool,
    network_create_calls: Arc<AtomicUsize>,
    container_delete_calls: Arc<AtomicUsize>,
    deployment_apply_calls: Arc<AtomicUsize>,
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

    async fn start(&self, _: Request<ContainerIds>) -> Result<Response<()>, Status> {
        Err(Status::unimplemented("not used"))
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
        assert_eq!(request.ids, ["container-1"]);
        assert_eq!(request.v, Some(true));
        assert_eq!(request.force, Some(true));
        assert_eq!(request.link, Some(false));
        self.container_delete_calls.fetch_add(1, Ordering::Relaxed);
        Ok(Response::new(()))
    }

    async fn inspect(
        &self,
        _: Request<InspectContainerRequest>,
    ) -> Result<Response<InspectContainerResponse>, Status> {
        Err(Status::unimplemented("not used"))
    }

    async fn create(
        &self,
        _: Request<CreateContainerRequest>,
    ) -> Result<Response<CreateContainerResponse>, Status> {
        Err(Status::unimplemented("not used"))
    }

    type ExecStream = TestStream<ExecServerMessage>;

    async fn exec(
        &self,
        _: Request<tonic::Streaming<ExecClientMessage>>,
    ) -> Result<Response<Self::ExecStream>, Status> {
        Err(Status::unimplemented("not used"))
    }

    type ExecBinaryStream = TestStream<ExecServerMessage>;

    async fn exec_binary(
        &self,
        _: Request<ExecBinaryRequest>,
    ) -> Result<Response<Self::ExecBinaryStream>, Status> {
        Err(Status::unimplemented("not used"))
    }

    type StreamContainerLogsStream = TestStream<ContainerLogResponse>;

    async fn stream_container_logs(
        &self,
        _: Request<ContainerLogRequest>,
    ) -> Result<Response<Self::StreamContainerLogsStream>, Status> {
        Err(Status::unimplemented("not used"))
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
    let (address, shutdown) = start_fixture(MutationFixture {
        fail_network_create: false,
        network_create_calls: calls.clone(),
        container_delete_calls: container_calls.clone(),
        deployment_apply_calls: deployment_calls.clone(),
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
    let applied = client
        .apply_deployment(&deployment_command(), &cancellation)
        .await
        .unwrap();
    assert_eq!(applied.docker_container_id, "container-applied");
    assert_eq!(applied.state, RuntimeContainerState::Running);
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    assert_eq!(container_calls.load(Ordering::Relaxed), 1);
    assert_eq!(deployment_calls.load(Ordering::Relaxed), 1);
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
            .add_service(NetworkServiceServer::new(fixture.clone()))
            .add_service(VolumeServiceServer::new(fixture))
            .serve_with_shutdown(address, shutdown.cancelled_owned())
            .await
            .unwrap();
    });
    tokio::time::sleep(Duration::from_millis(20)).await;
    (format!("http://{address}"), cancellation)
}
