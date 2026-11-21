//! Edge test peer providing complete immutable image metadata without a live registry.
use citadel_adapters::connectors::edge::{EdgeRegistry, EdgeTarget};
use citadel_contracts::citadel::{
    containers::v1::ListContainersResponse,
    edge::v1::{EdgeCommandKind as Kind, core_envelope},
    images::v1::*,
    shared_models::v1::*,
    swarm::v1::*,
};
use prost::Message;
use uuid::Uuid;

pub fn deployed_digest() -> String {
    format!("sha256:{}", "a".repeat(64))
}
pub fn remote_digest() -> String {
    format!("sha256:{}", "b".repeat(64))
}

pub async fn start(
    pool: &sqlx::PgPool,
    edge: &EdgeRegistry,
    platform: Uuid,
    stacks: Vec<Uuid>,
    repository: &'static str,
) -> (
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<(usize, usize, usize)>,
) {
    sqlx::query("UPDATE platforms SET connectortype='EdgeAgent' WHERE id=$1")
        .bind(platform)
        .execute(pool)
        .await
        .unwrap();
    let (session, mut outbound) = edge
        .register(EdgeTarget::platform(platform), Uuid::now_v7())
        .unwrap();
    let (done, mut shutdown) = tokio::sync::oneshot::channel();
    let peer = tokio::spawn(async move {
        let mut calls = 0;
        let mut catalogs = (0, 0);
        loop {
            let envelope = tokio::select! { _ = &mut shutdown => break, envelope = tokio::time::timeout(std::time::Duration::from_secs(10), outbound.recv()) => envelope.unwrap().unwrap() };
            let command_id = Uuid::parse_str(&envelope.command_id).unwrap();
            let Some(core_envelope::Body::Command(command)) = envelope.body else {
                continue;
            };
            let payload = match command.kind() {
                Kind::SwarmNodeList => {
                    let request =
                        ListSwarmNodesRequest::decode(command.payload.as_slice()).unwrap();
                    assert!(!request.include_task_counts);
                    ListSwarmNodesResponse {
                        nodes: vec![SwarmNodeMessage {
                            id: "node-1".into(),
                            architecture: "amd64".into(),
                            operating_system: "linux".into(),
                            ..Default::default()
                        }],
                    }
                    .encode_to_vec()
                }
                Kind::SwarmServiceList => ListSwarmServicesResponse {
                    services: stacks
                        .iter()
                        .map(|stack| SwarmServiceMessage {
                            id: stack.to_string(),
                            name: format!("{stack}_web"),
                            image: format!("{repository}@{}", deployed_digest()),
                            labels: [
                                ("com.docker.stack.namespace".into(), stack.to_string()),
                                ("com.citadel.stack-id".into(), stack.to_string()),
                            ]
                            .into(),
                            ..Default::default()
                        })
                        .collect(),
                }
                .encode_to_vec(),
                Kind::SwarmTaskList if !stacks.is_empty() => ListSwarmTasksResponse {
                    tasks: stacks
                        .iter()
                        .map(|stack| SwarmTaskMessage {
                            id: format!("task-{stack}"),
                            service_id: stack.to_string(),
                            node_id: "node-1".into(),
                            desired_state: "running".into(),
                            ..Default::default()
                        })
                        .collect(),
                }
                .encode_to_vec(),
                Kind::SwarmTaskList => ListSwarmTasksResponse {
                    tasks: vec![SwarmTaskMessage {
                        id: "task-1".into(),
                        service_id: "service-1".into(),
                        node_id: "node-1".into(),
                        desired_state: "running".into(),
                        ..Default::default()
                    }],
                }
                .encode_to_vec(),
                Kind::ContainerList => {
                    catalogs.0 += 1;
                    ListContainersResponse {
                        containers: stacks
                            .iter()
                            .map(|stack| {
                                (
                                    stack.to_string(),
                                    ContainerMessage {
                                        id: stack.to_string(),
                                        name: "web".into(),
                                        image: "nginx:alpine".into(),
                                        image_id: "image-id".into(),
                                        state: ContainerStateType::Running as i32,
                                        labels: [
                                            ("com.citadel.stack-id".into(), stack.to_string()),
                                            ("com.docker.compose.service".into(), "web".into()),
                                        ]
                                        .into(),
                                        ..Default::default()
                                    },
                                )
                            })
                            .collect(),
                    }
                    .encode_to_vec()
                }
                Kind::ImageList => {
                    catalogs.1 += 1;
                    assert!(
                        ListImagesRequest::decode(command.payload.as_slice())
                            .unwrap()
                            .identity_only
                    );
                    ListImageResponse {
                        images: vec![ImageReply {
                            id: "image-id".into(),
                            repo_tags: vec!["nginx:alpine".into()],
                            repo_digests: vec![format!("nginx@{}", deployed_digest())],
                            ..Default::default()
                        }],
                    }
                    .encode_to_vec()
                }
                Kind::ImageInspect => {
                    let input = InspectImageRequest::decode(command.payload.as_slice()).unwrap();
                    assert_eq!(input.id, "image-id");
                    assert!(input.identity_only);
                    InspectImageResponse {
                        id: "image-id".into(),
                        os: Some("linux".into()),
                        architecture: Some("amd64".into()),
                        ..Default::default()
                    }
                    .encode_to_vec()
                }
                Kind::ImageDistributionInspect => {
                    let input =
                        DistributionInspectRequest::decode(command.payload.as_slice()).unwrap();
                    let target = input.target_platform.unwrap();
                    assert_eq!(
                        (target.os.as_str(), target.architecture.as_str()),
                        ("linux", "amd64")
                    );
                    let (name, digest) = input.image_name.split_once('@').unwrap();
                    assert_eq!(name, repository);
                    assert!(digest == deployed_digest() || digest == remote_digest());
                    let config_digest = format!(
                        "sha256:{}",
                        if digest == deployed_digest() {
                            "c"
                        } else {
                            "d"
                        }
                        .repeat(64)
                    );
                    DistributionInspectResponse {
                        descriptor: Some(OciDescriptorResponse {
                            digest: digest.into(),
                            ..Default::default()
                        }),
                        config_digest,
                        manifest_digest: digest.into(),
                        ..Default::default()
                    }
                    .encode_to_vec()
                }
                kind => panic!("unexpected image-check command: {kind:?}"),
            };
            session.output(command_id, payload);
            session.complete(command_id, true);
            calls += 1;
        }
        (calls, catalogs.0, catalogs.1)
    });
    (done, peer)
}
