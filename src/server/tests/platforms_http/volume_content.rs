use super::*;
use citadel_adapters::connectors::edge::EdgeSession;
use citadel_adapters::connectors::edge::EdgeTarget;
use citadel_contracts::citadel::{
    containers::v1::{
        CreateContainerRequest, CreateContainerResponse, DeleteContainerRequest, ExecBinaryRequest,
        ExecExit, ExecOutput, ExecServerMessage, exec_server_message::Msg,
    },
    edge::v1::{EdgeCommandKind as Kind, core_envelope},
    shared_models::v1::{PlatformInfoResponse, VolumeResponse},
};
use prost::Message;

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn volume_content_endpoints_authorize_route_bound_and_audit_completed_downloads() {
    let f = fixture().await;
    let reader = super::lookup::subject(&f).await;
    let base = format!("/api/v1/platforms/{}/volumes/data/files", f.platform_id);
    let registry = &f.edge;
    let (session, mut commands) = registry
        .register(
            EdgeTarget::node(f.platform_id, "node-1".into()),
            Uuid::now_v7(),
        )
        .unwrap();
    let (_, mut wrong) = registry
        .register(
            EdgeTarget::node(f.platform_id, "other-node".into()),
            Uuid::now_v7(),
        )
        .unwrap();
    assert_eq!(
        send(&f, &base, None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(&f, &base, Some(reader.clone())).await.status(),
        StatusCode::FORBIDDEN
    );
    for suffix in ["?path=../etc", "?path=%2F..%2Fetc", "/download?path=%2F"] {
        assert_eq!(
            send(
                &f,
                &format!("{base}{suffix}"),
                Some(f.administrator.clone())
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert!(commands.try_recv().is_err());
    super::lookup::grant(
        &f,
        reader.actor_id.value(),
        citadel_primitives::ResourceType::Platform,
        f.platform_id,
        0,
    )
    .await;
    super::lookup::grant(
        &f,
        reader.actor_id.value(),
        citadel_primitives::ResourceType::Volume,
        f.platform_id,
        citadel_primitives::SpecificPermission::Browse as i32,
    )
    .await;
    assert_eq!(
        send(
            &f,
            &format!("{base}/download?path=%2Ffile&dockerNodeId=node-1"),
            Some(reader.clone())
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    for case in ["list", "fallback", "symlink", "download", "interrupted"] {
        let url = if matches!(case, "list" | "fallback" | "symlink") {
            format!("{base}?path=%2F&dockerNodeId=node-1")
        } else {
            format!("{base}/download?path=%2Fconfig.txt&dockerNodeId=node-1")
        };
        let calls = async {
            let response = send(
                &f,
                &url,
                Some(if case == "list" {
                    reader.clone()
                } else {
                    f.administrator.clone()
                }),
            )
            .await;
            if case == "symlink" {
                assert_eq!(response.status(), StatusCode::BAD_REQUEST);
            } else {
                assert_eq!(response.status(), StatusCode::OK);
                assert_eq!(response.headers()["cache-control"], "no-store");
                if matches!(case, "list" | "fallback") {
                    let body: Value = serde_json::from_slice(
                        &to_bytes(response.into_body(), 1024 * 1024).await.unwrap(),
                    )
                    .unwrap();
                    assert_eq!(body["platformId"], f.platform_id.to_string());
                    assert_eq!(body["entries"][0]["type"], "Directory");
                    assert_eq!(body["entries"][1]["type"], "File");
                } else {
                    assert!(
                        response.headers()["content-disposition"]
                            .to_str()
                            .unwrap()
                            .contains("config.txt")
                    );
                    let body = to_bytes(response.into_body(), 1024 * 1024).await;
                    if case == "interrupted" {
                        assert!(body.is_err());
                    } else {
                        assert_eq!(body.unwrap().as_ref(), b"actual file bytes\0\xff");
                    }
                }
            }
        };
        let peer = async {
            let count = if matches!(case, "download" | "interrupted") {
                7
            } else {
                6
            };
            for step in 0..count {
                let envelope = commands.recv().await.unwrap();
                let id = Uuid::parse_str(&envelope.command_id).unwrap();
                let Some(core_envelope::Body::Command(command)) = envelope.body else {
                    panic!("expected command");
                };
                let expected = match step {
                    0 => Kind::VolumeInspect,
                    1 => Kind::PlatformGetInfo,
                    2 => Kind::ContainerCreate,
                    3 => Kind::ContainerStart,
                    n if n == count - 1 => Kind::ContainerDelete,
                    _ => Kind::ContainerExecBinary,
                };
                assert_eq!(command.kind, expected as i32, "{case} step {step}");
                match expected {
                    Kind::PlatformGetInfo => {
                        session.output(
                            id,
                            PlatformInfoResponse {
                                agent_runtime_image: if case == "fallback" {
                                    String::new()
                                } else {
                                    "citadel-agent:installed-on-node".into()
                                },
                                ..Default::default()
                            }
                            .encode_to_vec(),
                        );
                    }
                    Kind::VolumeInspect => {
                        session.output(
                            id,
                            VolumeResponse {
                                name: "data".into(),
                                driver: "local".into(),
                                ..Default::default()
                            }
                            .encode_to_vec(),
                        );
                    }
                    Kind::ContainerCreate => {
                        let request =
                            CreateContainerRequest::decode(command.payload.as_slice()).unwrap();
                        assert_eq!(request.readonly_rootfs, Some(true));
                        assert_eq!(
                            request.image_id,
                            if case == "fallback" {
                                "citadel-agent:test"
                            } else {
                                "citadel-agent:installed-on-node"
                            }
                        );
                        assert_eq!(request.network_mode.as_deref(), Some("none"));
                        assert_eq!(request.labels["com.citadel.system"], "true");
                        assert_eq!(request.entry_point, ["/app/Citadel.Agent.VolumeHelper"]);
                        assert_eq!(request.mounts.len(), 1);
                        assert_eq!(request.mounts[0].source.as_deref(), Some("data"));
                        assert_eq!(request.mounts[0].read_only, Some(true));
                        session.output(
                            id,
                            CreateContainerResponse {
                                container_id: "volume-helper".into(),
                            }
                            .encode_to_vec(),
                        );
                    }
                    Kind::ContainerExecBinary => {
                        let request =
                            ExecBinaryRequest::decode(command.payload.as_slice()).unwrap();
                        assert_eq!(request.container_id, "volume-helper");
                        assert!(request.env.is_empty());
                        assert!(!request.tty);
                        if request.cmd[2] == "list" {
                            assert_eq!(request.cmd[9], "--max-payload-bytes");
                            let payload = if case == "symlink" {
                                json!({"Path":"/","Entries":[],"IsTruncated":false,"ErrorCode":"VolumePathIsSymlink"})
                            } else {
                                json!({"Path":"/","Entries":[{"Name":"z.txt","Path":"/z.txt","Type":"file","Size":5},{"Name":"folder","Path":"/folder","Type":"directory"}],"IsTruncated":false})
                            };
                            output(&session, id, serde_json::to_vec(&payload).unwrap());
                            exit(&session, id, 0);
                        } else if request.cmd[2] == "inspect" {
                            output(&session,id,br#"{"Exists":true,"Type":"file","Size":19,"SafeFileName":"config.txt"}"#.to_vec());
                            exit(&session, id, 0);
                        } else {
                            assert_eq!(request.cmd[2], "stream-file");
                            output(&session, id, b"actual file bytes\0\xff".to_vec());
                            if case != "interrupted" {
                                exit(&session, id, 0);
                            }
                        }
                    }
                    Kind::ContainerDelete => {
                        let request =
                            DeleteContainerRequest::decode(command.payload.as_slice()).unwrap();
                        assert_eq!(request.ids, ["volume-helper"]);
                        assert_eq!(request.force, Some(true));
                        assert_eq!(request.v, Some(false));
                    }
                    _ => {}
                }
                session.complete(id, true);
            }
        };
        tokio::time::timeout(StdDuration::from_secs(15), async {
            tokio::join!(calls, peer);
        })
        .await
        .unwrap();
        assert!(wrong.try_recv().is_err());
    }
    let records: Vec<Value> = sqlx::query_scalar("SELECT info::jsonb FROM activityevents WHERE platformid=$1 AND eventtype='VolumeContentDownloaded'").bind(f.platform_id).fetch_all(&f.pool).await.unwrap();
    assert_eq!(
        records.len(),
        1,
        "failed/interrupted reads must not create success activity"
    );
    assert_eq!(records[0]["$type"], "VolumeContentDownloaded");
    assert_eq!(records[0]["Path"], "/config.txt");
    registry.remove(&session);
    assert_eq!(
        send(
            &f,
            &format!("{base}?dockerNodeId=node-1"),
            Some(f.administrator.clone())
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    f.docker_server.abort();
    let _ = tokio::fs::remove_file(f.docker_socket).await;
}

fn output(session: &EdgeSession, id: Uuid, data: Vec<u8>) {
    session.output(
        id,
        ExecServerMessage {
            msg: Some(Msg::Output(ExecOutput { data, stream: 0 })),
        }
        .encode_to_vec(),
    );
}
fn exit(session: &EdgeSession, id: Uuid, code: i32) {
    session.output(
        id,
        ExecServerMessage {
            msg: Some(Msg::Exit(ExecExit { exit_code: code })),
        }
        .encode_to_vec(),
    );
}
