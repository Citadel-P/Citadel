//! Real Core installer and node routing; no seeded database projections.
use super::*;

async fn nested(f: &Fixture, role: &str, args: &[&str]) -> String {
    let name = f.container(role);
    let mut command = vec!["exec", &name, "docker"];
    command.extend_from_slice(args);
    docker(&command).await
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
    // Transfer only the candidate image into the disposable manager, then publish
    // it to the private fixture registry so the installer can resolve its digest.
    let output = tokio::time::timeout(
        Duration::from_secs(120),
        tokio::process::Command::new("bash")
            .args([
                "-o",
                "pipefail",
                "-c",
                "docker image save \"$1\" | docker exec -i \"$2\" docker image load",
                "acceptance",
                image,
                &f.container("swarm-manager"),
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
        &["tag", image, "registry:5000/citadel-agent:acceptance-native"],
    )
    .await;
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
