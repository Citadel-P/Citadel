use citadel_adapters::{
    agent::{AgentClient, AgentRequestSigner},
    edge::{EdgeRegistry, EdgeRuntime, EdgeTarget},
    swarm_inventory::SwarmInventoryClient,
};
use citadel_contracts::citadel::swarm::v1::swarm_service_server::{
    SwarmService, SwarmServiceServer,
};
use citadel_contracts::citadel::{
    edge::v1::{EdgeCommandKind, core_envelope},
    swarm::v1::*,
};
use citadel_platforms::{PlatformInventoryPort, RuntimeErrorKind, swarm_mutations::*};
use prost::Message;
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use tonic::{Request, Response, Status};
use uuid::Uuid;

async fn exercise(client: &SwarmInventoryClient<'_>) {
    let cancel = CancellationToken::new();
    let (node, running, desired) = client.inspect_node("node", &cancel).await.unwrap();
    assert_eq!(node.id, "node");
    assert_eq!((running, desired), (2, 3));
    let service = client.inspect_service("service", &cancel).await.unwrap();
    assert_eq!(service.id, "service");
    assert!(service.definition.is_some());
    client
        .update_node(
            "node",
            &UpdateSwarmNodeInput {
                version_index: 3,
                availability: "Drain".into(),
                labels: BTreeMap::from([("zone".into(), "a".into())]),
            },
            &cancel,
        )
        .await
        .unwrap();
    client.restart_service("service", &cancel).await.unwrap();
    client.delete_service("service", &cancel).await.unwrap();
    for secret in [true, false] {
        client
            .create_material(
                secret,
                &CreateSwarmMaterialInput {
                    name: "material".into(),
                    data: "héllo".into(),
                    labels: BTreeMap::new(),
                },
                &cancel,
            )
            .await
            .unwrap();
        client
            .update_labels(
                secret,
                "material",
                &UpdateSwarmResourceLabelsInput {
                    version_index: 4,
                    labels: BTreeMap::from([("team".into(), "ops".into())]),
                },
                &cancel,
            )
            .await
            .unwrap();
        client
            .delete_material(secret, "material", &cancel)
            .await
            .unwrap();
    }
    assert_eq!(
        client.config_data("material", &cancel).await.unwrap(),
        "content"
    );
}
fn node() -> SwarmNodeMessage {
    SwarmNodeMessage {
        id: "node".into(),
        running_task_count: 2,
        desired_task_count: 3,
        ..Default::default()
    }
}
fn service() -> SwarmServiceMessage {
    SwarmServiceMessage {
        id: "service".into(),
        definition: Some(SwarmServiceMutationSpecMessage::default()),
        ..Default::default()
    }
}

#[tokio::test]
async fn edge_inventory_uses_canonical_requests_and_exact_manager_session() {
    let registry = EdgeRegistry::default();
    let pid = Uuid::now_v7();
    let (session, mut commands) = registry
        .register(EdgeTarget::platform(pid), Uuid::now_v7())
        .unwrap();
    let (_other, mut other) = registry
        .register(EdgeTarget::node(pid, "worker".into()), Uuid::now_v7())
        .unwrap();
    let runtime = EdgeRuntime {
        session: session.clone(),
    };
    let responder = tokio::spawn(async move {
        for _ in 0..13 {
            let envelope = tokio::time::timeout(Duration::from_secs(5), commands.recv())
                .await
                .unwrap()
                .unwrap();
            let Some(core_envelope::Body::Command(command)) = envelope.body else {
                panic!("expected unary command")
            };
            let kind = EdgeCommandKind::try_from(command.kind).unwrap();
            let data = match kind {
                EdgeCommandKind::SwarmTaskList => {
                    let input = ListSwarmTasksRequest::decode(command.payload.as_slice()).unwrap();
                    assert_eq!(
                        input.max_items,
                        i32::MAX,
                        "Agent otherwise caps the response at 500"
                    );
                    ListSwarmTasksResponse {
                        tasks: vec![SwarmTaskMessage::default(); 501],
                    }
                    .encode_to_vec()
                }
                EdgeCommandKind::SwarmNodeInspect => {
                    assert_eq!(
                        InspectSwarmNodeRequest::decode(command.payload.as_slice())
                            .unwrap()
                            .node_id,
                        "node"
                    );
                    node().encode_to_vec()
                }
                EdgeCommandKind::SwarmServiceInspect => service().encode_to_vec(),
                EdgeCommandKind::SwarmNodeUpdate => {
                    let input = UpdateSwarmNodeRequest::decode(command.payload.as_slice()).unwrap();
                    assert_eq!(input.version_index, 3);
                    assert_eq!(input.availability, "Drain");
                    assert_eq!(input.labels["zone"], "a");
                    vec![]
                }
                EdgeCommandKind::SwarmServiceRestart => {
                    assert_eq!(
                        RestartSwarmServiceRequest::decode(command.payload.as_slice())
                            .unwrap()
                            .service_id,
                        "service"
                    );
                    vec![]
                }
                EdgeCommandKind::SwarmServiceDelete => vec![],
                EdgeCommandKind::SwarmSecretCreate => {
                    let input =
                        CreateSwarmSecretRequest::decode(command.payload.as_slice()).unwrap();
                    assert_eq!(input.data, "héllo".as_bytes());
                    SwarmResourceCreateResponse {
                        resource_id: "material".into(),
                    }
                    .encode_to_vec()
                }
                EdgeCommandKind::SwarmConfigCreate => {
                    let input =
                        CreateSwarmConfigRequest::decode(command.payload.as_slice()).unwrap();
                    assert_eq!(input.data, "héllo".as_bytes());
                    SwarmResourceCreateResponse {
                        resource_id: "material".into(),
                    }
                    .encode_to_vec()
                }
                EdgeCommandKind::SwarmConfigUpdate | EdgeCommandKind::SwarmSecretUpdate => {
                    let input =
                        UpdateSwarmResourceLabelsRequest::decode(command.payload.as_slice())
                            .unwrap();
                    assert_eq!(input.version_index, 4);
                    assert_eq!(input.labels["team"], "ops");
                    vec![]
                }
                EdgeCommandKind::SwarmConfigDelete | EdgeCommandKind::SwarmSecretDelete => vec![],
                EdgeCommandKind::SwarmConfigData => SwarmConfigDataResponse {
                    data: b"content".to_vec(),
                }
                .encode_to_vec(),
                unexpected => panic!("unexpected {unexpected:?}"),
            };
            let id = Uuid::parse_str(&envelope.command_id).unwrap();
            session.output(id, data);
            session.complete(id, true);
        }
    });
    exercise(&SwarmInventoryClient::Edge(&runtime)).await;
    assert_eq!(
        runtime
            .list_swarm_tasks(&CancellationToken::new())
            .await
            .unwrap()
            .len(),
        501
    );
    responder.await.unwrap();
    assert!(other.try_recv().is_err());
}

#[derive(Clone)]
struct Fixture {
    calls: Arc<AtomicUsize>,
    reject: bool,
}
impl Fixture {
    fn checked<T>(&self, request: Request<T>) -> Result<T, Status> {
        assert!(request.metadata().get_bin("x-signature-bin").is_some());
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.reject {
            Err(Status::unavailable("ambiguous outcome"))
        } else {
            Ok(request.into_inner())
        }
    }
}
#[tonic::async_trait]
impl SwarmService for Fixture {
    async fn inspect_node(
        &self,
        r: Request<InspectSwarmNodeRequest>,
    ) -> Result<Response<SwarmNodeMessage>, Status> {
        assert_eq!(self.checked(r)?.node_id, "node");
        Ok(Response::new(node()))
    }
    async fn inspect_service(
        &self,
        r: Request<InspectSwarmServiceRequest>,
    ) -> Result<Response<SwarmServiceMessage>, Status> {
        assert_eq!(self.checked(r)?.service_id, "service");
        Ok(Response::new(service()))
    }
    async fn update_node(
        &self,
        r: Request<UpdateSwarmNodeRequest>,
    ) -> Result<Response<()>, Status> {
        let r = self.checked(r)?;
        assert_eq!(r.version_index, 3);
        assert_eq!(r.labels["zone"], "a");
        Ok(Response::new(()))
    }
    async fn restart_service(
        &self,
        r: Request<RestartSwarmServiceRequest>,
    ) -> Result<Response<()>, Status> {
        self.checked(r)?;
        Ok(Response::new(()))
    }
    async fn delete_service(
        &self,
        r: Request<DeleteManagedSwarmServiceRequest>,
    ) -> Result<Response<()>, Status> {
        self.checked(r)?;
        Ok(Response::new(()))
    }
    async fn create_secret(
        &self,
        r: Request<CreateSwarmSecretRequest>,
    ) -> Result<Response<SwarmResourceCreateResponse>, Status> {
        assert_eq!(self.checked(r)?.data, "héllo".as_bytes());
        Ok(Response::new(SwarmResourceCreateResponse {
            resource_id: "material".into(),
        }))
    }
    async fn create_config(
        &self,
        r: Request<CreateSwarmConfigRequest>,
    ) -> Result<Response<SwarmResourceCreateResponse>, Status> {
        assert_eq!(self.checked(r)?.data, "héllo".as_bytes());
        Ok(Response::new(SwarmResourceCreateResponse {
            resource_id: "material".into(),
        }))
    }
    async fn update_secret_labels(
        &self,
        r: Request<UpdateSwarmResourceLabelsRequest>,
    ) -> Result<Response<()>, Status> {
        assert_eq!(self.checked(r)?.version_index, 4);
        Ok(Response::new(()))
    }
    async fn update_config_labels(
        &self,
        r: Request<UpdateSwarmResourceLabelsRequest>,
    ) -> Result<Response<()>, Status> {
        assert_eq!(self.checked(r)?.version_index, 4);
        Ok(Response::new(()))
    }
    async fn delete_secret(
        &self,
        r: Request<DeleteSwarmSecretRequest>,
    ) -> Result<Response<()>, Status> {
        self.checked(r)?;
        Ok(Response::new(()))
    }
    async fn delete_config(
        &self,
        r: Request<DeleteSwarmConfigRequest>,
    ) -> Result<Response<()>, Status> {
        self.checked(r)?;
        Ok(Response::new(()))
    }
    async fn get_config_data(
        &self,
        r: Request<InspectSwarmConfigRequest>,
    ) -> Result<Response<SwarmConfigDataResponse>, Status> {
        self.checked(r)?;
        Ok(Response::new(SwarmConfigDataResponse {
            data: b"content".to_vec(),
        }))
    }
    async fn list_nodes(
        &self,
        _: Request<ListSwarmNodesRequest>,
    ) -> Result<Response<ListSwarmNodesResponse>, Status> {
        Err(Status::unimplemented("not exercised"))
    }
    async fn list_services(
        &self,
        _: Request<ListSwarmServicesRequest>,
    ) -> Result<Response<ListSwarmServicesResponse>, Status> {
        Err(Status::unimplemented("not exercised"))
    }
    async fn create_service(
        &self,
        _: Request<CreateManagedSwarmServiceRequest>,
    ) -> Result<Response<SwarmServiceMutationResponse>, Status> {
        Err(Status::unimplemented("not exercised"))
    }
    async fn update_service(
        &self,
        _: Request<UpdateManagedSwarmServiceRequest>,
    ) -> Result<Response<SwarmServiceMutationResponse>, Status> {
        Err(Status::unimplemented("not exercised"))
    }
    async fn get_service_logs(
        &self,
        _: Request<SwarmLogsRequest>,
    ) -> Result<Response<SwarmLogsResponse>, Status> {
        Err(Status::unimplemented("not exercised"))
    }
    async fn list_tasks(
        &self,
        request: Request<ListSwarmTasksRequest>,
    ) -> Result<Response<ListSwarmTasksResponse>, Status> {
        let input = self.checked(request)?;
        assert_eq!(
            input.max_items,
            i32::MAX,
            "Agent otherwise caps the response at 500"
        );
        Ok(Response::new(ListSwarmTasksResponse {
            tasks: vec![SwarmTaskMessage::default(); 501],
        }))
    }
    async fn inspect_task(
        &self,
        _: Request<InspectSwarmTaskRequest>,
    ) -> Result<Response<SwarmTaskMessage>, Status> {
        Err(Status::unimplemented("not exercised"))
    }
    async fn get_task_logs(
        &self,
        _: Request<SwarmLogsRequest>,
    ) -> Result<Response<SwarmLogsResponse>, Status> {
        Err(Status::unimplemented("not exercised"))
    }
    async fn list_networks(
        &self,
        _: Request<ListSwarmNetworksRequest>,
    ) -> Result<Response<ListSwarmNetworksResponse>, Status> {
        Err(Status::unimplemented("not exercised"))
    }
    async fn inspect_network(
        &self,
        _: Request<InspectSwarmNetworkRequest>,
    ) -> Result<Response<SwarmNetworkMessage>, Status> {
        Err(Status::unimplemented("not exercised"))
    }
    async fn list_secrets(
        &self,
        _: Request<ListSwarmSecretsRequest>,
    ) -> Result<Response<ListSwarmSecretsResponse>, Status> {
        Err(Status::unimplemented("not exercised"))
    }
    async fn inspect_secret(
        &self,
        _: Request<InspectSwarmSecretRequest>,
    ) -> Result<Response<SwarmSecretMessage>, Status> {
        Err(Status::unimplemented("not exercised"))
    }
    async fn list_configs(
        &self,
        _: Request<ListSwarmConfigsRequest>,
    ) -> Result<Response<ListSwarmConfigsResponse>, Status> {
        Err(Status::unimplemented("not exercised"))
    }
    async fn inspect_config(
        &self,
        _: Request<InspectSwarmConfigRequest>,
    ) -> Result<Response<SwarmConfigMessage>, Status> {
        Err(Status::unimplemented("not exercised"))
    }
    async fn create_system_service(
        &self,
        _: Request<CreateSystemSwarmServiceRequest>,
    ) -> Result<Response<SwarmServiceMutationResponse>, Status> {
        Err(Status::unimplemented("not exercised"))
    }
    async fn update_system_service(
        &self,
        _: Request<UpdateSystemSwarmServiceRequest>,
    ) -> Result<Response<SwarmServiceMutationResponse>, Status> {
        Err(Status::unimplemented("not exercised"))
    }
}
#[tokio::test]
async fn direct_agent_inventory_preserves_wire_contract_and_does_not_retry_writes() {
    for reject in [false, true] {
        let calls = Arc::new(AtomicUsize::new(0));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let cancel = CancellationToken::new();
        let stop = cancel.clone();
        let count = calls.clone();
        let server = tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_service(SwarmServiceServer::new(Fixture {
                    calls: count,
                    reject,
                }))
                .serve_with_incoming_shutdown(
                    async_stream::stream! {
                        loop {
                            yield listener.accept().await.map(|(stream, _)| stream);
                        }
                    },
                    stop.cancelled(),
                )
                .await
                .unwrap();
        });
        let agent = AgentClient::connect(
            &format!("http://{addr}"),
            AgentRequestSigner::from_bytes(&[5; 32]),
            Duration::from_secs(2),
            true,
        )
        .await
        .unwrap();
        let client = SwarmInventoryClient::Agent(&agent);
        if reject {
            let error = client
                .restart_service("service", &CancellationToken::new())
                .await
                .unwrap_err();
            assert_eq!(error.kind, RuntimeErrorKind::Unavailable);
            assert_eq!(calls.load(Ordering::SeqCst), 1);
        } else {
            exercise(&client).await;
            assert_eq!(
                agent
                    .list_swarm_tasks(&CancellationToken::new())
                    .await
                    .unwrap()
                    .len(),
                501
            );
            assert_eq!(calls.load(Ordering::SeqCst), 13);
        }
        cancel.cancel();
        server.await.unwrap();
    }
}
