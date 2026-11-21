use super::*;
use crate::connectors::agent::{
    client::{AgentClient, AgentRequestSigner},
    execution::AgentExecutionClient,
};
use crate::connectors::edge::{EdgeRegistry, EdgeTarget};
use citadel_contracts::citadel::{
    edge::v1::{EdgeCommandKind, core_envelope},
    shared_models::v1::VolumeResponse,
    volumes::v1::*,
};
use citadel_platforms::RuntimeErrorKind;
use prost::Message;
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use tonic::{Request, Response, Status};
use volume_service_server::{VolumeService, VolumeServiceServer};

#[derive(Default, Clone)]
struct VolumeDriver {
    inspections: Arc<AtomicUsize>,
    creates: Arc<AtomicUsize>,
    lists: Arc<AtomicUsize>,
}

#[tonic::async_trait]
impl VolumeService for VolumeDriver {
    async fn inspect(
        &self,
        request: Request<InspectVolumeRequest>,
    ) -> Result<Response<VolumeResponse>, Status> {
        let request = request.into_inner();
        assert_eq!(
            request.observation,
            inspect_volume_request::Observation::Metadata as i32
        );
        self.inspections.fetch_add(1, Ordering::Relaxed);
        match request.name.as_str() {
            "data" => Ok(Response::new(VolumeResponse {
                name: request.name,
                ..Default::default()
            })),
            "missing" => Err(Status::not_found("volume missing")),
            "private" => Err(Status::permission_denied("access denied")),
            _ => Err(Status::unavailable("driver unavailable")),
        }
    }
    async fn list(
        &self,
        _: Request<ListVolumesRequest>,
    ) -> Result<Response<ListVolumesResponse>, Status> {
        self.lists.fetch_add(1, Ordering::Relaxed);
        Ok(Response::new(ListVolumesResponse::default()))
    }
    async fn create(
        &self,
        _: Request<CreateVolumeRequest>,
    ) -> Result<Response<VolumeResponse>, Status> {
        self.creates.fetch_add(1, Ordering::Relaxed);
        Err(Status::failed_precondition("unexpected create"))
    }
    async fn remove(
        &self,
        _: Request<RemoveVolumeRequest>,
    ) -> Result<Response<RemoveVolumeResponse>, Status> {
        Err(Status::unimplemented("unexpected remove"))
    }
}

#[tokio::test]
async fn direct_volume_existence_is_one_metadata_read_and_restore_preserves_unknown_targets() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let incoming = futures_util::stream::unfold(listener, |listener| async move {
        Some((listener.accept().await.map(|(socket, _)| socket), listener))
    });
    let driver = VolumeDriver::default();
    let shutdown = CancellationToken::new();
    let server = tokio::spawn(
        tonic::transport::Server::builder()
            .add_service(VolumeServiceServer::new(driver.clone()))
            .serve_with_incoming_shutdown(incoming, shutdown.clone().cancelled_owned()),
    );
    let client = AgentClient::connect(
        &format!("http://{address}"),
        AgentRequestSigner::from_bytes(&[31; 32]),
        Duration::from_secs(2),
        true,
    )
    .await
    .unwrap();
    let client = AgentExecutionClient::Direct(Arc::new(client));
    let token = CancellationToken::new();
    assert!(client.volume_exists("data", &token).await.unwrap());
    assert!(!client.volume_exists("missing", &token).await.unwrap());
    assert_eq!(
        client
            .volume_exists("private", &token)
            .await
            .unwrap_err()
            .kind,
        RuntimeErrorKind::PermissionDenied
    );
    assert_eq!(
        client
            .volume_exists("offline", &token)
            .await
            .unwrap_err()
            .kind,
        RuntimeErrorKind::Unavailable
    );
    let owner = RestoreVolumeOwner::new(Uuid::now_v7(), Uuid::now_v7());
    assert!(
        owner
            .prepare_agent_target(&client, "offline", false, &token)
            .await
            .is_err()
    );
    token.cancel();
    assert_eq!(
        client.volume_exists("data", &token).await.unwrap_err().kind,
        RuntimeErrorKind::Cancelled
    );
    assert_eq!(driver.inspections.load(Ordering::Relaxed), 5);
    assert_eq!(driver.lists.load(Ordering::Relaxed), 0);
    assert_eq!(driver.creates.load(Ordering::Relaxed), 0);
    shutdown.cancel();
    server.await.unwrap().unwrap();
}

#[tokio::test]
async fn edge_volume_existence_uses_the_same_metadata_and_absence_contract() {
    let registry = EdgeRegistry::default();
    let (session, mut receiver) = registry
        .register(EdgeTarget::platform(Uuid::now_v7()), Uuid::now_v7())
        .unwrap();
    let client = AgentExecutionClient::Edge(session.clone());
    let cancellation = CancellationToken::new();
    for failure in [None, Some("not_found"), Some("unavailable")] {
        let (result, ()) = tokio::join!(client.volume_exists("data", &cancellation), async {
            let envelope = receiver.recv().await.unwrap();
            let id = Uuid::parse_str(&envelope.command_id).unwrap();
            let Some(core_envelope::Body::Command(command)) = envelope.body else {
                panic!("expected inspection")
            };
            assert_eq!(command.kind, EdgeCommandKind::VolumeInspect as i32);
            let request = InspectVolumeRequest::decode(command.payload.as_slice()).unwrap();
            assert_eq!(request.name, "data");
            assert_eq!(
                request.observation,
                inspect_volume_request::Observation::Metadata as i32
            );
            if let Some(code) = failure {
                session.fail(id, code);
            } else {
                session.output(
                    id,
                    VolumeResponse {
                        name: "data".into(),
                        ..Default::default()
                    }
                    .encode_to_vec(),
                );
                session.complete(id, true);
            }
        });
        match failure {
            None => assert!(result.unwrap()),
            Some("not_found") => assert!(!result.unwrap()),
            _ => assert_eq!(result.unwrap_err().kind, RuntimeErrorKind::Unavailable),
        }
        assert!(receiver.try_recv().is_err());
    }
}
