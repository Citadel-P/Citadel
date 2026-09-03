use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use citadel_adapters::agent::{AgentClient, AgentRequestSigner};
use citadel_contracts::citadel::networks::v1::network_service_server::{
    NetworkService, NetworkServiceServer,
};
use citadel_contracts::citadel::networks::v1::{
    CreateNetworkRequest, CreateNetworkResponse, DeleteNetworkRequest, DeleteNetworkResponse,
    InspectNetworkRequest, InspectNetworkResponse, ListNetworksRequest, ListNetworksResponse,
};
use citadel_contracts::citadel::shared_models::v1::VolumeResponse;
use citadel_contracts::citadel::volumes::v1::volume_service_server::{
    VolumeService, VolumeServiceServer,
};
use citadel_contracts::citadel::volumes::v1::{
    CreateVolumeRequest, InspectVolumeRequest, ListVolumesRequest, ListVolumesResponse,
    RemoveVolumeRequest, RemoveVolumeResponse,
};
use citadel_platforms::{
    CreateRuntimeNetwork, CreateRuntimeVolume, PlatformResourceMutationPort, RuntimeErrorKind,
};
use tokio_util::sync::CancellationToken;
use tonic::{Request, Response, Status};

#[derive(Clone)]
struct MutationFixture {
    fail_network_create: bool,
    network_create_calls: Arc<AtomicUsize>,
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
async fn agent_network_and_volume_mutations_are_signed_and_transport_equivalent() {
    let calls = Arc::new(AtomicUsize::new(0));
    let (address, shutdown) = start_fixture(MutationFixture {
        fail_network_create: false,
        network_create_calls: calls.clone(),
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
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    shutdown.cancel();
}

#[tokio::test]
async fn agent_mutations_do_not_retry_an_ambiguous_failure() {
    let calls = Arc::new(AtomicUsize::new(0));
    let (address, shutdown) = start_fixture(MutationFixture {
        fail_network_create: true,
        network_create_calls: calls.clone(),
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
            .add_service(NetworkServiceServer::new(fixture.clone()))
            .add_service(VolumeServiceServer::new(fixture))
            .serve_with_shutdown(address, shutdown.cancelled_owned())
            .await
            .unwrap();
    });
    tokio::time::sleep(Duration::from_millis(20)).await;
    (format!("http://{address}"), cancellation)
}
