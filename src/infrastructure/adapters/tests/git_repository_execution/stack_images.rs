//! Edge test peer providing complete immutable image metadata without a live registry.
use citadel_adapters::connectors::edge::{EdgeRegistry, EdgeTarget};
use citadel_contracts::citadel::{
    containers::v1::ListContainersResponse,
    edge::v1::{EdgeCommandKind as Kind, core_envelope},
    images::v1::*,
    shared_models::v1::*,
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
    platform: Uuid,
    stack: Uuid,
) -> (EdgeRegistry, tokio::task::JoinHandle<usize>) {
    sqlx::query("UPDATE platforms SET connectortype='EdgeAgent' WHERE id=$1")
        .bind(platform)
        .execute(pool)
        .await
        .unwrap();
    let edge = EdgeRegistry::default();
    let (session, mut outbound) = edge
        .register(EdgeTarget::platform(platform), Uuid::now_v7())
        .unwrap();
    let peer = tokio::spawn(async move {
        let mut calls = 0;
        while calls < 5 {
            let envelope =
                tokio::time::timeout(std::time::Duration::from_secs(10), outbound.recv())
                    .await
                    .unwrap()
                    .unwrap();
            let command_id = Uuid::parse_str(&envelope.command_id).unwrap();
            let Some(core_envelope::Body::Command(command)) = envelope.body else {
                continue;
            };
            let payload = match command.kind() {
                Kind::ContainerList => ListContainersResponse {
                    containers: [(
                        "web".into(),
                        ContainerMessage {
                            id: "web".into(),
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
                    )]
                    .into(),
                }
                .encode_to_vec(),
                Kind::ImageList => ListImageResponse {
                    images: vec![ImageReply {
                        id: "image-id".into(),
                        repo_tags: vec!["nginx:alpine".into()],
                        repo_digests: vec![format!("nginx@{}", deployed_digest())],
                        ..Default::default()
                    }],
                }
                .encode_to_vec(),
                Kind::ImageInspect => {
                    let input = InspectImageRequest::decode(command.payload.as_slice()).unwrap();
                    assert_eq!(input.id, "image-id");
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
                    let digest = input.image_name.strip_prefix("nginx@").unwrap();
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
        calls
    });
    (edge, peer)
}
