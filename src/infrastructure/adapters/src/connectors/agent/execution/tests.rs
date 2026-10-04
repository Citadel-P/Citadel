use super::*;
use crate::connectors::agent::client::AgentBuildCommand;
use crate::connectors::edge::EdgeRegistry;
use crate::connectors::edge::EdgeTarget;
use citadel_contracts::citadel::{
    containers::v1::{ExecExit, ExecOutput},
    edge::v1::core_envelope,
    images::v1::{BuildImageRequest, ImageBuildResponse, PushImageRequest},
};
use futures_util::StreamExt;
use uuid::Uuid;

#[tokio::test]
async fn edge_failures_preserve_types_for_unary_and_streaming_commands() {
    use citadel_platforms::RuntimeErrorKind::*;
    let registry = EdgeRegistry::default();
    let (session, mut receiver) = registry
        .register(EdgeTarget::platform(Uuid::now_v7()), Uuid::now_v7())
        .unwrap();
    for (code, kind, grpc) in [
        ("not_found", NotFound, tonic::Code::NotFound),
        (
            "permission_denied",
            PermissionDenied,
            tonic::Code::PermissionDenied,
        ),
        (
            "unauthenticated",
            Authentication,
            tonic::Code::Unauthenticated,
        ),
        (
            "invalid_argument",
            InvalidRequest,
            tonic::Code::InvalidArgument,
        ),
        (
            "failed_precondition",
            InvalidRequest,
            tonic::Code::FailedPrecondition,
        ),
        ("out_of_range", InvalidRequest, tonic::Code::OutOfRange),
        ("already_exists", Conflict, tonic::Code::AlreadyExists),
        ("aborted", Conflict, tonic::Code::Aborted),
        (
            "resource_exhausted",
            ResourceExhausted,
            tonic::Code::ResourceExhausted,
        ),
        ("cancelled", Cancelled, tonic::Code::Cancelled),
        ("deadline_exceeded", Timeout, tonic::Code::DeadlineExceeded),
        ("unavailable", Unavailable, tonic::Code::Unavailable),
        ("unimplemented", Remote, tonic::Code::Unimplemented),
        ("data_loss", Remote, tonic::Code::DataLoss),
        ("command_failed", Remote, tonic::Code::Unknown),
        ("unknown-code-with-secret", Remote, tonic::Code::Unknown),
    ] {
        let cancellation = CancellationToken::new();
        let (result, ()) = tokio::join!(
            unary::<_, ()>(&session, EdgeCommandKind::NetworkInspect, (), &cancellation),
            async {
                let command = receiver.recv().await.unwrap();
                let id = Uuid::parse_str(&command.command_id).unwrap();
                // A buffered response cannot turn a terminal failure into success.
                session.output(id, vec![]);
                session.fail(id, code);
                session.complete(id, true);
            }
        );
        let error = result.unwrap_err();
        assert_eq!(error.kind, kind, "{code}");
        assert!(!error.to_string().contains("secret"));
        let mut stream = stream::<_, ()>(
            &session,
            EdgeCommandKind::ContainerLogsStream,
            (),
            &cancellation,
        )
        .unwrap();
        let command = receiver.recv().await.unwrap();
        session.fail(Uuid::parse_str(&command.command_id).unwrap(), code);
        assert_eq!(
            stream.next().await.unwrap().unwrap_err().code(),
            grpc,
            "{code}"
        );
        assert!(stream.next().await.is_none());
        assert_eq!(session.pending_count(), 0);
    }
}

#[tokio::test]
async fn edge_deployment_uses_the_direct_agent_request_and_result_mapping() {
    use citadel_contracts::citadel::deployments::v1::{
        ApplyDeploymentRequest, ApplyDeploymentResponse, DeployedContainerState,
    };
    let registry = EdgeRegistry::default();
    let (session, mut receiver) = registry
        .register(EdgeTarget::platform(Uuid::now_v7()), Uuid::now_v7())
        .unwrap();
    let client = AgentExecutionClient::Edge(session.clone());
    let command = citadel_deployments::RuntimeDeploymentCommand {
        deployment_id: Uuid::now_v7(),
        name: "nginx".into(),
        image_id: "sha256:abc".into(),
        spec: serde_json::from_value(serde_json::json!({
            "image": {"$type":"External", "registryId":Uuid::nil(), "imageTag":"nginx"},
            "ports":["8080:80"], "networks":["bridge"], "labels":{"com.citadel.managed":"true"}
        }))
        .unwrap(),
        environment_variables: vec!["TOKEN=resolved-secret".into()],
    };
    let cancellation = CancellationToken::new();
    let agent = async {
        let envelope = receiver.recv().await.unwrap();
        let id = Uuid::parse_str(&envelope.command_id).unwrap();
        let Some(core_envelope::Body::Command(request)) = envelope.body else {
            panic!("expected command")
        };
        assert_eq!(request.kind, EdgeCommandKind::DeploymentApply as i32);
        let request = ApplyDeploymentRequest::decode(request.payload.as_slice()).unwrap();
        assert_eq!(
            request,
            crate::connectors::agent::client::workloads::deployment_request(&command)
        );
        assert_eq!(request.spec.unwrap().env_vars, ["TOKEN=resolved-secret"]);
        session.output(
            id,
            ApplyDeploymentResponse {
                container_id: "container".into(),
                deployed_container_state: DeployedContainerState::Running as i32,
            }
            .encode_to_vec(),
        );
        session.complete(id, true);
    };
    let (result, ()) = tokio::join!(client.apply_deployment(&command, &cancellation), agent);
    let result = result.unwrap();
    assert_eq!(result.docker_container_id, "container");
    assert_eq!(result.docker_image_id, "sha256:abc");
    assert_eq!(
        result.state,
        citadel_deployments::RuntimeContainerState::Running
    );
    assert_eq!(session.pending_count(), 0);
}

#[tokio::test]
async fn edge_pull_reads_error_frames_even_when_output_is_not_retained() {
    let registry = EdgeRegistry::default();
    let (session, mut receiver) = registry
        .register(EdgeTarget::platform(Uuid::now_v7()), Uuid::now_v7())
        .unwrap();
    let client = AgentExecutionClient::Edge(session.clone());
    let cancellation = CancellationToken::new();
    let (result, ()) = tokio::join!(
        client.pull_deployment_image("nginx", None, &cancellation),
        async {
            let envelope = receiver.recv().await.unwrap();
            let id = Uuid::parse_str(&envelope.command_id).unwrap();
            session.output(
                id,
                ImageBuildResponse {
                    error_message: Some("pull rejected".into()),
                    ..Default::default()
                }
                .encode_to_vec(),
            );
            session.complete(id, true);
        }
    );
    assert!(result.is_err());
    assert_eq!(session.pending_count(), 0);
}

// EdgeAgentConnectorTests parity: transport must retain the canonical request
// payload, completion semantics and cancellation of the regular Agent path.
#[tokio::test]
async fn edge_build_pushes_the_requested_tags_using_the_shared_contract() {
    let registry = EdgeRegistry::default();
    let (session, mut receiver) = registry
        .register(EdgeTarget::platform(Uuid::now_v7()), Uuid::now_v7())
        .unwrap();
    let client = AgentExecutionClient::Edge(session.clone());
    let cancellation = CancellationToken::new();
    let (progress, mut progress_receiver) = tokio::sync::mpsc::channel(1);
    let command = AgentBuildCommand {
        push_to_registry: true,
        output: Some(progress),
        context_archive: vec![1, 2, 3],
        dockerfile_path: "Dockerfile".into(),
        tags: vec!["registry.test/app:test".into()],
        build_args: Default::default(),
        target: None,
        registry_auth: Some("registry-credential".into()),
        registry_host: Some("registry.test".into()),
        timeout_seconds: 5,
        maximum_log_bytes: 1024,
        secrets: vec![("token".into(), "secret-value".into())],
    };
    let agent = async {
        let envelope = receiver.recv().await.unwrap();
        let id = Uuid::parse_str(&envelope.command_id).unwrap();
        let Some(core_envelope::Body::Command(command)) = envelope.body else {
            panic!("expected command")
        };
        assert_eq!(command.kind, EdgeCommandKind::ImageBuildStream as i32);
        let request = BuildImageRequest::decode(command.payload.as_slice()).unwrap();
        assert_eq!(request.context_archive, [1, 2, 3]);
        assert_eq!(request.build_secrets[0].value, "secret-value");
        assert_eq!(
            request.registry_auth.as_deref(),
            Some("registry-credential")
        );
        session.output(
            id,
            ImageBuildResponse {
                stream: Some("built".into()),
                ..Default::default()
            }
            .encode_to_vec(),
        );
        let live = progress_receiver.recv().await.unwrap();
        assert_eq!(
            live.bytes, b"built",
            "Output must arrive before Agent command completion"
        );
        session.complete(id, true);
        let envelope = receiver.recv().await.unwrap();
        let id = Uuid::parse_str(&envelope.command_id).unwrap();
        let Some(core_envelope::Body::Command(command)) = envelope.body else {
            panic!("expected command")
        };
        assert_eq!(command.kind, EdgeCommandKind::ImagePushStream as i32);
        let request = PushImageRequest::decode(command.payload.as_slice()).unwrap();
        assert_eq!(request.image_reference, "registry.test/app:test");
        assert_eq!(
            request.registry_auth.as_deref(),
            Some("registry-credential")
        );
        session.output(
            id,
            ImageBuildResponse {
                status: Some("pushed".into()),
                ..Default::default()
            }
            .encode_to_vec(),
        );
        assert_eq!(progress_receiver.recv().await.unwrap().bytes, b"pushed");
        session.complete(id, true);
    };
    let (result, ()) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(client.build_image(command, &cancellation), agent)
    })
    .await
    .unwrap();
    let output = result.unwrap();
    assert!(output.contains("built") && output.contains("pushed"));
    assert_eq!(session.pending_count(), 0);
}

#[tokio::test]
async fn edge_binary_execution_requires_an_exit_code_and_bounds_combined_output() {
    for (size, exit, succeeds) in [(4, true, true), (4, false, false), (9, true, false)] {
        let registry = EdgeRegistry::default();
        let (session, mut receiver) = registry
            .register(
                EdgeTarget::node(Uuid::now_v7(), "worker".into()),
                Uuid::now_v7(),
            )
            .unwrap();
        let client = AgentExecutionClient::Edge(session.clone());
        let cancellation = CancellationToken::new();
        let agent = async {
            let command = receiver.recv().await.unwrap();
            let id = Uuid::parse_str(&command.command_id).unwrap();
            session.output(
                id,
                ExecServerMessage {
                    msg: Some(exec_server_message::Msg::Output(ExecOutput {
                        data: vec![b'x'; size],
                        stream: 0,
                    })),
                }
                .encode_to_vec(),
            );
            if exit {
                session.output(
                    id,
                    ExecServerMessage {
                        msg: Some(exec_server_message::Msg::Exit(ExecExit { exit_code: 0 })),
                    }
                    .encode_to_vec(),
                );
            }
            session.complete(id, true);
        };
        let (result, ()) = tokio::join!(
            client.exec_binary(
                ExecBinaryRequest::default(),
                Duration::from_secs(2),
                8,
                &cancellation
            ),
            agent
        );
        assert_eq!(result.is_ok(), succeeds);
        assert_eq!(session.pending_count(), 0);
    }
}

#[tokio::test]
async fn edge_unary_requires_a_response_and_pre_cancel_does_not_send() {
    let registry = EdgeRegistry::default();
    let (session, mut receiver) = registry
        .register(EdgeTarget::platform(Uuid::now_v7()), Uuid::now_v7())
        .unwrap();
    let client = AgentExecutionClient::Edge(session.clone());
    let cancellation = CancellationToken::new();
    let (result, ()) = tokio::join!(
        client.create_container(CreateContainerRequest::default(), &cancellation),
        async {
            let command = receiver.recv().await.unwrap();
            session.complete(Uuid::parse_str(&command.command_id).unwrap(), true);
        }
    );
    assert!(result.is_err());
    cancellation.cancel();
    assert!(
        client
            .create_container(CreateContainerRequest::default(), &cancellation)
            .await
            .is_err()
    );
    assert!(receiver.try_recv().is_err());
    assert_eq!(session.pending_count(), 0);
}
