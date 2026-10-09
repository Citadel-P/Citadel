//! Real Core installer and node routing; no seeded database projections.
use super::*;

async fn nested(f: &Fixture, role: &str, args: &[&str]) -> String {
    let name = f.container(role);
    let mut command = vec!["exec", &name, "docker"];
    command.extend_from_slice(args);
    docker(&command).await
}

async fn stage_agent_image(f: &Fixture, image: &str) {
    // Image IDs can identify a manifest/index in the containerd store and a config in
    // the classic store. Registry digest references also do not survive load.
    // Save a unique tag so either store can resolve the transferred image.
    let source: Value = serde_json::from_str(&docker(&["image", "inspect", image]).await).unwrap();
    let image_id = source[0]["Id"].as_str().unwrap();
    let transfer_tag = format!("{}:transfer", f.name);
    let output = tokio::time::timeout(
        Duration::from_secs(120),
        tokio::process::Command::new("bash")
            .args([
                "-o",
                "pipefail",
                "-c",
                "set -e; docker image tag \"$1\" \"$3\"; \
                 trap 'docker image rm \"$3\" >/dev/null' EXIT; \
                 docker image save \"$3\" | docker exec -i \"$2\" docker image load",
                "acceptance",
                image_id,
                &f.container("swarm-manager"),
                &transfer_tag,
            ])
            .kill_on_drop(true)
            .output(),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(
        output.status.success(),
        "image transfer: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    nested(
        f,
        "swarm-manager",
        &[
            "tag",
            &transfer_tag,
            "registry:5000/citadel-agent:acceptance-native",
        ],
    )
    .await;
    let loaded = nested(
        f,
        "swarm-manager",
        &[
            "image",
            "inspect",
            "registry:5000/citadel-agent:acceptance-native",
        ],
    )
    .await;
    let loaded: Value = serde_json::from_str(&loaded).unwrap();
    for field in ["Architecture", "Os", "Variant", "RootFS"] {
        assert_eq!(
            loaded[0][field], source[0][field],
            "Transferred Agent image changed {field}"
        );
    }
    // Older Docker APIs include empty/default Config fields that newer APIs omit.
    let config = |image: &Value| {
        let mut config = image[0]["Config"].as_object().unwrap().clone();
        config.retain(|_, value| {
            !value.is_null()
                && value != false
                && value != ""
                && value.as_array().is_none_or(|items| !items.is_empty())
                && value.as_object().is_none_or(|items| !items.is_empty())
        });
        config
    };
    assert_eq!(
        config(&loaded),
        config(&source),
        "Transferred Agent image changed Config"
    );
}

async fn operation(f: &Fixture, platform: &str, action: &str) {
    let path = format!("/api/v1/platforms/{platform}/node-agents");
    let (method, path) = if action == "remove" {
        (Method::DELETE, path)
    } else {
        (Method::POST, format!("{path}/{action}"))
    };
    let progress = f.api(method, &path, None).await;
    let last = progress.as_array().unwrap().last().unwrap();
    assert_eq!(last["isCompleted"], true, "{progress}");
    assert!(last["errorMessage"].is_null(), "{progress}");
}

async fn covered(f: &Fixture, platform: &str) -> Value {
    f.wait(
        &format!("/api/v1/platforms/{platform}/node-agent-coverage"),
        |v| v["state"] == "Complete" && v["coveredNodes"] == 3,
    )
    .await
}

pub(super) async fn exercise(f: &mut Fixture, image: &str, public_key: &str, core_url: &str) {
    for role in ["swarm-manager", "swarm-worker-one", "swarm-worker-two"] {
        f.daemon(role).await;
        // Avoid Docker's default inner gateway overlapping the outer fixture network.
        nested(
            f,
            role,
            &[
                "network",
                "create",
                "--driver",
                "bridge",
                "--subnet",
                "192.168.248.0/24",
                "--gateway",
                "192.168.248.1",
                "--opt",
                "com.docker.network.bridge.name=docker_gwbridge",
                "docker_gwbridge",
            ],
        )
        .await;
    }
    nested(f, "swarm-manager", &["swarm", "init"]).await;
    let token = nested(f, "swarm-manager", &["swarm", "join-token", "-q", "worker"]).await;
    let mut nodes = vec![];
    for role in ["swarm-worker-one", "swarm-worker-two"] {
        nested(
            f,
            role,
            &["swarm", "join", "--token", &token, "swarm-manager:2377"],
        )
        .await;
        nodes.push(nested(f, role, &["info", "--format", "{{.Swarm.NodeID}}"]).await);
    }
    stage_agent_image(f, image).await;
    nested(
        f,
        "swarm-manager",
        &["push", "registry:5000/citadel-agent:acceptance-native"],
    )
    .await;
    // Docker's distribution endpoint omits platforms for a single OCI manifest.
    // Use the release index format, with only this candidate's actual platform.
    nested(
        f,
        "swarm-manager",
        &[
            "manifest",
            "create",
            "--insecure",
            "registry:5000/citadel-agent:acceptance",
            "registry:5000/citadel-agent:acceptance-native",
        ],
    )
    .await;
    nested(
        f,
        "swarm-manager",
        &[
            "manifest",
            "push",
            "--insecure",
            "registry:5000/citadel-agent:acceptance",
        ],
    )
    .await;

    for connector in ["Agent", "EdgeAgent"] {
        let role = if connector == "Agent" {
            "swarm-direct"
        } else {
            "swarm-edge"
        };
        let mut body =
            json!({"name":role,"type":"DockerSwarm","connectorType":connector,"tagIds":[]});
        let platform;
        if connector == "Agent" {
            f.run(
                role,
                &[
                    "-e",
                    &format!("HUB_PUBLIC_KEY={public_key}"),
                    "-e",
                    "DOCKER_HOST=tcp://swarm-manager:2375",
                    "--health-interval=1s",
                    image,
                ],
            )
            .await;
            f.agent_ready(role).await;
            body["address"] = json!(format!("http://{role}:9000"));
            platform = f.api(Method::POST, "/api/v1/platforms", Some(body)).await;
        } else {
            platform = f.api(Method::POST, "/api/v1/platforms", Some(body)).await;
            let id = platform["id"].as_str().unwrap();
            let enrollment = f
                .api(
                    Method::POST,
                    &format!("/api/v1/platforms/{id}/edge/enrollments"),
                    None,
                )
                .await;
            let data = f.volume("swarm-edge-data");
            f.run(
                role,
                &[
                    "-e",
                    "CITADEL_AGENT_MODE=edge",
                    "-e",
                    &format!("CITADEL_CORE_URL={core_url}"),
                    "-e",
                    &format!(
                        "CITADEL_EDGE_ENROLLMENT_TOKEN={}",
                        enrollment["token"].as_str().unwrap()
                    ),
                    "-e",
                    "DOCKER_HOST=tcp://swarm-manager:2375",
                    "-v",
                    &data,
                    image,
                ],
            )
            .await;
        }
        let id = platform["id"].as_str().unwrap();
        f.wait(&format!("/api/v1/platforms/{id}"), |v| {
            v["status"] == "Online"
        })
        .await;
        f.wait(
            &format!("/api/v1/platforms/{id}/node-agent-coverage"),
            |v| v["totalNodes"] == 3,
        )
        .await;
        operation(f, id, "install").await;
        let coverage = covered(f, id).await;
        println!("Swarm {connector}: installation and complete three-node coverage passed");
        assert_eq!(coverage["isInstalled"], true, "{coverage}");
        assert_eq!(coverage["missingNodes"], 0, "{coverage}");
        let service_name = format!("citadel-node-agent-{}", id.replace('-', ""));
        let spec: Value = serde_json::from_str(
            &nested(f, "swarm-manager", &["service", "inspect", &service_name]).await,
        )
        .unwrap();
        assert!(
            spec[0]["Spec"]["TaskTemplate"]["ContainerSpec"]["Image"]
                .as_str()
                .unwrap()
                .contains("@sha256:")
        );

        for (worker, node) in ["swarm-worker-one", "swarm-worker-two"].iter().zip(&nodes) {
            let container = nested(
                f,
                worker,
                &[
                    "run",
                    "-d",
                    "--name",
                    "acceptance-worker",
                    "alpine:3.24",
                    "sh",
                    "-c",
                    "echo worker-ready; exec sleep 1200",
                ],
            )
            .await;
            nested(f, worker, &["volume", "create", "acceptance-worker-volume"]).await;
            let network = nested(
                f,
                worker,
                &["network", "create", "acceptance-worker-network"],
            )
            .await;
            let inventory = f
                .wait(&format!("/api/v1/platforms/{id}/containers"), |v| {
                    v["containers"].as_array().is_some_and(|items| {
                        items.iter().any(|item| {
                            item["containerId"] == container
                                && item["dockerNodeId"] == *node
                                && item["lastStats"].is_object()
                        })
                    })
                })
                .await;
            let tracked = inventory["containers"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["containerId"] == container)
                .unwrap();
            let inspected = f
                .api(
                    Method::GET,
                    &format!(
                        "/api/v1/containers/{}/inspect",
                        tracked["id"].as_str().unwrap()
                    ),
                    None,
                )
                .await;
            assert_eq!(inspected["config"]["image"], "alpine:3.24", "{inspected}");
            for path in [
                format!("/api/v1/volumes/{id}/acceptance-worker-volume?dockerNodeId={node}"),
                format!("/api/v1/networks/{id}/{network}?dockerNodeId={node}"),
            ] {
                f.api(Method::GET, &path, None).await;
            }
            f.api(Method::GET, &format!("/api/v1/platforms/{id}/volumes/acceptance-worker-volume/files?dockerNodeId={node}"), None).await;
        }

        docker(&["stop", "-t", "12", &f.container(role)]).await;
        f.wait(&format!("/api/v1/platforms/{id}"), |v| {
            v["status"] == "Offline"
        })
        .await;
        docker(&["start", &f.container(role)]).await;
        f.wait(&format!("/api/v1/platforms/{id}"), |v| {
            v["status"] == "Online"
        })
        .await;
        covered(f, id).await;

        docker(&["stop", "-t", "30", &f.container("swarm-worker-two")]).await;
        f.wait(
            &format!("/api/v1/platforms/{id}/node-agent-coverage"),
            |v| v["state"] == "Partial" && v["coveredNodes"] == 2,
        )
        .await;
        let volume_path = format!("/api/v1/volumes/{id}");
        f.wait(&volume_path, |v| {
            v["volumes"].as_array().is_some_and(|items| {
                items
                    .iter()
                    .any(|item| item["dockerNodeId"] == nodes[1] && item["isStale"] == true)
            })
        })
        .await;
        let (status, problem) = f
            .request(
                Method::GET,
                &format!(
                    "/api/v1/volumes/{id}/acceptance-worker-volume?dockerNodeId={}",
                    nodes[1]
                ),
                None,
            )
            .await;
        assert_eq!(status, StatusCode::CONFLICT, "{problem}");
        docker(&["start", &f.container("swarm-worker-two")]).await;
        f.daemon_ready("swarm-worker-two").await;
        covered(f, id).await;
        f.wait(&volume_path, |v| {
            v["volumes"].as_array().is_some_and(|items| {
                items
                    .iter()
                    .any(|item| item["dockerNodeId"] == nodes[1] && item["isStale"] == false)
            })
        })
        .await;
        operation(f, id, "repair").await;
        covered(f, id).await;
        operation(f, id, "upgrade").await;
        covered(f, id).await;
        operation(f, id, "remove").await;
        f.wait(
            &format!("/api/v1/platforms/{id}/node-agent-coverage"),
            |v| v["isInstalled"] == false,
        )
        .await;
        assert!(
            nested(
                f,
                "swarm-manager",
                &[
                    "service",
                    "ls",
                    "-q",
                    "--filter",
                    &format!("name={service_name}")
                ]
            )
            .await
            .is_empty()
        );
        f.api(
            Method::DELETE,
            "/api/v1/platforms",
            Some(json!({"ids":[id]})),
        )
        .await;
        docker(&["stop", "-t", "12", &f.container(role)]).await;
        for worker in ["swarm-worker-one", "swarm-worker-two"] {
            nested(f, worker, &["rm", "-f", "acceptance-worker"]).await;
            nested(f, worker, &["volume", "rm", "acceptance-worker-volume"]).await;
            nested(f, worker, &["network", "rm", "acceptance-worker-network"]).await;
        }
        println!(
            "Swarm {connector}: Core install, pinned image, worker routing, volume browsing, manager/worker recovery, repair, upgrade and removal passed"
        );
    }
}

#[tokio::test]
#[ignore = "requires Docker; transfers a digest-pinned image into a disposable daemon"]
async fn stages_digest_pinned_image_without_registry_digest_metadata() {
    // A small multi-platform image exercises the same archive path as a released Agent.
    let image = "alpine@sha256:294b683cb724975bec92580e1e685676bd4b50bda910ddb8c51d4cabeaec77e6";
    docker(&["pull", image]).await;
    let mut f = Fixture::new();
    docker(&["network", "create", &f.name]).await;
    f.daemon("swarm-manager").await;
    // The daemon readiness setup pulls Alpine; remove it so registry metadata
    // cannot hide a broken save/load transfer.
    nested(&f, "swarm-manager", &["image", "rm", "alpine:3.24"]).await;
    assert!(
        nested(&f, "swarm-manager", &["image", "ls", "-q"])
            .await
            .is_empty()
    );
    stage_agent_image(&f, image).await;
    assert_eq!(
        nested(
            &f,
            "swarm-manager",
            &[
                "run",
                "--rm",
                "--pull=never",
                "--network=none",
                "registry:5000/citadel-agent:acceptance-native",
                "cat",
                "/etc/alpine-release",
            ],
        )
        .await,
        docker(&[
            "run",
            "--rm",
            "--pull=never",
            "--network=none",
            image,
            "cat",
            "/etc/alpine-release",
        ])
        .await,
        "Transferred image must run without fetching it again"
    );
    assert!(
        docker(&["image", "ls", "--quiet", &format!("{}:transfer", f.name)])
            .await
            .is_empty(),
        "Temporary transfer tag was not removed"
    );
}
